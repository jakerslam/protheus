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

fn is_retryable_code(code: &str) -> bool {
    matches!(
        code.trim().to_ascii_lowercase().as_str(),
        "env_blocked"
            | "dns_unreachable"
            | "connection_refused"
            | "connection_reset"
            | "timeout"
            | "http_5xx"
            | "rate_limited"
            | "http_error"
            | "collector_error"
    )
}

fn is_transport_failure_code(code: &str) -> bool {
    matches!(
        code.trim().to_ascii_lowercase().as_str(),
        "env_blocked"
            | "dns_unreachable"
            | "connection_refused"
            | "connection_reset"
            | "timeout"
            | "tls_error"
            | "http_4xx"
            | "http_404"
            | "http_5xx"
            | "rate_limited"
            | "http_error"
    )
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
    if s.contains("enotfound") || s.contains("getaddrinfo") || s.contains("dns") || s.contains("eai_again") {
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
        fs::create_dir_all(parent).map_err(|err| format!("collector_runtime_kernel_create_dir_failed:{err}"))?;
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
        obj.and_then(|o| o.get("last_success")).and_then(Value::as_str),
        80,
    );
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
        fs::create_dir_all(parent).map_err(|err| format!("collector_runtime_kernel_create_dir_failed:{err}"))?;
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
    if state_obj.get("schema_id").and_then(Value::as_str).unwrap_or("") != RATE_SCHEMA_ID {
        state_obj.insert("schema_id".to_string(), Value::String(RATE_SCHEMA_ID.to_string()));
    }
    if !state_obj.get("collectors").map(Value::is_object).unwrap_or(false) {
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
        .map(Value::is_object)
        .unwrap_or(false)
    {
        collectors.insert(
            collector_id.to_string(),
            json!({
                "last_attempt_ms": 0,
                "last_success_ms": 0,
                "failure_streak": 0,
                "next_allowed_ms": 0,
                "circuit_open_until_ms": 0,
                "last_error_code": null
            }),
        );
    }
    collectors
        .get_mut(collector_id)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "collector_runtime_kernel_row_not_object".to_string())
}

