// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::{DateTime, Utc};
use regex::Regex;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::contract_lane_utils as lane_utils;

fn usage() {
    println!("stock-market-collector-kernel commands:");
    println!("  protheus-ops stock-market-collector-kernel run --payload-base64=<json>");
    println!("  protheus-ops stock-market-collector-kernel prepare-run --payload-base64=<json>");
    println!(
        "  protheus-ops stock-market-collector-kernel build-fetch-plan --payload-base64=<json>"
    );
    println!("  protheus-ops stock-market-collector-kernel finalize-run --payload-base64=<json>");
    println!("  protheus-ops stock-market-collector-kernel collect --payload-base64=<json>");
    println!("  protheus-ops stock-market-collector-kernel extract-quotes --payload-base64=<json>");
    println!("  protheus-ops stock-market-collector-kernel map-quotes --payload-base64=<json>");
    println!(
        "  protheus-ops stock-market-collector-kernel fallback-indices --payload-base64=<json>"
    );
}

const COLLECTOR_ID: &str = "stock_market";
const EYES_STATE_DEFAULT_REL: &str = "local/state/sensory/eyes";

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    lane_utils::clean_text(raw, max_len)
}

fn clamp_u64(payload: &Map<String, Value>, key: &str, fallback: u64, lo: u64, hi: u64) -> u64 {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn as_f64(value: Option<&Value>, fallback: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(fallback)
}

fn resolve_eyes_state_dir(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Some(raw) = payload.get("eyes_state_dir").and_then(Value::as_str) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }
    if let Ok(raw) = std::env::var("EYES_STATE_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    root.join(EYES_STATE_DEFAULT_REL)
}

fn meta_path_for(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    resolve_eyes_state_dir(root, payload)
        .join("collector_meta")
        .join(format!("{COLLECTOR_ID}.json"))
}

fn cache_path_for(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    resolve_eyes_state_dir(root, payload)
        .join("collector_meta")
        .join(format!("{COLLECTOR_ID}.cache.json"))
}

fn read_json(path: &Path, fallback: Value) -> Value {
    lane_utils::read_json(path).unwrap_or(fallback)
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("stock_market_collector_kernel_create_dir_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let body = format!(
        "{}\n",
        serde_json::to_string_pretty(value)
            .map_err(|err| format!("stock_market_collector_kernel_encode_failed:{err}"))?
    );
    fs::write(&tmp, body)
        .map_err(|err| format!("stock_market_collector_kernel_write_failed:{err}"))?;
    fs::rename(&tmp, path)
        .map_err(|err| format!("stock_market_collector_kernel_rename_failed:{err}"))
}

fn clean_seen_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if out.len() >= 120 {
            break;
        }
        let keep = ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.');
        if keep {
            out.push(ch);
        }
    }
    out
}

fn normalize_meta_value(raw: Option<&Value>) -> Value {
    let obj = raw.and_then(Value::as_object);
    let last_run = clean_text(
        obj.and_then(|o| o.get("last_run")).and_then(Value::as_str),
        80,
    );
    let last_success = clean_text(
        obj.and_then(|o| o.get("last_success"))
            .and_then(Value::as_str),
        80,
    );
    let mut seen_ids = Vec::new();
    if let Some(items) = obj
        .and_then(|o| o.get("seen_ids"))
        .and_then(Value::as_array)
    {
        for entry in items {
            if let Some(raw_id) = entry.as_str() {
                let cleaned = clean_seen_id(raw_id);
                if !cleaned.is_empty() {
                    seen_ids.push(Value::String(cleaned));
                }
            }
        }
    }
    if seen_ids.len() > 2000 {
        let split = seen_ids.len() - 2000;
        seen_ids = seen_ids.into_iter().skip(split).collect::<Vec<_>>();
    }
    json!({
        "collector_id": COLLECTOR_ID,
        "last_run": if last_run.is_empty() { Value::Null } else { Value::String(last_run) },
        "last_success": if last_success.is_empty() { Value::Null } else { Value::String(last_success) },
        "seen_ids": seen_ids
    })
}

fn parse_iso_ms(raw: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn classify_curl_transport_error(stderr: &str) -> String {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("could not resolve host")
        || lower.contains("name or service not known")
        || lower.contains("temporary failure in name resolution")
    {
        return "dns_unreachable".to_string();
    }
    if lower.contains("connection refused") {
        return "connection_refused".to_string();
    }
    if lower.contains("operation timed out")
        || lower.contains("connection timed out")
        || lower.contains("timed out")
    {
        return "timeout".to_string();
    }
    if lower.contains("ssl") || lower.contains("tls") || lower.contains("certificate") {
        return "tls_error".to_string();
    }
    "collector_error".to_string()
}

fn http_status_to_code(status: u64) -> &'static str {
    match status {
        401 => "auth_unauthorized",
        403 => "auth_forbidden",
        404 => "http_404",
        408 => "timeout",
        429 => "rate_limited",
        500..=u64::MAX => "http_5xx",
        400..=499 => "http_4xx",
        _ => "http_error",
    }
}

