// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::cmp::max;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::contract_lane_utils as lane_utils;

const DEFAULT_RATE_STATE_REL: &str = "local/state/sensory/eyes/collector_rate_state.json";
const RATE_SCHEMA_ID: &str = "collector_rate_state_v1";
const EYES_STATE_DEFAULT_REL: &str = "local/state/sensory/eyes";

fn usage() {
    println!("collector-runtime-kernel commands:");
    println!("  protheus-ops collector-runtime-kernel classify-error --payload-base64=<json>");
    println!("  protheus-ops collector-runtime-kernel resolve-controls --payload-base64=<json>");
    println!("  protheus-ops collector-runtime-kernel begin-collection --payload-base64=<json>");
    println!("  protheus-ops collector-runtime-kernel prepare-run --payload-base64=<json>");
    println!("  protheus-ops collector-runtime-kernel finalize-run --payload-base64=<json>");
    println!("  protheus-ops collector-runtime-kernel fetch-text --payload-base64=<json>");
    println!("  protheus-ops collector-runtime-kernel prepare-attempt --payload-base64=<json>");
    println!("  protheus-ops collector-runtime-kernel mark-success --payload-base64=<json>");
    println!("  protheus-ops collector-runtime-kernel mark-failure --payload-base64=<json>");
}

fn now_ms_u64() -> u64 {
    let ts = chrono::Utc::now().timestamp_millis();
    if ts <= 0 {
        0
    } else {
        ts as u64
    }
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    lane_utils::payload_obj(value)
}