fn row_u64(row: &Map<String, Value>, key: &str) -> u64 {
    row.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn set_row_u64(row: &mut Map<String, Value>, key: &str, value: u64) {
    row.insert(key.to_string(), Value::Number(value.into()));
}

fn render_row(row: &Map<String, Value>) -> Value {
    json!({
        "last_attempt_ms": row_u64(row, "last_attempt_ms"),
        "last_success_ms": row_u64(row, "last_success_ms"),
        "failure_streak": row_u64(row, "failure_streak"),
        "next_allowed_ms": row_u64(row, "next_allowed_ms"),
        "circuit_open_until_ms": row_u64(row, "circuit_open_until_ms"),
        "last_error_code": row.get("last_error_code").cloned().unwrap_or(Value::Null)
    })
}

fn handle_prepare_attempt(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let min_interval_ms = json_u64(
        payload,
        "min_interval_ms",
        default_u64_from_env("EYES_COLLECTOR_MIN_INTERVAL_MS", 300, 50, 30_000),
        50,
        30_000,
    );
    let state_path = resolve_rate_state_path(root, payload);

    let mut state = read_state(&state_path);
    let collectors = ensure_collectors_mut(&mut state)?;
    let row = ensure_row_mut(collectors, &collector_id)?;

    let now = now_ms_u64();
    let circuit_open_until_ms = row_u64(row, "circuit_open_until_ms");
    if circuit_open_until_ms > now {
        return Ok(json!({
            "ok": true,
            "collector_id": collector_id,
            "circuit_open": true,
            "retry_after_ms": circuit_open_until_ms.saturating_sub(now),
            "row": render_row(row),
            "rate_state_path": state_path.display().to_string()
        }));
    }

    let last_attempt_ms = row_u64(row, "last_attempt_ms");
    let next_allowed_ms = row_u64(row, "next_allowed_ms");
    let ready_at = max(next_allowed_ms, last_attempt_ms.saturating_add(min_interval_ms));
    let wait_ms = ready_at.saturating_sub(now);
    if wait_ms > 0 {
        thread::sleep(Duration::from_millis(wait_ms));
    }

    let attempted_at_ms = now_ms_u64();
    set_row_u64(row, "last_attempt_ms", attempted_at_ms);
    let row_snapshot = render_row(row);
    write_state(&state_path, &state)?;

    Ok(json!({
        "ok": true,
        "collector_id": collector_id,
        "circuit_open": false,
        "wait_ms": wait_ms,
        "attempted_at_ms": attempted_at_ms,
        "row": row_snapshot,
        "rate_state_path": state_path.display().to_string()
    }))
}

fn handle_mark_success(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let min_interval_ms = json_u64(
        payload,
        "min_interval_ms",
        default_u64_from_env("EYES_COLLECTOR_MIN_INTERVAL_MS", 300, 50, 30_000),
        50,
        30_000,
    );
    let state_path = resolve_rate_state_path(root, payload);

    let mut state = read_state(&state_path);
    let collectors = ensure_collectors_mut(&mut state)?;
    let row = ensure_row_mut(collectors, &collector_id)?;
    let now = now_ms_u64();

    set_row_u64(row, "last_success_ms", now);
    set_row_u64(row, "failure_streak", 0);
    set_row_u64(row, "next_allowed_ms", now.saturating_add(min_interval_ms));
    set_row_u64(row, "circuit_open_until_ms", 0);
    row.insert("last_error_code".to_string(), Value::Null);
    let row_snapshot = render_row(row);

    write_state(&state_path, &state)?;

    Ok(json!({
        "ok": true,
        "collector_id": collector_id,
        "row": row_snapshot,
        "rate_state_path": state_path.display().to_string()
    }))
}

fn handle_mark_failure(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let last_error_code = lane_utils::clean_token(payload.get("code").and_then(Value::as_str), "collector_error");
    let retryable = is_retryable_code(&last_error_code);

    let base_backoff_ms = json_u64(
        payload,
        "base_backoff_ms",
        default_u64_from_env("EYES_COLLECTOR_BACKOFF_BASE_MS", 300, 50, 30_000),
        50,
        30_000,
    );
    let max_backoff_ms = json_u64(
        payload,
        "max_backoff_ms",
        default_u64_from_env("EYES_COLLECTOR_BACKOFF_MAX_MS", 8_000, 200, 120_000),
        200,
        120_000,
    );
    let circuit_open_ms = json_u64(
        payload,
        "circuit_open_ms",
        default_u64_from_env("EYES_COLLECTOR_CIRCUIT_MS", 30_000, 500, 300_000),
        500,
        300_000,
    );
    let circuit_after_failures = json_u64(
        payload,
        "circuit_after_failures",
        default_u64_from_env("EYES_COLLECTOR_CIRCUIT_AFTER", 3, 1, 10),
        1,
        10,
    );

    let state_path = resolve_rate_state_path(root, payload);

    let mut state = read_state(&state_path);
    let collectors = ensure_collectors_mut(&mut state)?;
    let row = ensure_row_mut(collectors, &collector_id)?;
    let now = now_ms_u64();

    let next_failure_streak = row_u64(row, "failure_streak").saturating_add(1);
    set_row_u64(row, "failure_streak", next_failure_streak);

    if retryable {
        let exp = next_failure_streak.saturating_sub(1).min(16);
        let backoff_ms = std::cmp::min(max_backoff_ms, base_backoff_ms.saturating_mul(2_u64.pow(exp as u32)));
        set_row_u64(row, "next_allowed_ms", now.saturating_add(backoff_ms));
        if next_failure_streak >= circuit_after_failures {
            set_row_u64(row, "circuit_open_until_ms", now.saturating_add(circuit_open_ms));
        }
    } else {
        set_row_u64(
            row,
            "next_allowed_ms",
            now.saturating_add(std::cmp::min(max_backoff_ms, base_backoff_ms)),
        );
    }

    row.insert("last_error_code".to_string(), Value::String(last_error_code.clone()));
    let row_snapshot = render_row(row);

    write_state(&state_path, &state)?;

    Ok(json!({
        "ok": true,
        "collector_id": collector_id,
        "retryable": retryable,
        "last_error_code": last_error_code,
        "row": row_snapshot,
        "rate_state_path": state_path.display().to_string()
    }))
}

fn handle_prepare_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let force = json_bool(payload, "force", false);
    let min_hours = json_f64(payload, "min_hours", 4.0, 0.0, 24.0 * 365.0);

    let meta_path = meta_path_for(root, payload, &collector_id);
    let meta = normalize_meta_value(&collector_id, Some(&read_json(&meta_path, normalize_meta_value(&collector_id, None))));
    let last_run_ms = meta
        .get("last_run")
        .and_then(Value::as_str)
        .and_then(parse_iso_ms);
    let hours_since_last = last_run_ms.map(|ms| ((chrono::Utc::now().timestamp_millis() - ms) as f64 / 3_600_000.0).max(0.0));
    let skipped = !force && hours_since_last.map(|h| h < min_hours).unwrap_or(false);

    Ok(json!({
        "ok": true,
        "collector_id": collector_id,
        "force": force,
        "min_hours": min_hours,
        "hours_since_last": hours_since_last,
        "skipped": skipped,
        "reason": if skipped { Value::String("cadence".to_string()) } else { Value::Null },
        "meta": meta,
        "meta_path": meta_path.display().to_string()
    }))
}