fn curl_fetch_with_status(
    url: &str,
    timeout_ms: u64,
    accept: &str,
) -> Result<(u64, String, u64), String> {
    let timeout_secs = ((timeout_ms.max(1_000) as f64) / 1_000.0).ceil() as u64;
    let output = Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg(timeout_secs.to_string())
        .arg("-H")
        .arg("User-Agent: Infring-Eyes/1.0")
        .arg("-H")
        .arg(format!("Accept: {accept}"))
        .arg("-H")
        .arg("Accept-Language: en-US,en;q=0.9")
        .arg("-w")
        .arg("\n__PROTHEUS_STATUS__:%{http_code}\n")
        .arg(url)
        .output()
        .map_err(|err| format!("collector_fetch_spawn_failed:{err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let code = classify_curl_transport_error(&stderr);
        return Err(format!("{code}:{}", clean_text(Some(&stderr), 220)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let marker = "\n__PROTHEUS_STATUS__:";
    let marker_pos = stdout
        .rfind(marker)
        .ok_or_else(|| "collector_fetch_missing_status_marker".to_string())?;
    let body = stdout[..marker_pos].to_string();
    let status_raw = stdout[(marker_pos + marker.len())..]
        .lines()
        .next()
        .unwrap_or("0")
        .trim()
        .to_string();
    let status = status_raw.parse::<u64>().unwrap_or(0);
    let bytes = body.as_bytes().len() as u64;
    Ok((status, body, bytes))
}

fn load_cache_items(root: &Path, payload: &Map<String, Value>) -> Vec<Value> {
    read_json(&cache_path_for(root, payload), json!({ "items": [] }))
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn today_utc() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn sha16(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex::encode(digest)[..16].to_string()
}

#[derive(Clone, Debug)]
struct Quote {
    symbol: String,
    short_name: String,
    price: f64,
    change: f64,
    change_percent: f64,
    volume: i64,
}

fn quote_from_object(obj: &Map<String, Value>) -> Option<Quote> {
    let symbol = clean_text(obj.get("symbol").and_then(Value::as_str), 32).to_uppercase();
    let price = obj
        .get("regularMarketPrice")
        .and_then(Value::as_f64)
        .or_else(|| obj.get("price").and_then(Value::as_f64))
        .unwrap_or(0.0);
    if symbol.is_empty() || !(price.is_finite() && price > 0.0) {
        return None;
    }
    let short_name = clean_text(
        obj.get("shortName")
            .and_then(Value::as_str)
            .or_else(|| obj.get("longName").and_then(Value::as_str))
            .or_else(|| obj.get("name").and_then(Value::as_str))
            .or_else(|| obj.get("symbol").and_then(Value::as_str)),
        160,
    );
    let change = obj
        .get("regularMarketChange")
        .and_then(Value::as_f64)
        .or_else(|| obj.get("change").and_then(Value::as_f64))
        .unwrap_or(0.0);
    let change_percent = obj
        .get("regularMarketChangePercent")
        .and_then(Value::as_f64)
        .or_else(|| obj.get("changePercent").and_then(Value::as_f64))
        .or_else(|| obj.get("change_percent").and_then(Value::as_f64))
        .unwrap_or(0.0);
    let volume = obj
        .get("regularMarketVolume")
        .and_then(Value::as_i64)
        .or_else(|| obj.get("volume").and_then(Value::as_i64))
        .unwrap_or(0);
    Some(Quote {
        symbol,
        short_name: if short_name.is_empty() {
            "Unknown".to_string()
        } else {
            short_name
        },
        price,
        change,
        change_percent,
        volume,
    })
}

fn walk_quotes(value: &Value, out: &mut BTreeMap<String, Quote>, depth: usize) {
    if depth > 16 {
        return;
    }
    match value {
        Value::Object(obj) => {
            if let Some(quote) = quote_from_object(obj) {
                out.entry(quote.symbol.clone()).or_insert(quote);
            }
            for child in obj.values() {
                walk_quotes(child, out, depth + 1);
            }
        }
        Value::Array(rows) => {
            for row in rows {
                walk_quotes(row, out, depth + 1);
            }
        }
        _ => {}
    }
}

fn extract_quotes_from_html(html: &str) -> Vec<Quote> {
    let patterns = [
        r#"(?s)root\.App\.main\s*=\s*(\{.*?\});"#,
        r#"(?s)window\._initialState\s*=\s*(\{.*?\});"#,
        r#"(?s)"marketSummaryAndSparkResponse":(\{.*?\}),"#,
    ];

    let mut quotes = BTreeMap::<String, Quote>::new();
    for pat in patterns {
        let re = match Regex::new(pat) {
            Ok(v) => v,
            Err(_) => continue,
        };
        for caps in re.captures_iter(html) {
            let raw = caps.get(1).map(|m| m.as_str()).unwrap_or("").trim();
            if raw.is_empty() {
                continue;
            }
            if let Ok(parsed) = serde_json::from_str::<Value>(raw) {
                walk_quotes(&parsed, &mut quotes, 0);
            }
        }
    }
    quotes.into_values().collect::<Vec<_>>()
}

fn quote_to_value(q: &Quote) -> Value {
    json!({
        "symbol": q.symbol,
        "shortName": q.short_name,
        "price": q.price,
        "change": q.change,
        "changePercent": q.change_percent,
        "volume": q.volume
    })
}

fn date_seed(payload: &Map<String, Value>) -> String {
    let raw = clean_text(payload.get("date").and_then(Value::as_str), 32);
    if raw.is_empty() {
        today_utc()
    } else {
        raw
    }
}

fn normalize_seen_ids(payload: &Map<String, Value>) -> Vec<String> {
    payload
        .get("seen_ids")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| clean_text(Some(v), 120))
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn format_signed_2(value: f64) -> String {
    if value >= 0.0 {
        format!("+{value:.2}")
    } else {
        format!("{value:.2}")
    }
}

fn map_quotes(payload: &Map<String, Value>) -> Value {
