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
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
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
    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
    let source = clean_text(payload.get("source").and_then(Value::as_str), 64);
    let source = if source.is_empty() {
        "stock_market".to_string()
    } else {
        source
    };
    let date = date_seed(payload);
    let collected_at = now_iso();
    let mut seen = normalize_seen_ids(payload)
        .into_iter()
        .collect::<HashSet<_>>();

    let rows = payload
        .get("quotes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut items = Vec::<Value>::new();
    for row in rows {
        if items.len() >= max_items {
            break;
        }
        let quote = row.as_object().and_then(quote_from_object).or_else(|| {
            // Accept canonicalized quote shape from extract-quotes.
            row.as_object().and_then(|obj| {
                let symbol =
                    clean_text(obj.get("symbol").and_then(Value::as_str), 32).to_uppercase();
                let price = obj.get("price").and_then(Value::as_f64).unwrap_or(0.0);
                if symbol.is_empty() || !(price.is_finite() && price > 0.0) {
                    return None;
                }
                Some(Quote {
                    symbol,
                    short_name: clean_text(obj.get("shortName").and_then(Value::as_str), 160),
                    price,
                    change: obj.get("change").and_then(Value::as_f64).unwrap_or(0.0),
                    change_percent: obj
                        .get("changePercent")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                    volume: obj.get("volume").and_then(Value::as_i64).unwrap_or(0),
                })
            })
        });
        let q = match quote {
            Some(v) => v,
            None => continue,
        };

        let id = sha16(&format!("stock-{}-{}-{:.4}", q.symbol, date, q.price));
        if seen.contains(&id) {
            continue;
        }
        seen.insert(id.clone());

        let is_index = q.symbol.starts_with('^');
        let signal = q.change_percent.abs() > 2.0 || q.volume > 10_000_000;
        let signal_type = if is_index { "index" } else { "equity" };
        let movement_tag = if q.change > 0.0 {
            "gainer"
        } else if q.change < 0.0 {
            "loser"
        } else {
            "unchanged"
        };

        items.push(json!({
            "id": id,
            "collected_at": collected_at,
            "url": format!("https://finance.yahoo.com/quote/{}", q.symbol),
            "title": format!(
                "{}: ${:.2} ({}, {}%)",
                if q.short_name.is_empty() { q.symbol.clone() } else { q.short_name.clone() },
                q.price,
                format_signed_2(q.change),
                format_signed_2(q.change_percent)
            ),
            "description": format!("Volume: {}. Market data for {}.", q.volume, q.symbol),
            "symbol": q.symbol,
            "price": q.price,
            "change": q.change,
            "change_percent": q.change_percent,
            "volume": q.volume,
            "signal_type": signal_type,
            "signal": signal,
            "source": source,
            "tags": ["finance", "market", movement_tag],
            "topics": ["finance", "market"],
            "bytes": 0
        }));
    }

    let mut seen_ids = seen.into_iter().collect::<Vec<_>>();
    seen_ids.sort();
    if seen_ids.len() > 2000 {
        let drop = seen_ids.len() - 2000;
        seen_ids.drain(0..drop);
    }

    json!({
        "ok": true,
        "items": items,
        "seen_ids": seen_ids
    })
}

fn fallback_indices(payload: &Map<String, Value>) -> Value {
    let indices = [
        ("^GSPC", "S&P 500", "index"),
        ("^IXIC", "NASDAQ Composite", "index"),
        ("^DJI", "Dow Jones Industrial Average", "index"),
        ("^RUT", "Russell 2000", "index"),
        ("^VIX", "CBOE Volatility Index", "volatility"),
    ];
    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
    let date = date_seed(payload);
    let collected_at = now_iso();
    let mut seen = normalize_seen_ids(payload)
        .into_iter()
        .collect::<HashSet<_>>();

    let mut items = Vec::<Value>::new();
    for (symbol, name, signal_type) in indices {
        if items.len() >= max_items {
            break;
        }
        let id = sha16(&format!("stock-{symbol}-{date}"));
        if seen.contains(&id) {
            continue;
        }
        seen.insert(id.clone());
        items.push(json!({
            "id": id,
            "collected_at": collected_at,
            "url": format!("https://finance.yahoo.com/quote/{symbol}"),
            "title": format!("{name} - Market Index"),
            "description": "Major market index tracking. Monitor for significant moves.",
            "symbol": symbol,
            "signal_type": signal_type,
            "signal": true,
            "source": "stock_market",
            "tags": ["finance", "index", "market", "fallback"],
            "topics": ["finance", "market"],
            "bytes": 0
        }));
    }

    let mut seen_ids = seen.into_iter().collect::<Vec<_>>();
    seen_ids.sort();
    if seen_ids.len() > 2000 {
        let drop = seen_ids.len() - 2000;
        seen_ids.drain(0..drop);
    }

    json!({
        "ok": true,
        "items": items,
        "seen_ids": seen_ids
    })
}