fn handle_begin_collection(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let controls = crate::collector_runtime_kernel_support::resolve_controls(payload);
    let collector_id = lane_utils::clean_token(
        controls.get("collector_id").and_then(Value::as_str),
        "collector",
    );
    let min_hours = controls
        .get("min_hours")
        .and_then(Value::as_f64)
        .unwrap_or(4.0)
        .clamp(0.0, 24.0 * 365.0);
    let force = controls
        .get("force")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let prepared = handle_prepare_run(
        root,
        payload_obj(&json!({
            "collector_id": collector_id,
            "eyes_state_dir": payload.get("eyes_state_dir").cloned().unwrap_or(Value::Null),
            "force": force,
            "min_hours": min_hours
        })),
    )?;
    if prepared.get("skipped").and_then(Value::as_bool) == Some(true) {
        return Ok(json!({
            "ok": true,
            "success": true,
            "eye": collector_id,
            "skipped": true,
            "reason": "cadence",
            "hours_since_last": prepared.get("hours_since_last").cloned().unwrap_or(Value::Null),
            "min_hours": min_hours,
            "items": [],
            "controls": controls,
            "meta": prepared.get("meta").cloned().unwrap_or(Value::Object(Map::new()))
        }));
    }
    Ok(json!({
        "ok": true,
        "success": true,
        "skipped": false,
        "eye": collector_id,
        "min_hours": min_hours,
        "max_items": controls.get("max_items").cloned().unwrap_or(Value::from(20)),
        "controls": controls,
        "meta": prepared.get("meta").cloned().unwrap_or(Value::Object(Map::new()))
    }))
}

fn handle_classify_error(payload: &Map<String, Value>) -> Value {
    let message = lane_utils::clean_text(payload.get("message").and_then(Value::as_str), 200);
    let status_from_err = payload
        .get("http_status")
        .or_else(|| payload.get("status"))
        .and_then(Value::as_u64)
        .filter(|v| *v > 0);
    let status_from_msg = parse_http_status_from_message(&message);
    let http_status = status_from_err.or(status_from_msg);

    let mut code = normalize_node_code(payload.get("code").and_then(Value::as_str).unwrap_or(""));
    if code.is_empty() {
        if let Some(status) = http_status {
            code = http_status_to_code(status).to_string();
        }
    }
    if code.is_empty() {
        code = classify_message(&message);
    }
    if code.is_empty() {
        code = "collector_error".to_string();
    }

    json!({
        "ok": true,
        "code": code.clone(),
        "message": message,
        "http_status": http_status,
        "transport": is_transport_failure_code(&code),
        "retryable": is_retryable_code(&code)
    })
}

