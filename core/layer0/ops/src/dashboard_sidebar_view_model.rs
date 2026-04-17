// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};

const SIDEBAR_VIEW_MODEL_VERSION: &str = "sidebar_view_model_v1";

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

fn parse_ts_ms(raw: &str) -> Option<i64> {
    let text = clean_text(raw, 120);
    if text.is_empty() {
        return None;
    }
    if let Ok(num) = text.parse::<i64>() {
        if num.abs() < 1_000_000_000_000 {
            return Some(num.saturating_mul(1000));
        }
        return Some(num);
    }
    DateTime::parse_from_rfc3339(&text)
        .ok()
        .map(|value| value.with_timezone(&Utc).timestamp_millis())
}

fn value_ts_ms(value: Option<&Value>) -> Option<i64> {
    value.and_then(|row| {
        row.as_i64()
            .or_else(|| row.as_u64().map(|num| num as i64))
            .or_else(|| row.as_str().and_then(parse_ts_ms))
    })
}

fn normalize_agent_id(row: &Value) -> String {
    clean_text(
        row.get("id")
            .or_else(|| row.get("agent_id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    )
}

fn is_system_sidebar_row(row: &Value) -> bool {
    let id = normalize_agent_id(row).to_ascii_lowercase();
    if id == "system" {
        return true;
    }
    if row
        .get("is_system_thread")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return true;
    }
    clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 40)
        .eq_ignore_ascii_case("system")
}

fn sidebar_sort_ts_value(row: &Value) -> i64 {
    if let Some(ts) = value_ts_ms(row.get("sidebar_sort_ts")) {
        return ts.max(0);
    }
    let keys = [
        "last_active_at",
        "last_activity_at",
        "last_message_at",
        "last_seen_at",
        "updated_at",
        "created_at",
    ];
    keys.iter()
        .filter_map(|key| value_ts_ms(row.get(*key)))
        .max()
        .unwrap_or(0)
}

fn sidebar_topology_key_value(row: &Value) -> String {
    if let Some(raw) = row.get("sidebar_topology_key").and_then(Value::as_str) {
        let normalized = clean_text(raw, 180);
        if !normalized.is_empty() {
            return normalized;
        }
    }
    let tree_kind = clean_text(
        row.get("git_tree_kind")
            .and_then(Value::as_str)
            .unwrap_or(""),
        60,
    )
    .to_ascii_lowercase();
    let branch = clean_text(
        row.get("git_branch")
            .or_else(|| row.get("branch"))
            .or_else(|| row.get("git_tree"))
            .or_else(|| row.get("tree"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    )
    .to_ascii_lowercase();
    let root =
        tree_kind == "main" || tree_kind == "master" || branch == "main" || branch == "master";
    let depth_raw = row
        .get("topology_depth")
        .or_else(|| row.get("depth"))
        .and_then(Value::as_i64)
        .unwrap_or(if root { 0 } else { 1 });
    let depth = depth_raw.max(0);
    let depth_key = format!("{depth:04}");
    let branch_key = if !branch.is_empty() {
        branch
    } else {
        clean_text(
            row.get("parent_agent_id")
                .or_else(|| row.get("id"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        )
        .to_ascii_lowercase()
    };
    format!(
        "{}|{}|{}",
        if root { "0" } else { "1" },
        depth_key,
        branch_key
    )
}

fn sidebar_status_state_value(row: &Value) -> String {
    if row
        .get("archived")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return "offline".to_string();
    }
    let state =
        clean_text(row.get("state").and_then(Value::as_str).unwrap_or(""), 80).to_ascii_lowercase();
    let offline_hints = [
        "offline",
        "inactive",
        "archived",
        "archive",
        "terminated",
        "timed out",
        "timeout",
        "stopped",
        "crashed",
        "error",
        "failed",
        "dead",
        "disabled",
    ];
    if offline_hints.iter().any(|hint| state.contains(hint)) {
        return "offline".to_string();
    }
    let ts = sidebar_sort_ts_value(row);
    if ts > 0 {
        let age_minutes = ((Utc::now().timestamp_millis() - ts) as f64 / 60_000.0).max(0.0);
        if age_minutes <= 10.0 {
            return "active".to_string();
        }
        if age_minutes <= 90.0 {
            return "idle".to_string();
        }
    }
    let active_hints = ["running", "active", "connected", "online"];
    if active_hints.iter().any(|hint| state.contains(hint))
        || state.contains("idle")
        || state.contains("paused")
        || state.contains("suspend")
    {
        return "idle".to_string();
    }
    "offline".to_string()
}

fn is_archived_sidebar_row(row: &Value) -> bool {
    if row
        .get("archived")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return true;
    }
    let state =
        clean_text(row.get("state").and_then(Value::as_str).unwrap_or(""), 80).to_ascii_lowercase();
    if state.contains("archived") || state.contains("inactive") || state.contains("terminated") {
        return true;
    }
    let contract_status = clean_text(
        row.get("contract")
            .and_then(Value::as_object)
            .and_then(|contract| contract.get("status"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    contract_status.contains("archived")
        || contract_status.contains("inactive")
        || contract_status.contains("terminated")
}

pub fn augment_agent_roster_rows(rows: Vec<Value>) -> Vec<Value> {
    let mut out = rows
        .into_iter()
        .map(|mut row| {
            let sort_ts = sidebar_sort_ts_value(&row);
            let topology_key = sidebar_topology_key_value(&row);
            let status_state = sidebar_status_state_value(&row);
            let status_label = status_state.clone();
            let sidebar_archived = is_archived_sidebar_row(&row);
            if let Some(map) = row.as_object_mut() {
                map.insert("sidebar_sort_ts".to_string(), Value::from(sort_ts));
                map.insert(
                    "sidebar_topology_key".to_string(),
                    Value::String(topology_key),
                );
                map.insert(
                    "sidebar_status_state".to_string(),
                    Value::String(status_state),
                );
                map.insert(
                    "sidebar_status_label".to_string(),
                    Value::String(status_label),
                );
                map.insert(
                    "sidebar_archived".to_string(),
                    Value::Bool(sidebar_archived),
                );
            }
            row
        })
        .collect::<Vec<_>>();
    if !out.iter().any(is_system_sidebar_row) {
        out.push(json!({"id":"system","name":"System","is_system_thread":true,"role":"system","state":"running","model_provider":"system","model_name":"terminal","identity":{"emoji":"⚙️"},"sidebar_sort_ts":0,"sidebar_topology_key":"z|system","sidebar_status_state":"active","sidebar_status_label":"active","sidebar_archived":false}));
    }
    out
}

fn quick_action_row(id: &str, name: &str, preview: &str, action: Value, emoji: &str) -> Value {
    let clean_id = clean_text(id, 80);
    json!({
        "id": format!("_sidebar_action:{clean_id}"),
        "name": clean_text(name, 120),
        "state": "ready",
        "avatar_url": "",
        "identity": {"emoji": clean_text(emoji, 8)},
        "_sidebar_search_result": true,
        "_sidebar_preview_text": clean_text(preview, 200),
        "_sidebar_quick_action": action,
        "sidebar_sort_ts": 0,
        "sidebar_topology_key": format!("z|quick:{clean_id}"),
        "sidebar_status_state": "idle",
        "sidebar_status_label": "idle",
        "sidebar_archived": false
    })
}

fn build_sidebar_quick_actions(query: &str, has_hits: bool) -> Vec<Value> {
    let normalized = clean_text(query, 260).to_ascii_lowercase();
    if normalized.is_empty() {
        return Vec::new();
    }
    let wants_connect = [
        "connect", "pair", "token", "auth", "secure", "identity", "gateway",
    ]
    .iter()
    .any(|token| normalized.contains(token));
    let wants_agents = ["agent", "session", "chat", "thread", "roster"]
        .iter()
        .any(|token| normalized.contains(token));
    let wants_settings = [
        "model", "setting", "config", "token", "auth", "provider", "key",
    ]
    .iter()
    .any(|token| normalized.contains(token));
    let mut out = Vec::<Value>::new();
    if wants_settings {
        out.push(quick_action_row(
            "settings",
            "Open Settings",
            "Jump to models, keys, and gateway configuration.",
            json!({"type": "navigate", "page": "settings"}),
            "⚙️",
        ));
    }
    if wants_agents || !has_hits {
        out.push(quick_action_row(
            "agents",
            "Open Agents",
            "Jump to the agent roster and templates.",
            json!({"type": "navigate", "page": "agents"}),
            "🤖",
        ));
    }
    if wants_connect {
        out.push(quick_action_row(
            "connect",
            "Copy connect checklist",
            "Copy pairing/auth recovery guidance.",
            json!({"type": "copy_connect", "page": "settings"}),
            "🔐",
        ));
    }
    if out.is_empty() {
        out.push(quick_action_row(
            "chat",
            "Open Chat",
            "Return to the active chat workspace.",
            json!({"type": "navigate", "page": "chat"}),
            "💬",
        ));
        out.push(quick_action_row(
            "settings",
            "Open Settings",
            "Check models, keys, and runtime configuration.",
            json!({"type": "navigate", "page": "settings"}),
            "⚙️",
        ));
        out.push(quick_action_row(
            "agents",
            "Open Agents",
            "Jump to the agent roster and templates.",
            json!({"type": "navigate", "page": "agents"}),
            "🤖",
        ));
    }
    out.truncate(3);
    out
}

fn map_search_result_to_sidebar_row(row: &Value) -> Option<Value> {
    if is_system_sidebar_row(row) {
        return None;
    }
    let id = normalize_agent_id(row);
    if id.is_empty() {
        return None;
    }
    let archived = row
        .get("archived")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let snippet = clean_text(
        row.get("snippet").and_then(Value::as_str).unwrap_or(""),
        260,
    );
    Some(json!({
        "id": id,
        "name": clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 140),
        "state": clean_text(row.get("state").and_then(Value::as_str).unwrap_or(if archived { "archived" } else { "running" }), 40),
        "archived": archived,
        "avatar_url": clean_text(row.get("avatar_url").and_then(Value::as_str).unwrap_or(""), 500),
        "identity": {"emoji": clean_text(row.get("emoji").and_then(Value::as_str).unwrap_or(""), 16)},
        "updated_at": clean_text(row.get("updated_at").and_then(Value::as_str).unwrap_or(""), 80),
        "_sidebar_search_result": true,
        "_sidebar_search_score": row.get("score").cloned().unwrap_or_else(|| json!(0)),
        "_sidebar_preview_text": if snippet.is_empty() { "No matching text".to_string() } else { snippet }
    }))
}

pub fn augment_conversation_search_payload(payload: Value, query: &str) -> Value {
    let mut out = payload;
    let results = out
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mapped = results
        .iter()
        .filter_map(map_search_result_to_sidebar_row)
        .collect::<Vec<_>>();
    let mut mapped = augment_agent_roster_rows(mapped);
    mapped.retain(|row| !is_system_sidebar_row(row));
    let quick_actions = build_sidebar_quick_actions(query, !mapped.is_empty());
    let mut sidebar_rows = mapped.clone();
    sidebar_rows.extend(quick_actions.clone());
    if let Some(map) = out.as_object_mut() {
        map.insert(
            "sidebar_view_model_version".to_string(),
            Value::String(SIDEBAR_VIEW_MODEL_VERSION.to_string()),
        );
        map.insert("sidebar_rows".to_string(), Value::Array(sidebar_rows));
        map.insert("quick_actions".to_string(), Value::Array(quick_actions));
    } else {
        let mut map = Map::<String, Value>::new();
        map.insert("ok".to_string(), Value::Bool(true));
        map.insert(
            "type".to_string(),
            Value::String("dashboard_conversation_search".to_string()),
        );
        map.insert(
            "sidebar_view_model_version".to_string(),
            Value::String(SIDEBAR_VIEW_MODEL_VERSION.to_string()),
        );
        map.insert("sidebar_rows".to_string(), Value::Array(sidebar_rows));
        map.insert("quick_actions".to_string(), Value::Array(quick_actions));
        out = Value::Object(map);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn augment_agent_roster_rows_emits_sidebar_fields() {
        let rows = vec![
            json!({
                "id": "agent-1",
                "state": "Running",
                "updated_at": "2026-04-16T03:20:00Z",
                "git_tree_kind": "main",
                "git_branch": "main"
            }),
            json!({
                "id": "agent-terminated",
                "state": "running",
                "contract": {"status": "terminated"}
            }),
        ];
        let out = augment_agent_roster_rows(rows);
        let row = out
            .iter()
            .find(|row| row.get("id").and_then(Value::as_str) == Some("agent-1"))
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(row["sidebar_topology_key"].as_str(), Some("0|0000|main"));
        assert_eq!(row["sidebar_status_state"].as_str(), Some("idle"));
        assert!(row["sidebar_sort_ts"].as_i64().unwrap_or(0) > 0);
        assert_eq!(row["sidebar_archived"].as_bool(), Some(false));
    }

    #[test]
    fn augment_agent_roster_rows_appends_system_thread() {
        let rows = vec![json!({
            "id": "agent-1",
            "state": "running",
            "updated_at": "2026-04-16T03:20:00Z"
        })];
        let out = augment_agent_roster_rows(rows);
        assert!(out
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some("system")));
    }

    #[test]
    fn augment_search_payload_includes_sidebar_rows() {
        let payload = json!({
            "ok": true,
            "results": [{
                "agent_id": "agent-2",
                "name": "Builder",
                "snippet": "repair web tooling fallback",
                "score": 91,
                "updated_at": "2026-04-16T03:20:00Z"
            }]
        });
        let out = augment_conversation_search_payload(payload, "web tooling");
        let rows = out["sidebar_rows"].as_array().cloned().unwrap_or_default();
        assert!(!rows.is_empty());
        assert_eq!(rows[0]["id"].as_str(), Some("agent-2"));
        assert!(rows[0]["sidebar_sort_ts"].as_i64().unwrap_or(0) > 0);
        assert!(rows.iter().all(|row| row["id"].as_str() != Some("system")));
    }

    #[test]
    fn quick_actions_present_when_query_has_no_hits() {
        let out =
            augment_conversation_search_payload(json!({"ok": true, "results": []}), "settings");
        let quick = out["quick_actions"].as_array().cloned().unwrap_or_default();
        let sidebar_rows = out["sidebar_rows"].as_array().cloned().unwrap_or_default();
        assert!(!quick.is_empty());
        assert!(sidebar_rows
            .iter()
            .all(|row| row["id"].as_str() != Some("system")));
        assert_eq!(
            quick[0]["id"]
                .as_str()
                .map(|id| id.starts_with("_sidebar_action:")),
            Some(true)
        );
        assert_eq!(quick[0]["sidebar_status_state"].as_str(), Some("idle"));
        assert_eq!(quick[0]["sidebar_archived"].as_bool(), Some(false));
    }
}