fn command_prepare_run(root: &Path, payload: &Map<String, Value>) -> Value {
    let force = as_bool(payload.get("force"), false);
    let min_hours = as_f64(payload.get("min_hours"), 1.0).clamp(0.0, 24.0 * 365.0);
    let meta_path = meta_path_for(root, payload);
    let meta = normalize_meta_value(Some(&read_json(&meta_path, normalize_meta_value(None))));
    let last_run_ms = meta
        .get("last_run")
        .and_then(Value::as_str)
        .and_then(parse_iso_ms);
    let hours_since_last =
        last_run_ms.map(|ms| ((Utc::now().timestamp_millis() - ms) as f64 / 3_600_000.0).max(0.0));
    let skipped = !force && hours_since_last.map(|h| h < min_hours).unwrap_or(false);
    json!({
        "ok": true,
        "collector_id": COLLECTOR_ID,
        "force": force,
        "min_hours": min_hours,
        "hours_since_last": hours_since_last,
        "skipped": skipped,
        "reason": if skipped { Value::String("cadence".to_string()) } else { Value::Null },
        "meta": meta,
        "meta_path": meta_path.display().to_string()
    })
}

fn command_build_fetch_plan(_payload: &Map<String, Value>) -> Value {
    json!({
        "ok": true,
        "collector_id": COLLECTOR_ID,
        "requests": [
            {
                "key": "market_html",
                "url": "https://finance.yahoo.com/markets/",
                "required": true,
                "accept": "application/json,text/html,*/*"
            }
        ]
    })
}