fn compute_bytes_from_items(items: &[Value]) -> u64 {
    items
        .iter()
        .filter_map(Value::as_object)
        .map(|row| row.get("bytes").and_then(Value::as_u64).unwrap_or(0))
        .sum()
}

fn sample_title(items: &[Value]) -> Value {
    items
        .first()
        .and_then(Value::as_object)
        .and_then(|o| o.get("title"))
        .and_then(Value::as_str)
        .map(|v| lane_utils::clean_text(Some(v), 120))
        .filter(|v| !v.is_empty())
        .map(Value::String)
        .unwrap_or(Value::Null)
}

fn handle_finalize_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let min_hours = json_f64(payload, "min_hours", 4.0, 0.0, 24.0 * 365.0);
    let max_items = json_u64(payload, "max_items", 20, 1, 200) as usize;
    let use_cache_when_empty = json_bool(payload, "use_cache_when_empty", false);
    let bytes = json_u64(payload, "bytes", 0, 0, u64::MAX);
    let requests = json_u64(payload, "requests", 1, 0, u64::MAX);
    let duration_ms = json_u64(payload, "duration_ms", 0, 0, u64::MAX);
    let fetch_error_code = lane_utils::clean_text(payload.get("fetch_error_code").and_then(Value::as_str), 80);
    let fetch_error = if fetch_error_code.is_empty() {
        String::new()
    } else {
        fetch_error_code
    };
    let http_status = payload.get("http_status").and_then(Value::as_u64);

    let mut items = payload
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if items.len() > max_items {
        items = items.into_iter().take(max_items).collect::<Vec<_>>();
    }

    if items.is_empty() && (use_cache_when_empty || !fetch_error.is_empty()) {
        let cache = read_json(
            &cache_path_for(root, payload, &collector_id),
            json!({ "items": [] }),
        );
        let mut cached = cache
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if !cached.is_empty() {
            if cached.len() > max_items {
                cached = cached.into_iter().take(max_items).collect::<Vec<_>>();
            }
            return Ok(json!({
                "ok": true,
                "success": true,
                "eye": collector_id,
                "cache_hit": true,
                "degraded": !fetch_error.is_empty(),
                "error": if fetch_error.is_empty() { Value::Null } else { Value::String(fetch_error.clone()) },
                "items": cached,
                "bytes": compute_bytes_from_items(&cached),
                "requests": requests,
                "duration_ms": duration_ms,
                "cadence_hours": min_hours,
                "sample": sample_title(&cached)
            }));
        }
    }

    let mut meta = normalize_meta_value(&collector_id, payload.get("meta"));
    let now_iso = chrono::Utc::now().to_rfc3339();
    meta["last_run"] = Value::String(now_iso.clone());
    if !items.is_empty() {
        meta["last_success"] = Value::String(now_iso);
        write_json(
            &cache_path_for(root, payload, &collector_id),
            &json!({ "items": items.clone() }),
        )?;
    }
    write_json(&meta_path_for(root, payload, &collector_id), &meta)?;

    if items.is_empty() && !fetch_error.is_empty() {
        return Ok(json!({
            "ok": true,
            "success": false,
            "eye": collector_id,
            "items": [],
            "bytes": bytes,
            "requests": requests,
            "duration_ms": duration_ms,
            "error": fetch_error,
            "http_status": http_status,
            "cadence_hours": min_hours
        }));
    }

    Ok(json!({
        "ok": true,
        "success": true,
        "eye": collector_id,
        "items": items,
        "bytes": bytes,
        "requests": requests,
        "duration_ms": duration_ms,
        "cadence_hours": min_hours,
        "sample": sample_title(&items)
    }))
}

