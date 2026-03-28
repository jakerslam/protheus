// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const AGENT_SESSIONS_DIR_REL: &str = "client/runtime/local/state/ui/infring_dashboard/agent_sessions";
const MAX_MESSAGES: usize = 400;

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
    state["agent_id"] = Value::String(id.clone());
    if !state
        .get("active_session_id")
        .and_then(Value::as_str)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        state["active_session_id"] = Value::String("default".to_string());
    }
    let sessions = as_array_mut(&mut state, "sessions");
    if sessions.is_empty() {
        sessions.push(json!({
            "session_id": "default",
            "label": "Session",
            "created_at": now_iso(),
            "updated_at": now_iso(),
            "messages": []
        }));
    }
    let _ = as_object_mut(&mut state, "memory_kv");
    state
}

fn save_session_state(root: &Path, agent_id: &str, state: &Value) {
    let id = normalize_agent_id(agent_id);
    ensure_dir(&sessions_dir(root));
    write_json(&session_path(root, &id), state);
}

fn text_from_message(row: &Value) -> String {
    if let Some(text) = row.get("text").and_then(Value::as_str) {
        return clean_text(text, 400);
    }
    if let Some(text) = row.get("content").and_then(Value::as_str) {
        return clean_text(text, 400);
    }
    if let Some(text) = row.as_str() {
        return clean_text(text, 400);
    }
    String::new()
}

fn token_set(value: &str) -> HashSet<String> {
    clean_text(value, 300)
        .to_ascii_lowercase()
        .split(' ')
        .filter(|row| !row.is_empty())
        .map(ToString::to_string)
        .collect::<HashSet<_>>()
}

fn is_too_similar(left: &str, right: &str) -> bool {
    let a = token_set(left);
    let b = token_set(right);
    if a.is_empty() || b.is_empty() {
        return clean_text(left, 240).eq_ignore_ascii_case(&clean_text(right, 240));
    }
    let overlap = a.intersection(&b).count() as f64;
    let union = a.union(&b).count() as f64;
    if union <= 0.0 {
        return false;
    }
    (overlap / union) >= 0.8
}

fn sanitize_suggestion(value: &str) -> String {
    let cleaned = clean_text(value, 160).replace('"', "").replace('\'', "");
    if cleaned.is_empty() {
        return String::new();
    }
    let mut words = cleaned
        .split(' ')
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if words.len() > 12 {
        words.truncate(12);
    }
    words.join(" ")
}

pub fn append_turn(root: &Path, agent_id: &str, user_text: &str, assistant_text: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let message_count;
    {
        let sessions = as_array_mut(&mut state, "sessions");
        let mut target_idx = sessions
            .iter()
            .position(|row| {
                row.get("session_id")
                    .and_then(Value::as_str)
                    .map(|v| v == active_id)
                    .unwrap_or(false)
            })
            .unwrap_or(0);
        if sessions.is_empty() {
            sessions.push(json!({
                "session_id": "default",
                "label": "Session",
                "created_at": now_iso(),
                "updated_at": now_iso(),
                "messages": []
            }));
            target_idx = 0;
        }
        let session = &mut sessions[target_idx];
        if !session.get("messages").map(Value::is_array).unwrap_or(false) {
            session["messages"] = Value::Array(Vec::new());
        }
        let messages = session
            .get_mut("messages")
            .and_then(Value::as_array_mut)
            .expect("messages");
        let user = clean_text(user_text, 2000);
        let assistant = clean_text(assistant_text, 4000);
        if !user.is_empty() {
            messages.push(json!({"role": "user", "text": user, "ts": now_iso()}));
        }
        if !assistant.is_empty() {
            messages.push(json!({"role": "assistant", "text": assistant, "ts": now_iso()}));
        }
        if messages.len() > MAX_MESSAGES {
            let drain = messages.len() - MAX_MESSAGES;
            messages.drain(0..drain);
        }
        message_count = messages.len();
        session["updated_at"] = Value::String(now_iso());
    }
    save_session_state(root, &id, &state);
    json!({"ok": true, "type": "dashboard_agent_turn_append", "agent_id": id, "message_count": message_count})
}

