// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::{DateTime, Utc};
use regex::Regex;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::contract_lane_utils as lane_utils;

fn usage() {
    println!("upwork-gigs-collector-kernel commands:");
    println!("  protheus-ops upwork-gigs-collector-kernel run --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel prepare-run --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel build-fetch-plan --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel finalize-run --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel parse-rss --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel map-gigs --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel fallback-gigs --payload-base64=<json>");
    println!("  protheus-ops upwork-gigs-collector-kernel collect --payload-base64=<json>");
}

const COLLECTOR_ID: &str = "upwork_gigs";
const EYES_STATE_DEFAULT_REL: &str = "local/state/sensory/eyes";

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    crate::contract_lane_utils::clean_text(raw, max_len)
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
            let candidate = PathBuf::from(trimmed);
            return if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            };
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
            .map_err(|err| format!("upwork_gigs_collector_kernel_create_dir_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!("tmp-{}-{}", std::process::id(), Utc::now().timestamp_millis()));
    let body = format!(
        "{}\n",
        serde_json::to_string_pretty(value)
            .map_err(|err| format!("upwork_gigs_collector_kernel_encode_failed:{err}"))?
    );
    fs::write(&tmp, body).map_err(|err| format!("upwork_gigs_collector_kernel_write_failed:{err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("upwork_gigs_collector_kernel_rename_failed:{err}"))
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
    let last_run = clean_text(obj.and_then(|o| o.get("last_run")).and_then(Value::as_str), 80);
    let last_success = clean_text(obj.and_then(|o| o.get("last_success")).and_then(Value::as_str), 80);
    let mut seen_ids = Vec::new();
    if let Some(items) = obj.and_then(|o| o.get("seen_ids")).and_then(Value::as_array) {
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

fn encode_query_component(raw: &str) -> String {
    let mut out = String::new();
    for b in raw.bytes() {
        let keep = b.is_ascii_uppercase()
            || b.is_ascii_lowercase()
            || b.is_ascii_digit()
            || matches!(b, b'-' | b'_' | b'.' | b'~');
        if keep {
            out.push(char::from(b));
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn classify_curl_transport_error(stderr: &str) -> String {
    lane_utils::classify_curl_transport_error(stderr)
}

fn http_status_to_code(status: u64) -> &'static str {
    lane_utils::http_status_to_code(status)
}

fn curl_fetch_with_status(url: &str, timeout_ms: u64, accept: &str) -> Result<(u64, String, u64), String> {
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

fn strip_tags(raw: &str) -> String {
    let no_tags = match Regex::new(r"(?is)<[^>]+>") {
        Ok(re) => re.replace_all(raw, " ").to_string(),
        Err(_) => raw.to_string(),
    };
    clean_text(Some(&no_tags), 1_600)
}

fn decode_html(raw: &str) -> String {
    raw.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn extract_tag(block: &str, tag: &str, max_len: usize) -> String {
    let pat = format!(r"(?is)<{}\b[^>]*>(.*?)</{}\s*>", regex::escape(tag), regex::escape(tag));
    let re = match Regex::new(&pat) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };
    re.captures(block)
        .and_then(|caps| caps.get(1).map(|m| m.as_str()))
        .map(|raw| clean_text(Some(&decode_html(&strip_tags(raw))), max_len))
        .unwrap_or_default()
}

fn parse_rss(xml: &str) -> Vec<Value> {
    let item_re = match Regex::new(r"(?is)<item\b.*?</item>") {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::<Value>::new();
    for mat in item_re.find_iter(xml) {
        let block = mat.as_str();
        let title = extract_tag(block, "title", 220);
        let url = extract_tag(block, "link", 600);
        if title.is_empty() || url.is_empty() {
            continue;
        }
        out.push(json!({
            "title": title,
            "url": url,
            "description": extract_tag(block, "description", 420),
            "pubDate": extract_tag(block, "pubDate", 120),
            "budget": extract_tag(block, "budget", 120),
        }));
    }
    out
}

fn score_keywords() -> &'static [&'static str] {
    &[
        "ai",
        "artificial intelligence",
        "automation",
        "chatbot",
        "gpt",
        "llm",
        "openai",
        "claude",
        "frontier_provider",
        "agent",
        "workflow",
        "n8n",
        "make",
        "nocode",
        "no-code",
        "lowcode",
        "bubble",
        "webflow",
        "zapier",
        "script",
        "bot",
        "scraper",
        "api integration",
        "webhook",
        "chrome extension",
        "browser extension",
        "plugin",
        "data pipeline",
        "etl",
        "database",
        "supabase",
        "firebase",
        "nextjs",
        "next.js",
        "react",
        "typescript",
        "javascript",
    ]
}

fn score_gig_value(title: &str, description: &str) -> i64 {
    let hay = format!("{title} {description}").to_lowercase();
    let mut score = 0_i64;
    for kw in score_keywords() {
        if hay.contains(kw) {
            score += 2;
        }
    }
    if hay.contains("$$$$") || hay.contains("fixed price") {
        score += 3;
    }
    if hay.contains("$$$") {
        score += 2;
    }
    score
}

fn normalize_seen_ids(payload: &Map<String, Value>) -> HashSet<String> {
    payload
        .get("seen_ids")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(Some(row), 120))
                .filter(|row| !row.is_empty())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default()
}

fn date_seed(payload: &Map<String, Value>) -> String {
    let raw = clean_text(payload.get("date").and_then(Value::as_str), 32);
    if raw.is_empty() { today_utc() } else { raw }
}
