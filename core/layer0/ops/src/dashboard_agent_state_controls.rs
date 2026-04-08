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

pub fn create_session(root: &Path, agent_id: &str, label: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let base_label = clean_text(label, 80);
    let label_value = if base_label.is_empty() {
        "Session".to_string()
    } else {
        base_label
    };
    let mut session_id = format!(
        "s-{}",
        crate::deterministic_receipt_hash(
            &json!({"agent_id": id, "label": label_value, "ts": now_iso()})
        )
        .chars()
        .take(12)
        .collect::<String>()
    );
    {
        let sessions = as_array_mut(&mut state, "sessions");
        let mut attempt = 2usize;
        while sessions.iter().any(|row| {
            row.get("session_id")
                .and_then(Value::as_str)
                .map(|v| v == session_id)
                .unwrap_or(false)
        }) {
            session_id = format!("{}-{}", session_id, attempt);
            attempt += 1;
        }
        sessions.push(json!({
            "session_id": session_id,
            "label": label_value,
            "created_at": now_iso(),
            "updated_at": now_iso(),
            "messages": []
        }));
    }
    state["active_session_id"] = Value::String(session_id.clone());
    save_session_state(root, &id, &state);
    json!({"ok": true, "type": "dashboard_agent_session_create", "agent_id": id, "active_session_id": session_id, "session": state})
}

