// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const AGENT_SESSIONS_DIR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_sessions";

fn now_iso() -> String {
    crate::now_iso()
}

fn clean_text(value: &str, max_len: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn normalize_agent_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 140).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        }
    }
    out
}

fn normalize_memory_key(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 120).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' || ch == ':' {
            out.push(ch);
        }
    }
    out
}

fn parse_json_loose(text: &str) -> Option<Value> {
    if text.trim().is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return Some(value);
    }
    for line in text.lines().rev() {
        let candidate = line.trim();
        if candidate.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(candidate) {
            return Some(value);
        }
    }
    None
}

fn read_json_file(path: &Path) -> Option<Value> {
    let body = fs::read_to_string(path).ok()?;
    parse_json_loose(&body)
}

fn ensure_dir(path: &Path) {
    let _ = fs::create_dir_all(path);
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        ensure_dir(parent);
    }
    if let Ok(body) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{body}\n"));
    }
}

fn value_search_text(value: &Value, max_len: usize) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(v) => {
            if *v {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(v) => clean_text(&v.to_string(), max_len),
        Value::String(v) => clean_text(v, max_len),
        _ => clean_text(&value.to_string(), max_len),
    }
}

fn query_tokens(query: &str, max_tokens: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for token in clean_text(query, 600)
        .to_ascii_lowercase()
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.'))
    {
        let normalized = token.trim();
        if normalized.len() < 2 {
            continue;
        }
        if seen.insert(normalized.to_string()) {
            out.push(normalized.to_string());
        }
        if out.len() >= max_tokens.max(1) {
            break;
        }
    }
    out
}

fn memory_semantic_score(key: &str, value_text: &str, query_tokens: &[String]) -> i64 {
    if query_tokens.is_empty() {
        return 0;
    }
    let key_lc = clean_text(key, 240).to_ascii_lowercase();
    let value_lc = clean_text(value_text, 2000).to_ascii_lowercase();
    let mut score = 0i64;
    for token in query_tokens {
        if key_lc == *token {
            score += 8;
            continue;
        }
        if key_lc.contains(token) {
            score += 5;
        }
        if value_lc.contains(token) {
            score += 3;
        }
    }
    score
}

fn as_array_mut<'a>(root: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    if !root.get(key).map(Value::is_array).unwrap_or(false) {
        root[key] = Value::Array(Vec::new());
    }
    root.get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("array shape")
}

fn as_object_mut<'a>(root: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !root.get(key).map(Value::is_object).unwrap_or(false) {
        root[key] = json!({});
    }
    root.get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object shape")
}

fn duality_memory_tags(root: &Path, key: &str, value: &Value) -> Value {
    let payload = json!({
        "key": key,
        "value": value
    });
    match crate::duality_seed::invoke(root, "duality_memory_tag", Some(&payload)) {
        Ok(out) => out
            .get("nodes")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("duality_tags"))
            .cloned()
            .unwrap_or_else(|| json!({})),
        Err(_) => json!({}),
    }
}

fn sessions_dir(root: &Path) -> PathBuf {
    root.join(AGENT_SESSIONS_DIR_REL)
}

fn session_path(root: &Path, agent_id: &str) -> PathBuf {
    sessions_dir(root).join(format!("{}.json", normalize_agent_id(agent_id)))
}

fn default_session_state(agent_id: &str) -> Value {
    let now = now_iso();
    json!({
        "type": "infring_dashboard_agent_session",
        "agent_id": agent_id,
        "active_session_id": "default",
        "sessions": [
            {
                "session_id": "default",
                "label": "Session",
                "created_at": now,
                "updated_at": now,
                "messages": []
            }
        ],
        "memory_kv": {}
    })
}

fn load_session_state(root: &Path, agent_id: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    let mut state =
        read_json_file(&session_path(root, &id)).unwrap_or_else(|| default_session_state(&id));
    if !state.is_object() {
        state = default_session_state(&id);
    }
    state["agent_id"] = Value::String(id);
    let _ = as_array_mut(&mut state, "sessions");
    let _ = as_object_mut(&mut state, "memory_kv");
    state
}

fn save_session_state(root: &Path, agent_id: &str, state: &Value) {
    let id = normalize_agent_id(agent_id);
    ensure_dir(&sessions_dir(root));
    write_json(&session_path(root, &id), state);
}

fn memory_duality_tags(state: &Value, key: &str) -> Value {
    state
        .get("memory_kv_meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get(key))
        .cloned()
        .unwrap_or_else(|| json!({}))
}
