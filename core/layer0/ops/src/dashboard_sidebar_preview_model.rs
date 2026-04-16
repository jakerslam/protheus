// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const AGENT_SESSIONS_DIR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_sessions";

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn parse_ts_ms(value: Option<&Value>) -> i64 {
    if let Some(num) = value.and_then(Value::as_i64) {
        return if num.abs() < 1_000_000_000_000 {
            num.saturating_mul(1000)
        } else {
            num
        };
    }
    if let Some(text) = value.and_then(Value::as_str) {
        let cleaned = clean_text(text, 120);
        if cleaned.is_empty() {
            return 0;
        }
        if let Ok(num) = cleaned.parse::<i64>() {
            return if num.abs() < 1_000_000_000_000 {
                num.saturating_mul(1000)
            } else {
                num
            };
        }
        if let Ok(parsed) = DateTime::parse_from_rfc3339(&cleaned) {
            return parsed.with_timezone(&Utc).timestamp_millis();
        }
    }
    0
}

fn read_json_file(path: &Path) -> Option<Value> {
    let body = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&body).ok()
}

fn sessions_dir(root: &Path) -> PathBuf {
    root.join(AGENT_SESSIONS_DIR_REL)
}

fn summarize_tools(tools: &[Value]) -> (bool, String, String) {
    if tools.is_empty() {
        return (false, String::new(), String::new());
    }
    let mut state = "success";
    for tool in tools {
        let status = clean_text(
            tool.get("status")
                .or_else(|| tool.get("result"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        let blocked = tool
            .get("blocked")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || status.contains("blocked")
            || status.contains("policy")
            || status.contains("denied")
            || status.contains("forbidden")
            || status.contains("not allowed")
            || status.contains("approval")
            || status.contains("permission")
            || status.contains("fail-closed");
        let running = tool
            .get("running")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || status.contains("running")
            || status.contains("in_progress");
        let is_error = tool
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || status.contains("error")
            || status.contains("failed");
        if is_error {
            state = "error";
            break;
        }
        if blocked || running {
            state = "warning";
        }
    }
    let label = if state == "error" {
        "Tool error"
    } else if state == "warning" {
        "Tool warning"
    } else {
        "Tool success"
    };
    (true, state.to_string(), label.to_string())
}

fn preview_from_messages(messages: &[Value]) -> Value {
    for row in messages.iter().rev() {
        let mut text = clean_text(
            row.get("text")
                .or_else(|| row.get("content"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            260,
        );
        let tools = row
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if text.is_empty() && !tools.is_empty() {
            let names = tools
                .iter()
                .map(|tool| {
                    clean_text(
                        tool.get("name").and_then(Value::as_str).unwrap_or("tool"),
                        60,
                    )
                })
                .filter(|name| !name.is_empty())
                .collect::<Vec<_>>();
            if !names.is_empty() {
                text = clean_text(&format!("[Processes] {}", names.join(", ")), 260);
            }
        }
        if text.is_empty() {
            continue;
        }
        let role = clean_text(
            row.get("role")
                .and_then(Value::as_str)
                .unwrap_or("assistant"),
            20,
        );
        let ts = parse_ts_ms(row.get("ts"));
        let (has_tools, tool_state, tool_label) = summarize_tools(&tools);
        return json!({
            "text": text,
            "ts": ts,
            "role": role,
            "has_tools": has_tools,
            "tool_state": tool_state,
            "tool_label": tool_label
        });
    }
    json!({
        "text": "",
        "ts": 0,
        "role": "assistant",
        "has_tools": false,
        "tool_state": "",
        "tool_label": ""
    })
}

fn load_sidebar_preview(root: &Path, agent_id: &str) -> Value {
    let id = clean_text(agent_id, 180);
    if id.is_empty() || id.eq_ignore_ascii_case("system") {
        return preview_from_messages(&[]);
    }
    let state_path = sessions_dir(root).join(format!("{id}.json"));
    let state = match read_json_file(&state_path) {
        Some(value) => value,
        None => return preview_from_messages(&[]),
    };
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
            clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            ) == active_id
        })
        .cloned()
        .unwrap_or_else(|| sessions.first().cloned().unwrap_or_else(|| json!({})));
    let messages = active
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    preview_from_messages(&messages)
}

fn search_snippet_preview(row: &Value) -> Option<Value> {
    let is_search = row
        .get("_sidebar_search_result")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !is_search {
        return None;
    }
    let snippet = clean_text(
        row.get("_sidebar_preview_text")
            .and_then(Value::as_str)
            .unwrap_or(""),
        260,
    );
    if snippet.is_empty() {
        return None;
    }
    let ts = parse_ts_ms(row.get("sidebar_sort_ts"));
    Some(json!({
        "text": snippet,
        "ts": ts,
        "role": "assistant",
        "has_tools": false,
        "tool_state": "",
        "tool_label": ""
    }))
}

pub fn augment_agent_roster_with_previews(root: &Path, rows: Vec<Value>) -> Vec<Value> {
    rows.into_iter()
        .map(|mut row| {
            let id = clean_text(
                row.get("id")
                    .or_else(|| row.get("agent_id"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                180,
            );
            if id.starts_with("_sidebar_action:") {
                return row;
            }
            let mut preview = load_sidebar_preview(root, &id);
            let preview_text = clean_text(
                preview.get("text").and_then(Value::as_str).unwrap_or(""),
                260,
            );
            if preview_text.is_empty() {
                if let Some(search_preview) = search_snippet_preview(&row) {
                    preview = search_preview;
                }
            }
            let preview_ts = parse_ts_ms(preview.get("ts"));
            if let Some(map) = row.as_object_mut() {
                map.insert("sidebar_preview".to_string(), preview);
                let sort_ts = map
                    .get("sidebar_sort_ts")
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                if preview_ts > sort_ts {
                    map.insert("sidebar_sort_ts".to_string(), Value::from(preview_ts));
                }
            }
            row
        })
        .collect()
}

pub fn augment_search_payload_with_previews(root: &Path, mut payload: Value) -> Value {
    let rows = payload
        .get("sidebar_rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if rows.is_empty() {
        return payload;
    }
    let augmented = augment_agent_roster_with_previews(root, rows);
    if let Some(map) = payload.as_object_mut() {
        map.insert("sidebar_rows".to_string(), Value::Array(augmented));
    }
    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_from_messages_tracks_tool_state() {
        let preview = preview_from_messages(&[json!({
            "role": "assistant",
            "text": "Ran web check",
            "ts": "2026-04-16T04:20:00Z",
            "tools": [{"name":"web_search","status":"policy_blocked"}]
        })]);
        assert_eq!(
            preview.get("has_tools").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            preview.get("tool_state").and_then(Value::as_str),
            Some("warning")
        );
        assert_eq!(
            preview.get("tool_label").and_then(Value::as_str),
            Some("Tool warning")
        );
    }

    #[test]
    fn augment_injects_sidebar_preview() {
        let root = tempfile::tempdir().expect("tempdir");
        let sessions_path = root.path().join(AGENT_SESSIONS_DIR_REL);
        let _ = fs::create_dir_all(&sessions_path);
        let _ = fs::write(
            sessions_path.join("agent-1.json"),
            serde_json::to_string(&json!({
                "active_session_id": "default",
                "sessions": [{
                    "session_id": "default",
                    "messages": [{
                        "role": "assistant",
                        "text": "hello world",
                        "ts": "2026-04-16T04:20:00Z"
                    }]
                }]
            }))
            .unwrap_or_default(),
        );
        let out = augment_agent_roster_with_previews(
            root.path(),
            vec![json!({"id":"agent-1", "sidebar_sort_ts": 0})],
        );
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0]
                .get("sidebar_preview")
                .and_then(|row| row.get("text"))
                .and_then(Value::as_str),
            Some("hello world")
        );
        assert!(
            out[0]
                .get("sidebar_sort_ts")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > 0
        );
    }

    #[test]
    fn augment_search_payload_injects_preview_rows() {
        let root = tempfile::tempdir().expect("tempdir");
        let sessions_path = root.path().join(AGENT_SESSIONS_DIR_REL);
        let _ = fs::create_dir_all(&sessions_path);
        let _ = fs::write(
            sessions_path.join("agent-2.json"),
            serde_json::to_string(&json!({
                "active_session_id": "default",
                "sessions": [{
                    "session_id": "default",
                    "messages": [{"role":"assistant","text":"preview hello","ts":"2026-04-16T04:20:00Z"}]
                }]
            }))
            .unwrap_or_default(),
        );
        let payload = json!({
            "ok": true,
            "sidebar_rows": [
                {"id":"agent-2","name":"Agent Two"},
                {"id":"_sidebar_action:settings","name":"Open Settings"}
            ]
        });
        let out = augment_search_payload_with_previews(root.path(), payload);
        let rows = out
            .get("sidebar_rows")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0]
                .get("sidebar_preview")
                .and_then(|row| row.get("text"))
                .and_then(Value::as_str),
            Some("preview hello")
        );
        assert!(rows[1].get("sidebar_preview").is_none());
    }

    #[test]
    fn search_rows_use_snippet_preview_when_sessions_missing() {
        let root = tempfile::tempdir().expect("tempdir");
        let payload = json!({
            "ok": true,
            "sidebar_rows": [
                {
                    "id":"agent-missing",
                    "name":"Missing Session Agent",
                    "sidebar_sort_ts": 1713270000,
                    "_sidebar_search_result": true,
                    "_sidebar_preview_text":"Match snippet from indexed search"
                }
            ]
        });
        let out = augment_search_payload_with_previews(root.path(), payload);
        let rows = out
            .get("sidebar_rows")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0]
                .get("sidebar_preview")
                .and_then(|row| row.get("text"))
                .and_then(Value::as_str),
            Some("Match snippet from indexed search")
        );
    }
}
