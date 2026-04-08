// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::contract_lane_utils as lane_utils;

pub const COLLECTOR_ID: &str = "bird_x";
const EYES_STATE_DEFAULT_REL: &str = "local/state/sensory/eyes";

pub fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    lane_utils::clean_text(raw, max_len)
}

pub fn clamp_u64(payload: &Map<String, Value>, key: &str, fallback: u64, lo: u64, hi: u64) -> u64 {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

pub fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

pub fn as_f64(value: Option<&Value>, fallback: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(fallback)
}

pub fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

pub fn sha16(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex::encode(digest)[..16].to_string()
}

pub fn resolve_eyes_state_dir(root: &Path, payload: &Map<String, Value>) -> PathBuf {
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

pub fn meta_path_for(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    resolve_eyes_state_dir(root, payload)
        .join("collector_meta")
        .join(format!("{COLLECTOR_ID}.json"))
}

pub fn cache_path_for(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    resolve_eyes_state_dir(root, payload)
        .join("collector_meta")
        .join(format!("{COLLECTOR_ID}.cache.json"))
}

pub fn read_json(path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

pub fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("bird_x_collector_kernel_create_dir_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let body = format!(
        "{}\n",
        serde_json::to_string_pretty(value)
            .map_err(|err| format!("bird_x_collector_kernel_encode_failed:{err}"))?
    );
    fs::write(&tmp, body).map_err(|err| format!("bird_x_collector_kernel_write_failed:{err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("bird_x_collector_kernel_rename_failed:{err}"))
}

pub fn clean_seen_id(raw: &str) -> String {
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

pub fn normalize_meta_value(raw: Option<&Value>) -> Value {
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
        for row in items {
            if let Some(raw_id) = row.as_str() {
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

pub fn parse_iso_ms(raw: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

pub fn load_cache_items(root: &Path, payload: &Map<String, Value>) -> Vec<Value> {
    read_json(&cache_path_for(root, payload), json!({ "items": [] }))
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

pub fn infer_topics(content: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let text = content.to_lowercase();
    out.push("social".to_string());

    let mut add = |topic: &str| {
        if !out.iter().any(|existing| existing == topic) {
            out.push(topic.to_string());
        }
    };

    if text.contains("ai")
        || text.contains("llm")
        || text.contains("model")
        || text.contains("gpt")
        || text.contains("claude")
        || text.contains("agent")
        || text.contains("autonomous")
    {
        add("ai");
    }
    if text.contains("startup")
        || text.contains("venture")
        || text.contains("founder")
        || text.contains("ceo")
        || text.contains("business")
        || text.contains("revenue")
        || text.contains("mrr")
    {
        add("business");
    }
    if text.contains("code")
        || text.contains("dev")
        || text.contains("engineering")
        || text.contains("software")
        || text.contains("github")
        || text.contains("api")
    {
        add("dev");
    }
    if text.contains("moltbook")
        || text.contains("infring")
        || text.contains("clawhub")
        || text.contains("agent")
    {
        add("agent_community");
    }
    if text.contains("news")
        || text.contains("breaking")
        || text.contains("update")
        || text.contains("announced")
        || text.contains("launch")
    {
        add("news");
    }

    out.into_iter().take(6).collect::<Vec<_>>()
}

pub fn as_i64(obj: &Map<String, Value>, keys: &[&str]) -> i64 {
    for key in keys {
        if let Some(v) = obj.get(*key) {
            if let Some(n) = v.as_i64() {
                return n;
            }
            if let Some(n) = v.as_u64() {
                return n as i64;
            }
            if let Some(n) = v.as_f64() {
                if n.is_finite() {
                    return n.round() as i64;
                }
            }
        }
    }
    0
}

pub fn extract_author_parts(item: &Map<String, Value>) -> (String, String) {
    let mut handle = String::new();
    let mut name = String::new();

    let candidates = [
        item.get("author").and_then(Value::as_object),
        item.get("user").and_then(Value::as_object),
    ];
    for obj in candidates.into_iter().flatten() {
        if handle.is_empty() {
            handle = clean_text(
                obj.get("handle")
                    .and_then(Value::as_str)
                    .or_else(|| obj.get("username").and_then(Value::as_str)),
                80,
            );
        }
        if name.is_empty() {
            name = clean_text(
                obj.get("name")
                    .and_then(Value::as_str)
                    .or_else(|| obj.get("displayName").and_then(Value::as_str)),
                120,
            );
        }
    }

    if handle.is_empty() {
        handle = clean_text(item.get("author").and_then(Value::as_str), 80);
    }
    if name.is_empty() {
        name = clean_text(item.get("author_name").and_then(Value::as_str), 120);
    }
    if handle.is_empty() {
        handle = "unknown".to_string();
    }
    if name.is_empty() {
        name = handle.clone();
    }
    (handle, name)
}

pub fn first_line_title(content: &str, author_handle: &str) -> String {
    let first = content.lines().next().unwrap_or("");
    let title = clean_text(Some(first), 100);
    if title.is_empty() {
        format!("Post by @{}", clean_text(Some(author_handle), 80))
    } else {
        title
    }
}

pub fn normalize_seen_ids(payload: &Map<String, Value>) -> HashSet<String> {
    payload
        .get("seen_ids")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(clean_seen_id)
                .filter(|row| !row.is_empty())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default()
}

pub fn query_failures(payload: &Map<String, Value>) -> Vec<Value> {
    payload
        .get("query_failures")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_object().cloned())
        .map(|row| {
            json!({
                "code": clean_text(row.get("code").and_then(Value::as_str), 80),
                "message": clean_text(row.get("message").and_then(Value::as_str), 220),
                "http_status": row.get("http_status").and_then(Value::as_i64),
            })
        })
        .collect::<Vec<_>>()
}

pub fn bird_cli_present() -> bool {
    match Command::new("bird")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

pub fn normalize_queries(payload: &Map<String, Value>) -> Vec<String> {
    let mut out = payload
        .get("queries")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(Some(row), 200))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if out.is_empty() {
        out = vec![
            "AI agent".to_string(),
            "moltbook OR infring".to_string(),
            "local LLM ollama".to_string(),
        ];
    }
    out.into_iter().take(3).collect::<Vec<_>>()
}

pub fn normalize_retry_attempts(payload: &Map<String, Value>) -> u64 {
    clamp_u64(payload, "retry_attempts", 2, 1, 4)
}

pub fn normalize_timeout_ms(payload: &Map<String, Value>) -> u64 {
    clamp_u64(payload, "timeout_ms", 15_000, 1_000, 120_000)
}

pub fn normalize_max_items(payload: &Map<String, Value>) -> u64 {
    clamp_u64(payload, "max_items", 15, 1, 200)
}

pub fn normalize_max_items_per_query(payload: &Map<String, Value>) -> u64 {
    clamp_u64(payload, "max_items_per_query", 10, 1, 50)
}

pub fn sleep_backoff_ms(attempt_index: usize) -> u64 {
    120u64.saturating_mul((attempt_index as u64).saturating_add(1))
}

fn wait_for_output_with_timeout(
    mut child: std::process::Child,
    timeout_ms: u64,
) -> Result<std::process::Output, String> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|err| format!("bird_wait_output_failed:{err}"));
            }
            Ok(None) => {
                if start.elapsed().as_millis() as u64 >= timeout_ms {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("timeout:{timeout_ms}"));
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("bird_try_wait_failed:{err}"));
            }
        }
    }
}

pub fn run_bird_search_once(
    query: &str,
    max_items_per_query: u64,
    timeout_ms: u64,
) -> Result<(Vec<Value>, u64), Value> {
    let query = clean_text(Some(query), 200);
    if query.is_empty() {
        return Err(json!({
            "code": "collector_error",
            "message": "empty_query",
            "http_status": Value::Null
        }));
    }
    let spawn = Command::new("bird")
        .arg("search")
        .arg(query.clone())
        .arg("-n")
        .arg(max_items_per_query.to_string())
        .arg("--json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let child = match spawn {
        Ok(child) => child,
        Err(err) => {
            let code = match err.kind() {
                std::io::ErrorKind::NotFound => "env_blocked",
                _ => "collector_error",
            };
            return Err(json!({
                "code": code,
                "message": clean_text(Some(&format!("bird_spawn_failed:{err}")), 220),
                "http_status": Value::Null
            }));
        }
    };

    let output = match wait_for_output_with_timeout(child, timeout_ms) {
        Ok(output) => output,
        Err(err) if err.starts_with("timeout:") => {
            return Err(json!({
                "code": "timeout",
                "message": clean_text(Some(&format!("bird command timed out after {timeout_ms}ms")), 220),
                "http_status": Value::Null
            }));
        }
        Err(err) => {
            return Err(json!({
                "code": "collector_error",
                "message": clean_text(Some(&err), 220),
                "http_status": Value::Null
            }));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(json!({
            "code": "parse_failed",
            "message": clean_text(Some(&format!("bird_nonzero_status:{} {}", output.status, stderr)), 220),
            "http_status": Value::Null
        }));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let bytes = stdout.len() as u64;
    let results = match serde_json::from_str::<Value>(&stdout) {
        Ok(Value::Array(rows)) => rows,
        Ok(_) => {
            return Err(json!({
                "code": "parse_failed",
                "message": "bird_output_not_array",
                "http_status": Value::Null
            }))
        }
        Err(err) => {
            return Err(json!({
                "code": "parse_failed",
                "message": clean_text(Some(&format!("bird_output_parse_failed:{err}")), 220),
                "http_status": Value::Null
            }))
        }
    };
    Ok((results, bytes))
}