fn json_u64(payload: &Map<String, Value>, key: &str, fallback: u64, lo: u64, hi: u64) -> u64 {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn json_bool(payload: &Map<String, Value>, key: &str, fallback: bool) -> bool {
    payload
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

fn json_f64(payload: &Map<String, Value>, key: &str, fallback: f64, lo: f64, hi: f64) -> f64 {
    payload
        .get(key)
        .and_then(Value::as_f64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn code_matches_any(code: &str, allowed: &[&str]) -> bool {
    let normalized = code.trim().to_ascii_lowercase();
    allowed.iter().any(|candidate| normalized == *candidate)
}

fn is_retryable_code(code: &str) -> bool {
    const RETRYABLE: [&str; 9] = [
        "env_blocked",
        "dns_unreachable",
        "connection_refused",
        "connection_reset",
        "timeout",
        "http_5xx",
        "rate_limited",
        "http_error",
        "collector_error",
    ];
    code_matches_any(code, &RETRYABLE)
}

fn is_transport_failure_code(code: &str) -> bool {
    const TRANSPORT_FAILURES: [&str; 11] = [
        "env_blocked",
        "dns_unreachable",
        "connection_refused",
        "connection_reset",
        "timeout",
        "tls_error",
        "http_4xx",
        "http_404",
        "http_5xx",
        "rate_limited",
        "http_error",
    ];
    code_matches_any(code, &TRANSPORT_FAILURES)
}

fn http_status_to_code(status: u64) -> &'static str {
    lane_utils::http_status_to_code(status)
}

fn normalize_node_code(raw: &str) -> String {
    let c = raw.trim().to_ascii_lowercase();
    if c.is_empty() {
        return String::new();
    }
    match c.as_str() {
        "auth_missing"
        | "auth_unauthorized"
        | "auth_forbidden"
        | "env_blocked"
        | "dns_unreachable"
        | "connection_refused"
        | "connection_reset"
        | "timeout"
        | "tls_error"
        | "rate_limited"
        | "http_4xx"
        | "http_404"
        | "http_5xx"
        | "http_error"
        | "network_error"
        | "endpoint_unsupported"
        | "collector_error" => c,
        "enotfound" | "eai_again" => "dns_unreachable".to_string(),
        "eperm" => "env_blocked".to_string(),
        "econnrefused" => "connection_refused".to_string(),
        "econnreset" => "connection_reset".to_string(),
        "etimedout" | "esockettimedout" => "timeout".to_string(),
        "unauthorized" => "auth_unauthorized".to_string(),
        "forbidden" => "auth_forbidden".to_string(),
        _ => {
            if c.contains("cert") || c.contains("ssl") || c.contains("tls") {
                "tls_error".to_string()
            } else {
                String::new()
            }
        }
    }
}

fn parse_http_status_from_message(msg: &str) -> Option<u64> {
    let lower = msg.to_ascii_lowercase();
    let idx = lower.find("http ")?;
    let rest = lower.get((idx + 5)..)?;
    let digits = rest
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.len() == 3 {
        digits.parse::<u64>().ok()
    } else {
        None
    }
}

fn classify_message(msg: &str) -> String {
    let s = msg.to_ascii_lowercase();
    if s.is_empty() {
        return String::new();
    }
    if s.contains("missing_moltbook_api_key") || s.contains("missing api key") {
        return "auth_missing".to_string();
    }
    if s.contains("unauthorized") {
        return "auth_unauthorized".to_string();
    }
    if s.contains("forbidden") {
        return "auth_forbidden".to_string();
    }
    if s.contains("enotfound")
        || s.contains("getaddrinfo")
        || s.contains("dns")
        || s.contains("eai_again")
    {
        return "dns_unreachable".to_string();
    }
    if s.contains("operation not permitted") || s.contains("permission denied") {
        return "env_blocked".to_string();
    }
    if s.contains("econnrefused") || s.contains("connection refused") {
        return "connection_refused".to_string();
    }
    if s.contains("econnreset") {
        return "connection_reset".to_string();
    }
    if s.contains("timed out") || s.contains("timeout") || s.contains("etimedout") {
        return "timeout".to_string();
    }
    if s.contains("ssl") || s.contains("tls") || s.contains("certificate") {
        return "tls_error".to_string();
    }
    if let Some(status) = parse_http_status_from_message(&s) {
        return http_status_to_code(status).to_string();
    }
    String::new()
}

fn clean_collector_id(payload: &Map<String, Value>) -> String {
    lane_utils::clean_token(
        payload.get("collector_id").and_then(Value::as_str),
        "collector",
    )
}

fn default_u64_from_env(key: &str, fallback: u64, lo: u64, hi: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn resolve_rate_state_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Some(raw) = payload.get("rate_state_path").and_then(Value::as_str) {
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
            return PathBuf::from(trimmed).join("collector_rate_state.json");
        }
    }

    root.join(DEFAULT_RATE_STATE_REL)
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

fn meta_path_for(root: &Path, payload: &Map<String, Value>, collector_id: &str) -> PathBuf {
    resolve_eyes_state_dir(root, payload)
        .join("collector_meta")
        .join(format!("{collector_id}.json"))
}

fn cache_path_for(root: &Path, payload: &Map<String, Value>, collector_id: &str) -> PathBuf {
    resolve_eyes_state_dir(root, payload)
        .join("collector_meta")
        .join(format!("{collector_id}.cache.json"))
}

fn read_json(path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("collector_runtime_kernel_create_dir_failed:{err}"))?;
    }
    let body = format!(
        "{}\n",
        serde_json::to_string_pretty(value)
            .map_err(|err| format!("collector_runtime_kernel_encode_failed:{err}"))?
    );
    fs::write(path, body).map_err(|err| format!("collector_runtime_kernel_write_failed:{err}"))
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

fn normalize_meta_value(collector_id: &str, raw: Option<&Value>) -> Value {
    let obj = raw.and_then(Value::as_object);
    let last_run = lane_utils::clean_text(
        obj.and_then(|o| o.get("last_run")).and_then(Value::as_str),
        80,
    );
    let last_success = lane_utils::clean_text(
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
        "collector_id": collector_id,
        "last_run": if last_run.is_empty() { Value::Null } else { Value::String(last_run) },
        "last_success": if last_success.is_empty() { Value::Null } else { Value::String(last_success) },
        "seen_ids": seen_ids
    })
}

fn parse_iso_ms(raw: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn default_state_json() -> Value {
    json!({
        "schema_id": RATE_SCHEMA_ID,
        "collectors": {}
    })
}

fn read_state(path: &Path) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| default_state_json()),
        Err(_) => default_state_json(),
    }
}

fn write_state(path: &Path, state: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("collector_runtime_kernel_create_dir_failed:{err}"))?;
    }
    let pretty = serde_json::to_string_pretty(state)
        .map_err(|err| format!("collector_runtime_kernel_encode_failed:{err}"))?;
    fs::write(path, format!("{pretty}\n"))
        .map_err(|err| format!("collector_runtime_kernel_write_failed:{err}"))
}

fn ensure_collectors_mut(state: &mut Value) -> Result<&mut Map<String, Value>, String> {
    if !state.is_object() {
        *state = default_state_json();
    }
    let state_obj = state
        .as_object_mut()
        .ok_or_else(|| "collector_runtime_kernel_state_not_object".to_string())?;
    if state_obj
        .get("schema_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        != RATE_SCHEMA_ID
    {
        state_obj.insert(
            "schema_id".to_string(),
            Value::String(RATE_SCHEMA_ID.to_string()),
        );
    }
    if !state_obj
        .get("collectors")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state_obj.insert("collectors".to_string(), Value::Object(Map::new()));
    }
    state_obj
        .get_mut("collectors")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "collector_runtime_kernel_collectors_not_object".to_string())
}

fn ensure_row_mut<'a>(
    collectors: &'a mut Map<String, Value>,
    collector_id: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    if !collectors
        .get(collector_id)