fn finalize_success(
    root: &Path,
    payload: &Map<String, Value>,
    min_hours: f64,
    max_items: usize,
    bytes: u64,
    requests: u64,
    duration_ms: u64,
) -> Result<Value, String> {
    let mut meta = normalize_meta_value(payload.get("meta"));
    let today = date_seed(payload);
    let initial_seen = meta
        .get("seen_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let quotes = payload
        .get("quotes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut degraded = false;
    let mapped = if !quotes.is_empty() {
        map_quotes(
            &json!({
                "date": today,
                "max_items": max_items,
                "seen_ids": initial_seen,
                "quotes": quotes
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        )
    } else {
        degraded = true;
        fallback_indices(
            &json!({
                "date": today,
                "max_items": max_items,
                "seen_ids": initial_seen
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        )
    };

    let items = mapped
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let seen_ids = mapped
        .get("seen_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    meta["seen_ids"] = Value::Array(seen_ids);
    meta["last_run"] = Value::String(now_iso());
    if !items.is_empty() {
        meta["last_success"] = Value::String(now_iso());
        write_json_atomic(&cache_path_for(root, payload), &json!({ "items": items }))?;
    }
    write_json_atomic(&meta_path_for(root, payload), &meta)?;

    let sample = items
        .first()
        .and_then(Value::as_object)
        .and_then(|o| o.get("symbol"))
        .and_then(Value::as_str)
        .map(|s| clean_text(Some(s), 64))
        .filter(|s| !s.is_empty())
        .map(Value::String)
        .unwrap_or(Value::Null);

    Ok(json!({
        "ok": true,
        "success": true,
        "eye": COLLECTOR_ID,
        "items": items,
        "bytes": bytes,
        "duration_ms": duration_ms,
        "requests": requests.max(1),
        "cadence_hours": min_hours,
        "degraded": degraded,
        "sample": sample
    }))
}

fn finalize_error(
    root: &Path,
    payload: &Map<String, Value>,
    min_hours: f64,
    max_items: usize,
    bytes: u64,
    requests: u64,
    duration_ms: u64,
    error: &str,
) -> Result<Value, String> {
    let mut meta = normalize_meta_value(payload.get("meta"));
    let cached = load_cache_items(root, payload);
    if !cached.is_empty() {
        return Ok(json!({
            "ok": true,
            "success": true,
            "eye": COLLECTOR_ID,
            "cache_hit": true,
            "degraded": true,
            "error": clean_text(Some(error), 120),
            "items": cached.into_iter().take(max_items).collect::<Vec<_>>(),
            "bytes": bytes,
            "requests": requests,
            "duration_ms": duration_ms,
            "cadence_hours": min_hours
        }));
    }

    let initial_seen = meta
        .get("seen_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let fallback = fallback_indices(
        &json!({
            "date": date_seed(payload),
            "max_items": max_items,
            "seen_ids": initial_seen
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    );
    let fallback_items = fallback
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    meta["last_run"] = Value::String(now_iso());
    if let Some(seen_ids) = fallback.get("seen_ids").and_then(Value::as_array) {
        meta["seen_ids"] = Value::Array(seen_ids.clone());
    }
    write_json_atomic(&meta_path_for(root, payload), &meta)?;

    let sample = fallback_items
        .first()
        .and_then(Value::as_object)
        .and_then(|o| o.get("symbol"))
        .and_then(Value::as_str)
        .map(|s| clean_text(Some(s), 64))
        .filter(|s| !s.is_empty())
        .map(Value::String)
        .unwrap_or(Value::Null);

    Ok(json!({
        "ok": true,
        "success": true,
        "eye": COLLECTOR_ID,
        "items": fallback_items,
        "bytes": bytes,
        "duration_ms": duration_ms,
        "requests": requests.max(1),
        "cadence_hours": min_hours,
        "degraded": true,
        "error": clean_text(Some(error), 120),
        "sample": sample
    }))
}

fn command_finalize_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let min_hours = as_f64(payload.get("min_hours"), 1.0).clamp(0.0, 24.0 * 365.0);
    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
    let bytes = clamp_u64(payload, "bytes", 0, 0, u64::MAX);
    let requests = clamp_u64(payload, "requests", 0, 0, u64::MAX);
    let duration_ms = clamp_u64(payload, "duration_ms", 0, 0, u64::MAX);
    let fetch_error = clean_text(payload.get("fetch_error").and_then(Value::as_str), 220);
    if !fetch_error.is_empty() {
        return finalize_error(
            root,
            payload,
            min_hours,
            max_items,
            bytes,
            requests,
            duration_ms,
            &fetch_error,
        );
    }
    finalize_success(
        root,
        payload,
        min_hours,
        max_items,
        bytes,
        requests,
        duration_ms,
    )
}

fn command_collect(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let min_hours = as_f64(payload.get("min_hours"), 1.0).clamp(0.0, 24.0 * 365.0);
    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
    let force = as_bool(payload.get("force"), false);
    let prepared = command_prepare_run(
        root,
        &json!({
            "eyes_state_dir": payload.get("eyes_state_dir").cloned().unwrap_or(Value::Null),
            "force": force,
            "min_hours": min_hours
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    );

    if prepared.get("skipped").and_then(Value::as_bool) == Some(true) {
        return Ok(json!({
            "ok": true,
            "success": true,
            "eye": COLLECTOR_ID,
            "skipped": true,
            "reason": "cadence",
            "hours_since_last": prepared.get("hours_since_last").cloned().unwrap_or(Value::Null),
            "min_hours": min_hours,
            "items": []
        }));
    }

    let market_html = payload
        .get("market_html")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let quotes = if market_html.trim().is_empty() {
        Vec::<Value>::new()
    } else {
        extract_quotes_from_html(market_html.as_str())
            .iter()
            .map(quote_to_value)
            .collect::<Vec<_>>()
    };
    let bytes = clamp_u64(payload, "bytes", 0, 0, u64::MAX);
    let requests = clamp_u64(payload, "requests", 0, 0, u64::MAX);
    let duration_ms = clamp_u64(payload, "duration_ms", 0, 0, u64::MAX);
    let mut fetch_error = clean_text(payload.get("fetch_error").and_then(Value::as_str), 220);
    if fetch_error.is_empty() && market_html.trim().is_empty() {
        fetch_error = "collector_error_no_market_html".to_string();
    }

    command_finalize_run(
        root,
        &json!({
            "eyes_state_dir": payload.get("eyes_state_dir").cloned().unwrap_or(Value::Null),
            "meta": prepared.get("meta").cloned().unwrap_or_else(|| normalize_meta_value(None)),
            "min_hours": min_hours,
            "max_items": max_items,
            "bytes": bytes,
            "requests": requests,
            "duration_ms": duration_ms,
            "quotes": quotes,
            "fetch_error": if fetch_error.is_empty() { Value::Null } else { Value::String(fetch_error) }
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    )
}

fn command_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let max_items = clamp_u64(payload, "max_items", 20, 1, 200) as usize;
    let min_hours = as_f64(payload.get("min_hours"), 1.0).clamp(0.0, 24.0 * 365.0);
    let force = as_bool(payload.get("force"), false);
    let timeout_ms = clamp_u64(payload, "timeout_ms", 15_000, 1_000, 120_000);
    let started_at_ms = Utc::now().timestamp_millis().max(0) as u64;

    let plan = command_build_fetch_plan(&Map::new());
    let request = plan
        .get("requests")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let fetch_url = {
        let override_url = clean_text(payload.get("url").and_then(Value::as_str), 800);
        if override_url.is_empty() {
            clean_text(request.get("url").and_then(Value::as_str), 800)
        } else {
            override_url
        }
    };
    let accept = clean_text(request.get("accept").and_then(Value::as_str), 160);

    let (market_html, bytes, requests, fetch_error) =
        match curl_fetch_with_status(&fetch_url, timeout_ms, &accept) {
            Ok((status, body, body_bytes)) => {
                if status >= 400 {
                    (
                        String::new(),
                        0_u64,
                        0_u64,
                        Some(http_status_to_code(status).to_string()),
                    )
                } else {
                    (body, body_bytes, 1_u64, None)
                }
            }
            Err(err) => {
                let code = clean_text(Some(&err), 120)
                    .split(':')
                    .next()
                    .unwrap_or("collector_error")
                    .to_string();
                (String::new(), 0_u64, 0_u64, Some(code))
            }
        };

    let duration_ms = Utc::now()
        .timestamp_millis()
        .max(0)
        .saturating_sub(started_at_ms as i64) as u64;

    command_collect(
        root,
        &json!({
            "eyes_state_dir": payload.get("eyes_state_dir").cloned().unwrap_or(Value::Null),
            "force": force,
            "min_hours": min_hours,
            "max_items": max_items,
            "bytes": bytes,
            "requests": requests,
            "duration_ms": duration_ms,
            "market_html": market_html,
            "fetch_error": fetch_error
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    )
}

fn dispatch(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "run" => command_run(root, payload),
        "prepare-run" => Ok(command_prepare_run(root, payload)),
        "build-fetch-plan" => Ok(command_build_fetch_plan(payload)),
        "finalize-run" => command_finalize_run(root, payload),
        "collect" => command_collect(root, payload),
        "extract-quotes" => {
            let html = payload.get("html").and_then(Value::as_str).unwrap_or("");
            let quotes = extract_quotes_from_html(html)
                .into_iter()
                .map(|q| quote_to_value(&q))
                .collect::<Vec<_>>();
            Ok(json!({ "ok": true, "quotes": quotes }))
        }
        "map-quotes" => Ok(map_quotes(payload)),
        "fallback-indices" => Ok(fallback_indices(payload)),
        _ => Err("stock_market_collector_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "stock_market_collector_kernel") {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "stock_market_collector_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);

    match dispatch(root, &command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt(
                "stock_market_collector_kernel",
                out,
            ));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "stock_market_collector_kernel_error",
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
#[path = "stock_market_collector_kernel_tests.rs"]
mod stock_market_collector_kernel_tests;