fn handle_fetch_text(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let url = lane_utils::clean_text(payload.get("url").and_then(Value::as_str), 800);
    if url.is_empty() {
        return Err("collector_runtime_kernel_fetch_text_missing_url".to_string());
    }
    let attempts = json_u64(payload, "attempts", 3, 1, 5);
    let timeout_ms = json_u64(payload, "timeout_ms", 15_000, 1_000, 120_000);
    let headers = crate::collector_runtime_kernel_support::parse_headers(payload);
    let mut last_error = "collector_error".to_string();

    for attempt in 1..=attempts {
        let prep = handle_prepare_attempt(
            root,
            payload_obj(&json!({
                "collector_id": collector_id.clone(),
                "rate_state_path": payload.get("rate_state_path").cloned().unwrap_or(Value::Null),
                "min_interval_ms": payload.get("min_interval_ms").cloned().unwrap_or(Value::Null)
            })),
        )?;
        if prep.get("circuit_open").and_then(Value::as_bool) == Some(true) {
            let retry_after = prep.get("retry_after_ms").and_then(Value::as_u64).unwrap_or(0);
            return Err(format!("rate_limited:circuit_open:{retry_after}"));
        }

        match crate::collector_runtime_kernel_support::curl_fetch_with_status(&url, timeout_ms, &headers) {
            Ok((status, body, bytes)) if status < 400 => {
                let _ = handle_mark_success(
                    root,
                    payload_obj(&json!({
                        "collector_id": collector_id.clone(),
                        "rate_state_path": payload.get("rate_state_path").cloned().unwrap_or(Value::Null),
                        "min_interval_ms": payload.get("min_interval_ms").cloned().unwrap_or(Value::Null)
                    })),
                );
                return Ok(json!({
                    "ok": true,
                    "collector_id": collector_id.clone(),
                    "status": status,
                    "text": body,
                    "bytes": bytes,
                    "attempt": attempt
                }));
            }
            Ok((status, _, _)) => {
                last_error = http_status_to_code(status).to_string();
            }
            Err(err) => {
                last_error = crate::collector_runtime_kernel_support::split_error_code(&err);
            }
        }

        let mark = handle_mark_failure(
            root,
            payload_obj(&json!({
                "collector_id": collector_id.clone(),
                "rate_state_path": payload.get("rate_state_path").cloned().unwrap_or(Value::Null),
                "code": last_error.clone(),
                "base_backoff_ms": payload.get("base_backoff_ms").cloned().unwrap_or(Value::Null),
                "max_backoff_ms": payload.get("max_backoff_ms").cloned().unwrap_or(Value::Null),
                "circuit_open_ms": payload.get("circuit_open_ms").cloned().unwrap_or(Value::Null),
                "circuit_after_failures": payload.get("circuit_after_failures").cloned().unwrap_or(Value::Null)
            })),
        )?;
        let retryable = mark.get("retryable").and_then(Value::as_bool).unwrap_or(false);
        if !retryable || attempt >= attempts {
            break;
        }
    }

    Err(format!("collector_runtime_kernel_fetch_text_failed:{last_error}"))
}

fn dispatch(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "classify-error" => Ok(handle_classify_error(payload)),
        "resolve-controls" => Ok(crate::collector_runtime_kernel_support::resolve_controls(payload)),
        "begin-collection" => handle_begin_collection(root, payload),
        "prepare-run" => handle_prepare_run(root, payload),
        "finalize-run" => handle_finalize_run(root, payload),
        "fetch-text" => handle_fetch_text(root, payload),
        "prepare-attempt" => handle_prepare_attempt(root, payload),
        "mark-success" => handle_mark_success(root, payload),
        "mark-failure" => handle_mark_failure(root, payload),
        _ => Err("collector_runtime_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "collector_runtime_kernel") {
        Ok(value) => value,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error("collector_runtime_kernel_error", &err));
            return 1;
        }
    };
    let payload_obj = payload_obj(&payload);

    match dispatch(root, &command, payload_obj) {
        Ok(value) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt("collector_runtime_kernel", value));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error("collector_runtime_kernel_error", &err));
            1
        }
    }
}

#[cfg(test)]
#[path = "collector_runtime_kernel_tests.rs"]
mod collector_runtime_kernel_tests;