pub fn switch_session(root: &Path, agent_id: &str, session_id: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    let sid = clean_text(session_id, 120);
    if id.is_empty() || sid.is_empty() {
        return json!({"ok": false, "error": "agent_id_and_session_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let exists = state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("session_id")
                    .and_then(Value::as_str)
                    .map(|v| v == sid)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    if !exists {
        return json!({"ok": false, "error": "session_not_found", "agent_id": id, "session_id": sid});
    }
    state["active_session_id"] = Value::String(sid.clone());
    save_session_state(root, &id, &state);
    json!({"ok": true, "type": "dashboard_agent_session_switch", "agent_id": id, "active_session_id": sid})
}

pub fn delete_session(root: &Path, agent_id: &str, session_id: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    let sid = clean_text(session_id, 120);
    if id.is_empty() || sid.is_empty() {
        return json!({"ok": false, "error": "agent_id_and_session_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let removed = {
        let sessions = as_array_mut(&mut state, "sessions");
        let before = sessions.len();
        sessions.retain(|row| {
            row.get("session_id")
                .and_then(Value::as_str)
                .map(|v| v != sid)
                .unwrap_or(true)
        });
        let removed = sessions.len() != before;
        if sessions.is_empty() {
            sessions.push(json!({
                "session_id": "default",
                "label": "Session",
                "created_at": now_iso(),
                "updated_at": now_iso(),
                "messages": []
            }));
        }
        removed
    };
    let next_active = state
        .get("sessions")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("session_id").and_then(Value::as_str))
        .unwrap_or("default")
        .to_string();
    if state
        .get("active_session_id")
        .and_then(Value::as_str)
        .map(|v| v == sid)
        .unwrap_or(false)
    {
        state["active_session_id"] = Value::String(next_active.clone());
    }
    save_session_state(root, &id, &state);
    json!({"ok": true, "type": "dashboard_agent_session_delete", "agent_id": id, "removed": removed, "active_session_id": next_active})
}

pub fn memory_kv_set(root: &Path, agent_id: &str, key: &str, value: &Value) -> Value {
    let id = normalize_agent_id(agent_id);
    let k = normalize_memory_key(key);
    if id.is_empty() || k.is_empty() {
        return json!({"ok": false, "error": "agent_id_and_key_required"});
    }
    let mut state = load_session_state(root, &id);
    let memory = as_object_mut(&mut state, "memory_kv");
    memory.insert(k.clone(), value.clone());
    let tags = duality_memory_tags(root, &k, value);
    let memory_meta = as_object_mut(&mut state, "memory_kv_meta");
    memory_meta.insert(k.clone(), tags.clone());
    save_session_state(root, &id, &state);
    json!({
        "ok": true,
        "type": "dashboard_agent_memory_kv_set",
        "agent_id": id,
        "key": k,
        "value": value.clone(),
        "duality_tags": tags
    })
}

pub fn memory_kv_pairs(root: &Path, agent_id: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let state = load_session_state(root, &id);
    let mut kv_pairs = state
        .get("memory_kv")
        .and_then(Value::as_object)
        .map(|rows| {
            rows.iter()
                .map(|(key, value)| {
                    let duality_tags = state
                        .get("memory_kv_meta")
                        .and_then(Value::as_object)
                        .and_then(|meta| meta.get(key))
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    json!({
                        "key": key,
                        "value": value,
                        "duality_tags": duality_tags
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    kv_pairs
        .sort_by_key(|row| clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 160));
    json!({
        "ok": true,
        "type": "dashboard_agent_memory_kv_pairs",
        "agent_id": id,
        "kv_pairs": kv_pairs
    })
}

pub fn memory_kv_get(root: &Path, agent_id: &str, key: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    let k = normalize_memory_key(key);
    if id.is_empty() || k.is_empty() {
        return json!({"ok": false, "error": "agent_id_and_key_required"});
    }
    let state = load_session_state(root, &id);
    let value = state
        .get("memory_kv")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&k))
        .cloned()
        .unwrap_or(Value::Null);
    let duality_tags = state
        .get("memory_kv_meta")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&k))
        .cloned()
        .unwrap_or_else(|| json!({}));
    json!({
        "ok": true,
        "type": "dashboard_agent_memory_kv_get",
        "agent_id": id,
        "key": k,
        "value": value,
        "duality_tags": duality_tags
    })
}

pub fn memory_kv_delete(root: &Path, agent_id: &str, key: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    let k = normalize_memory_key(key);
    if id.is_empty() || k.is_empty() {
        return json!({"ok": false, "error": "agent_id_and_key_required"});
    }
    let mut state = load_session_state(root, &id);
    let memory = as_object_mut(&mut state, "memory_kv");
    let removed = memory.remove(&k).is_some();
    let memory_meta = as_object_mut(&mut state, "memory_kv_meta");
    let _ = memory_meta.remove(&k);
    save_session_state(root, &id, &state);
    json!({"ok": true, "type": "dashboard_agent_memory_kv_delete", "agent_id": id, "key": k, "removed": removed})
}

pub fn memory_kv_semantic_query(root: &Path, agent_id: &str, query: &str, limit: usize) -> Value {
    let id = normalize_agent_id(agent_id);
    let cleaned_query = clean_text(query, 600);
    if id.is_empty() || cleaned_query.is_empty() {
        return json!({"ok": false, "error": "agent_id_and_query_required"});
    }
    let state = load_session_state(root, &id);
    let query_terms = query_tokens(&cleaned_query, 10);
    let mut matches = state
        .get("memory_kv")
        .and_then(Value::as_object)
        .map(|rows| {
            rows.iter()
                .filter_map(|(key, value)| {
                    let value_text = value_search_text(value, 2000);
                    let score = memory_semantic_score(key, &value_text, &query_terms);
                    if score <= 0 {
                        return None;
                    }
                    let duality_tags = state
                        .get("memory_kv_meta")
                        .and_then(Value::as_object)
                        .and_then(|meta| meta.get(key))
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    Some(json!({
                        "key": clean_text(key, 200),
                        "value": value,
                        "score": score,
                        "snippet": clean_text(&value_text, 220),
                        "duality_tags": duality_tags
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    matches.sort_by(|a, b| {
        b.get("score")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .cmp(&a.get("score").and_then(Value::as_i64).unwrap_or(0))
            .then_with(|| {
                clean_text(a.get("key").and_then(Value::as_str).unwrap_or(""), 200).cmp(
                    &clean_text(b.get("key").and_then(Value::as_str).unwrap_or(""), 200),
                )
            })
    });
    matches.truncate(limit.clamp(1, 25));
    json!({
        "ok": true,
        "type": "dashboard_agent_memory_semantic_query",
        "agent_id": id,
        "query": cleaned_query,
        "matches": matches
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_controls_create_switch_delete() {
        let root = tempfile::tempdir().expect("tempdir");
        let created = create_session(root.path(), "agent-z", "Ops");
        assert_eq!(created.get("ok").and_then(Value::as_bool), Some(true));
        let sid = created
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(sid.starts_with("s-"));
        let switched = switch_session(root.path(), "agent-z", &sid);
        assert_eq!(switched.get("ok").and_then(Value::as_bool), Some(true));
        let deleted = delete_session(root.path(), "agent-z", &sid);
        assert_eq!(deleted.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn memory_kv_controls_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let set = memory_kv_set(root.path(), "agent-z", "focus.topic", &json!("reliability"));
        assert_eq!(set.get("ok").and_then(Value::as_bool), Some(true));
        let pairs = memory_kv_pairs(root.path(), "agent-z");
        assert_eq!(
            pairs
                .get("kv_pairs")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        let got = memory_kv_get(root.path(), "agent-z", "focus.topic");
        assert_eq!(
            got.get("value").and_then(Value::as_str),
            Some("reliability")
        );
        let deleted = memory_kv_delete(root.path(), "agent-z", "focus.topic");
        assert_eq!(deleted.get("removed").and_then(Value::as_bool), Some(true));
        let missing = memory_kv_get(root.path(), "agent-z", "focus.topic");
        assert!(missing.get("value").map(Value::is_null).unwrap_or(false));
    }

    #[test]
    fn memory_semantic_query_returns_ranked_matches() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = memory_kv_set(
            root.path(),
            "agent-q",
            "fact.auth.flow",
            &json!("OAuth callback uses PKCE and nonce binding"),
        );
        let _ = memory_kv_set(
            root.path(),
            "agent-q",
            "fact.release.notes",
            &json!("Dashboard blur transition was tuned for resize"),
        );
        let out = memory_kv_semantic_query(root.path(), "agent-q", "auth pkce", 5);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let rows = out
            .get("matches")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!rows.is_empty());
        let first_key = rows
            .first()
            .and_then(|row| row.get("key").and_then(Value::as_str))
            .unwrap_or("");
        assert_eq!(first_key, "fact.auth.flow");
    }
}