pub fn load_session(root: &Path, agent_id: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let state = load_session_state(root, &id);
    json!({"ok": true, "type": "dashboard_agent_session", "agent_id": id, "session": state})
}

pub fn suggestions(root: &Path, agent_id: &str, user_hint: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required", "suggestions": []});
    }
    let state = load_session_state(root, &id);
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let sessions = state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let active = sessions
        .iter()
        .find(|row| {
            row.get("session_id")
                .and_then(Value::as_str)
                .map(|v| v == active_id)
                .unwrap_or(false)
        })
        .cloned()
        .unwrap_or_else(|| json!({"messages": []}));
    let messages = active
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let recent_user = messages
        .iter()
        .rev()
        .filter(|row| {
            row.get("role")
                .and_then(Value::as_str)
                .map(|v| v == "user")
                .unwrap_or(false)
        })
        .map(text_from_message)
        .filter(|row| !row.is_empty())
        .take(8)
        .collect::<Vec<_>>();

    if recent_user.is_empty() {
        return json!({"ok": true, "type": "dashboard_agent_suggestions", "agent_id": id, "suggestions": []});
    }

    let mut candidates = Vec::<String>::new();
    let hint = sanitize_suggestion(user_hint);
    if !hint.is_empty() {
        candidates.push(hint);
    }
    let last = recent_user.first().cloned().unwrap_or_default();
    if last.to_ascii_lowercase().contains("queue") {
        candidates.push("Can you reduce queue depth and report exact changes?".to_string());
    }
    candidates.push("What changed since the last runtime update?".to_string());
    candidates.push("Give me the highest ROI next action now.".to_string());
    candidates.push("Run the safest fix and report receipts.".to_string());

    let recent_set = recent_user
        .iter()
        .map(|row| sanitize_suggestion(row).to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut out = Vec::<String>::new();
    for raw in candidates {
        let row = sanitize_suggestion(&raw);
        if row.is_empty() {
            continue;
        }
        let row_lc = row.to_ascii_lowercase();
        if recent_set.contains(&row_lc) {
            continue;
        }
        if out.iter().any(|existing| is_too_similar(existing, &row)) {
            continue;
        }
        out.push(row);
        if out.len() >= 3 {
            break;
        }
    }

    json!({"ok": true, "type": "dashboard_agent_suggestions", "agent_id": id, "suggestions": out})
}

pub fn session_summaries(root: &Path, limit: usize) -> Value {
    let mut rows = Vec::<Value>::new();
    let dir = sessions_dir(root);
    if let Ok(read_dir) = fs::read_dir(&dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|v| v.to_str()) != Some("json") {
                continue;
            }
            if let Some(state) = read_json_file(&path) {
                let agent_id =
                    clean_text(state.get("agent_id").and_then(Value::as_str).unwrap_or(""), 140);
                let active = clean_text(
                    state
                        .get("active_session_id")
                        .and_then(Value::as_str)
                        .unwrap_or("default"),
                    120,
                );
                let sessions = state
                    .get("sessions")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let current = sessions
                    .iter()
                    .find(|row| {
                        row.get("session_id")
                            .and_then(Value::as_str)
                            .map(|v| v == active)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .unwrap_or_else(|| json!({"messages": []}));
                let messages = current
                    .get("messages")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let updated_at = clean_text(
                    current.get("updated_at").and_then(Value::as_str).unwrap_or(""),
                    80,
                );
                rows.push(json!({
                    "agent_id": agent_id,
                    "active_session_id": active,
                    "message_count": messages.len(),
                    "updated_at": updated_at
                }));
            }
        }
    }
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows.truncate(limit.clamp(1, 500));
    json!({"type": "dashboard_agent_session_summaries", "rows": rows})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestions_are_deduped_and_never_quoted() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = append_turn(
            root.path(),
            "agent-a",
            "Can you reduce queue depth before spikes?",
            "On it.",
        );
        let value = suggestions(
            root.path(),
            "agent-a",
            "\"Can you reduce queue depth before spikes?\"",
        );
        let rows = value
            .get("suggestions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.len() <= 3);
        for row in rows {
            let text = row.as_str().unwrap_or("");
            assert!(!text.contains('"'));
            assert!(!text.contains('\''));
        }
    }
}
