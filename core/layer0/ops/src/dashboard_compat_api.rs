// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const PROVIDER_REGISTRY_REL: &str = "client/runtime/local/state/ui/infring_dashboard/provider_registry.json";
const APPROVALS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/approvals.json";
const WORKFLOWS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/workflows.json";
const CRON_JOBS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/cron_jobs.json";
const TRIGGERS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/triggers.json";
const AGENT_PROFILES_REL: &str = "client/runtime/local/state/ui/infring_dashboard/agent_profiles.json";
const AGENT_CONTRACTS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/agent_contracts.json";
const AGENT_SESSIONS_DIR_REL: &str = "client/runtime/local/state/ui/infring_dashboard/agent_sessions";
const AGENT_FILES_DIR_REL: &str = "client/runtime/local/state/ui/infring_dashboard/agent_files";
const AGENT_TOOLS_DIR_REL: &str = "client/runtime/local/state/ui/infring_dashboard/agent_tools";
const ACTION_HISTORY_REL: &str = "client/runtime/local/state/ui/infring_dashboard/actions/history.jsonl";
const EYES_CATALOG_STATE_PATHS: [&str; 3] = [
    "client/runtime/local/state/ui/infring_dashboard/eyes_catalog.json",
    "client/runtime/local/state/eyes/catalog.json",
    "client/runtime/local/state/ui/eyes/catalog.json",
];

#[path = "dashboard_compat_api_channels.rs"]
mod dashboard_compat_api_channels;
#[path = "dashboard_skills_marketplace.rs"]
mod dashboard_skills_marketplace;

#[derive(Debug, Clone)]
pub struct CompatApiResponse {
    pub status: u16,
    pub payload: Value,
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

#[cfg(test)]
fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, raw);
    }
}

fn parse_non_negative_i64(value: Option<&Value>, fallback: i64) -> i64 {
    value
        .and_then(Value::as_i64)
        .unwrap_or(fallback)
        .max(0)
}

fn state_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

fn query_value(path: &str, key: &str) -> Option<String> {
    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if clean_text(k, 80).eq_ignore_ascii_case(key) {
            let decoded = urlencoding::decode(v)
                .ok()
                .map(|s| s.to_string())
                .unwrap_or_default();
            let value = clean_text(&decoded, 160);
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn extract_app_settings(snapshot: &Value) -> (String, String) {
    let provider = clean_text(
        snapshot
            .pointer("/app/settings/provider")
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        80,
    );
    let model = clean_text(
        snapshot
            .pointer("/app/settings/model")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    (provider, model)
}

fn runtime_sync_summary(snapshot: &Value) -> Value {
    if let Some(summary) = snapshot.pointer("/runtime_sync/summary") {
        return summary.clone();
    }
    json!({
        "queue_depth": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/queue_depth/value"), 0),
        "cockpit_blocks": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/hermes_cockpit_stream/value"), 0),
        "cockpit_total_blocks": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/hermes_cockpit_stream/value"), 0),
        "attention_batch_count": 0,
        "conduit_signals": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/collab_team_surface/value"), 0),
        "conduit_channels_observed": parse_non_negative_i64(snapshot.pointer("/health/dashboard_metrics/collab_team_surface/value"), 0),
        "target_conduit_signals": 4,
        "conduit_scale_required": false,
        "sync_mode": "live_sync",
        "backpressure_level": "normal"
    })
}

fn usage_from_snapshot(snapshot: &Value) -> Value {
    let turn_count = parse_non_negative_i64(snapshot.pointer("/app/turn_count"), 0);
    let (provider, model) = extract_app_settings(snapshot);
    let model_rows = if model.is_empty() {
        Vec::new()
    } else {
        vec![json!({
            "provider": provider,
            "model": model,
            "requests": turn_count,
            "input_tokens": 0,
            "output_tokens": 0,
            "cost_usd": 0.0
        })]
    };
    let today = crate::now_iso().chars().take(10).collect::<String>();
    let daily = vec![json!({
        "date": today,
        "requests": turn_count,
        "input_tokens": 0,
        "output_tokens": 0,
        "cost_usd": 0.0
    })];
    json!({
        "agents": {
            "active": snapshot
                .pointer("/collab/dashboard/agents")
                .and_then(Value::as_array)
                .map(|rows| rows.len() as i64)
                .unwrap_or(0),
            "archived": 0
        },
        "summary": {
            "requests": turn_count,
            "input_tokens": 0,
            "output_tokens": 0,
            "total_cost_usd": 0.0,
            "active_provider": provider,
            "active_model": model
        },
        "models": model_rows,
        "daily": daily
    })
}

fn providers_payload(root: &Path, snapshot: &Value) -> Value {
    let mut rows = Vec::<Value>::new();
    if let Some(registry) = read_json(&state_path(root, PROVIDER_REGISTRY_REL)) {
        if let Some(obj) = registry.get("providers").and_then(Value::as_object) {
            for row in obj.values() {
                rows.push(row.clone());
            }
        }
    }
    if rows.is_empty() {
        let (provider, model) = extract_app_settings(snapshot);
        rows.push(json!({
            "id": if provider.is_empty() { "auto" } else { provider.as_str() },
            "display_name": "Runtime Provider",
            "is_local": provider == "ollama" || provider == "local",
            "needs_key": provider != "ollama" && provider != "local",
            "auth_status": "unknown",
            "detected_models": if model.is_empty() { vec![] } else { vec![model] }
        }));
    }
    rows.sort_by(|a, b| {
        clean_text(a.get("id").and_then(Value::as_str).unwrap_or(""), 80)
            .cmp(&clean_text(b.get("id").and_then(Value::as_str).unwrap_or(""), 80))
    });
    json!({"ok": true, "providers": rows})
}

fn approvals_payload(root: &Path) -> Value {
    let rows = read_json(&state_path(root, APPROVALS_REL))
        .and_then(|v| v.get("approvals").cloned())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_else(|| {
            vec![json!({
                "id": "default-approval-policy",
                "name": "Default Approval Policy",
                "status": "active",
                "updated_at": crate::now_iso()
            })]
        });
    json!({"ok": true, "approvals": rows})
}

fn rows_from_array_store(root: &Path, rel: &str, key: &str) -> Value {
    let rows = read_json(&state_path(root, rel))
        .and_then(|v| {
            if v.is_array() {
                v.as_array().cloned()
            } else {
                v.get(key).and_then(Value::as_array).cloned()
            }
        })
        .unwrap_or_default();
    json!({"ok": true, key: rows})
}

fn read_eyes_payload(root: &Path) -> Value {
    for rel in EYES_CATALOG_STATE_PATHS {
        if let Some(value) = read_json(&state_path(root, rel)) {
            return json!({"ok": true, "type": "eyes_catalog", "catalog": value});
        }
    }
    json!({"ok": true, "type": "eyes_catalog", "catalog": {"eyes": []}})
}

fn extract_profiles(root: &Path) -> Vec<Value> {
    let state = read_json(&state_path(root, AGENT_PROFILES_REL)).unwrap_or_else(|| json!({}));
    let mut rows = state
        .get("agents")
        .and_then(Value::as_object)
        .map(|obj| {
            obj.values()
                .map(|v| v.clone())
                .collect::<Vec<Value>>()
        })
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        clean_text(a.get("agent_id").and_then(Value::as_str).unwrap_or(""), 120)
            .cmp(&clean_text(b.get("agent_id").and_then(Value::as_str).unwrap_or(""), 120))
    });
    rows
}

fn recent_audit_entries(root: &Path, snapshot: &Value) -> Vec<Value> {
    let from_snapshot = snapshot
        .pointer("/receipts/recent")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !from_snapshot.is_empty() {
        return from_snapshot;
    }
    let raw = fs::read_to_string(state_path(root, ACTION_HISTORY_REL)).unwrap_or_default();
    raw.lines()
        .rev()
        .take(200)
        .filter_map(|row| serde_json::from_str::<Value>(row).ok())
        .collect::<Vec<_>>()
}

fn clean_agent_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 140).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        }
    }
    out
}

fn parse_json_loose(raw: &str) -> Option<Value> {
    if raw.trim().is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        return Some(value);
    }
    for line in raw.lines().rev() {
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

fn read_json_loose(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    parse_json_loose(&raw)
}

fn write_json_pretty(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn parse_rfc3339_utc(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn profiles_map(root: &Path) -> Map<String, Value> {
    read_json_loose(&state_path(root, AGENT_PROFILES_REL))
        .and_then(|v| v.get("agents").and_then(Value::as_object).cloned())
        .unwrap_or_default()
}

fn contracts_map(root: &Path) -> Map<String, Value> {
    read_json_loose(&state_path(root, AGENT_CONTRACTS_REL))
        .and_then(|v| v.get("contracts").and_then(Value::as_object).cloned())
        .unwrap_or_default()
}

fn session_dir(root: &Path) -> PathBuf {
    state_path(root, AGENT_SESSIONS_DIR_REL)
}

fn session_path(root: &Path, agent_id: &str) -> PathBuf {
    session_dir(root).join(format!("{}.json", clean_agent_id(agent_id)))
}

fn agent_files_dir(root: &Path, agent_id: &str) -> PathBuf {
    state_path(root, AGENT_FILES_DIR_REL).join(clean_agent_id(agent_id))
}

fn agent_tools_path(root: &Path, agent_id: &str) -> PathBuf {
    state_path(root, AGENT_TOOLS_DIR_REL).join(format!("{}.json", clean_agent_id(agent_id)))
}

fn default_session_state(agent_id: &str) -> Value {
    let now = crate::now_iso();
    json!({
        "type": "infring_dashboard_agent_session",
        "agent_id": clean_agent_id(agent_id),
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

fn normalize_session_state(agent_id: &str, mut state: Value) -> Value {
    let id = clean_agent_id(agent_id);
    if !state.is_object() {
        state = default_session_state(&id);
    }
    state["agent_id"] = Value::String(id);
    if !state
        .get("active_session_id")
        .and_then(Value::as_str)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        state["active_session_id"] = Value::String("default".to_string());
    }
    if !state.get("sessions").map(Value::is_array).unwrap_or(false) {
        state["sessions"] = Value::Array(Vec::new());
    }
    if state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| rows.is_empty())
        .unwrap_or(true)
    {
        state["sessions"] = Value::Array(vec![json!({
            "session_id": "default",
            "label": "Session",
            "created_at": crate::now_iso(),
            "updated_at": crate::now_iso(),
            "messages": []
        })]);
    }
    if !state.get("memory_kv").map(Value::is_object).unwrap_or(false) {
        state["memory_kv"] = json!({});
    }
    state
}

fn load_session_state(root: &Path, agent_id: &str) -> Value {
    let path = session_path(root, agent_id);
    let state = read_json_loose(&path).unwrap_or_else(|| default_session_state(agent_id));
    normalize_session_state(agent_id, state)
}

fn save_session_state(root: &Path, agent_id: &str, state: &Value) {
    let path = session_path(root, agent_id);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    write_json_pretty(&path, state);
}

fn estimate_tokens(text: &str) -> i64 {
    ((clean_text(text, 20_000).chars().count() as i64) / 4).max(1)
}

fn active_session_row(state: &Value) -> Value {
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let rows = state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(found) = rows.iter().find(|row| {
        row.get("session_id")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 120) == active_id)
            .unwrap_or(false)
    }) {
        return found.clone();
    }
    rows.first().cloned().unwrap_or_else(|| json!({"messages": []}))
}

fn session_messages(state: &Value) -> Vec<Value> {
    active_session_row(state)
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn session_rows_payload(state: &Value) -> Vec<Value> {
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            let sid = clean_text(row.get("session_id").and_then(Value::as_str).unwrap_or(""), 120);
            let label = clean_text(row.get("label").and_then(Value::as_str).unwrap_or("Session"), 80);
            let updated_at = clean_text(
                row.get("updated_at")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            );
            let message_count = row
                .get("messages")
                .and_then(Value::as_array)
                .map(|rows| rows.len() as i64)
                .unwrap_or(0);
            json!({
                "id": sid,
                "session_id": sid,
                "label": if label.is_empty() { "Session" } else { &label },
                "updated_at": updated_at,
                "message_count": message_count,
                "active": sid == active_id
            })
        })
        .collect::<Vec<_>>()
}

fn split_model_ref(model_ref: &str, fallback_provider: &str, fallback_model: &str) -> (String, String) {
    let cleaned = clean_text(model_ref, 200);
    if cleaned.contains('/') {
        let mut parts = cleaned.splitn(2, '/');
        let provider = clean_text(parts.next().unwrap_or(""), 80);
        let model = clean_text(parts.next().unwrap_or(""), 120);
        if !provider.is_empty() && !model.is_empty() {
            return (provider, model);
        }
    }
    let provider = if fallback_provider.is_empty() {
        "auto".to_string()
    } else {
        clean_text(fallback_provider, 80)
    };
    let model = if cleaned.is_empty() {
        clean_text(fallback_model, 120)
    } else {
        cleaned
    };
    (provider, model)
}

fn parse_manifest_fields(manifest_toml: &str) -> HashMap<String, String> {
    let mut out = HashMap::<String, String>::new();
    let mut in_model = false;
    for line in manifest_toml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = trimmed.trim_matches(|ch| ch == '[' || ch == ']');
            in_model = section.eq_ignore_ascii_case("model");
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = clean_text(k, 80).to_ascii_lowercase();
            let mut value = v.trim().trim_matches('"').to_string();
            value = clean_text(&value, 400);
            if value.is_empty() {
                continue;
            }
            if key == "name" {
                out.insert("name".to_string(), value.clone());
            } else if key == "role" {
                out.insert("role".to_string(), value.clone());
            } else if in_model && key == "provider" {
                out.insert("provider".to_string(), value.clone());
            } else if in_model && key == "model" {
                out.insert("model".to_string(), value.clone());
            }
        }
    }
    out
}

fn make_agent_id(root: &Path, suggested_name: &str) -> String {
    let profiles = profiles_map(root);
    let contracts = contracts_map(root);
    let mut used = HashSet::<String>::new();
    for key in profiles.keys() {
        used.insert(clean_agent_id(key));
    }
    for key in contracts.keys() {
        used.insert(clean_agent_id(key));
    }
    let hint = clean_text(suggested_name, 80)
        .to_ascii_lowercase()
        .replace(' ', "-");
    let hash_seed = json!({"hint": hint, "ts": crate::now_iso(), "nonce": Utc::now().timestamp_nanos_opt().unwrap_or_default()});
    let hash = crate::deterministic_receipt_hash(&hash_seed);
    let mut base = format!("agent-{}", hash.chars().take(12).collect::<String>());
    if !hint.is_empty() && hint.len() <= 18 {
        base = format!("agent-{}-{}", hint, hash.chars().take(5).collect::<String>());
    }
    let mut candidate = clean_agent_id(&base);
    if candidate.is_empty() {
        candidate = format!("agent-{}", hash.chars().take(12).collect::<String>());
    }
    if !used.contains(&candidate) {
        return candidate;
    }
    for idx in 2..5000 {
        let next = format!("{candidate}-{idx}");
        if !used.contains(&next) {
            return next;
        }
    }
    format!("agent-{}", crate::deterministic_receipt_hash(&json!({"fallback": crate::now_iso()})).chars().take(14).collect::<String>())
}

fn contract_with_runtime_fields(contract: &Value) -> Value {
    let mut out = if contract.is_object() {
        contract.clone()
    } else {
        json!({})
    };
    let status = clean_text(
        out.get("status").and_then(Value::as_str).unwrap_or("active"),
        40,
    );
    let now = Utc::now();
    let created = out
        .get("created_at")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339_utc)
        .unwrap_or(now);
    let expiry_seconds = out
        .get("expiry_seconds")
        .and_then(Value::as_i64)
        .unwrap_or(3600)
        .clamp(1, 31 * 24 * 60 * 60);
    let expires = out
        .get("expires_at")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339_utc)
        .unwrap_or_else(|| created + chrono::Duration::seconds(expiry_seconds));
    if out
        .get("expires_at")
        .and_then(Value::as_str)
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
    {
        out["expires_at"] = Value::String(expires.to_rfc3339());
    }
    let mut remaining = (expires.timestamp_millis() - now.timestamp_millis()).max(0);
    if status.eq_ignore_ascii_case("terminated") {
        remaining = 0;
    }
    out["remaining_ms"] = Value::from(remaining);
    out
}

fn collab_agents_map(snapshot: &Value) -> HashMap<String, Value> {
    let mut out = HashMap::<String, Value>::new();
    let rows = snapshot
        .pointer("/collab/dashboard/agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in rows {
        let id = clean_agent_id(row.get("shadow").and_then(Value::as_str).unwrap_or(""));
        if id.is_empty() {
            continue;
        }
        out.insert(id, row);
    }
    out
}

fn first_string(value: Option<&Value>, key: &str) -> String {
    clean_text(
        value
            .and_then(|row| row.get(key).and_then(Value::as_str))
            .unwrap_or(""),
        240,
    )
}

fn build_agent_roster(root: &Path, snapshot: &Value, include_terminated: bool) -> Vec<Value> {
    let archived = crate::dashboard_agent_state::archived_agent_ids(root);
    let profiles = profiles_map(root);
    let contracts = contracts_map(root);
    let collab = collab_agents_map(snapshot);
    let (default_provider, default_model) = extract_app_settings(snapshot);
    let mut all_ids = HashSet::<String>::new();
    for key in profiles.keys() {
        let id = clean_agent_id(key);
        if !id.is_empty() {
            all_ids.insert(id);
        }
    }
    for key in contracts.keys() {
        let id = clean_agent_id(key);
        if !id.is_empty() {
            all_ids.insert(id);
        }
    }
    for key in collab.keys() {
        let id = clean_agent_id(key);
        if !id.is_empty() {
            all_ids.insert(id);
        }
    }
    let mut rows = Vec::<Value>::new();
    for agent_id in all_ids {
        if archived.contains(&agent_id) {
            continue;
        }
        let profile = profiles.get(&agent_id).cloned().unwrap_or_else(|| json!({}));
        let contract_raw = contracts.get(&agent_id).cloned().unwrap_or_else(|| json!({}));
        let contract = contract_with_runtime_fields(&contract_raw);
        let contract_status = clean_text(
            contract.get("status").and_then(Value::as_str).unwrap_or("active"),
            40,
        )
        .to_ascii_lowercase();
        if !include_terminated && contract_status == "terminated" {
            continue;
        }
        let collab_row = collab.get(&agent_id);
        let profile_name = clean_text(profile.get("name").and_then(Value::as_str).unwrap_or(""), 120);
        let name = if profile_name.is_empty() {
            agent_id.clone()
        } else {
            profile_name
        };
        let role = {
            let from_profile = clean_text(profile.get("role").and_then(Value::as_str).unwrap_or(""), 60);
            if !from_profile.is_empty() {
                from_profile
            } else {
                let from_collab = first_string(collab_row, "role");
                if !from_collab.is_empty() {
                    from_collab
                } else {
                    "analyst".to_string()
                }
            }
        };
        let state = if contract_status == "terminated" {
            "Terminated".to_string()
        } else {
            let raw = first_string(collab_row, "status");
            if raw.is_empty() {
                "Running".to_string()
            } else if raw.eq_ignore_ascii_case("active") || raw.eq_ignore_ascii_case("running") {
                "Running".to_string()
            } else if raw.eq_ignore_ascii_case("idle") {
                "Idle".to_string()
            } else {
                raw
            }
        };

        let identity = if profile.get("identity").map(Value::is_object).unwrap_or(false) {
            profile.get("identity").cloned().unwrap_or_else(|| json!({}))
        } else {
            json!({
                "emoji": profile.get("emoji").cloned().unwrap_or_else(|| json!("🧑‍💻")),
                "color": profile.get("color").cloned().unwrap_or_else(|| json!("#2563EB")),
                "archetype": profile.get("archetype").cloned().unwrap_or_else(|| json!("assistant")),
                "vibe": profile.get("vibe").cloned().unwrap_or_else(|| json!(""))
            })
        };
        let model_override = clean_text(
            profile
                .get("model_override")
                .and_then(Value::as_str)
                .unwrap_or(""),
            160,
        );
        let model_ref = if !model_override.is_empty() && !model_override.eq_ignore_ascii_case("auto") {
            model_override
        } else {
            default_model.clone()
        };
        let (model_provider, model_name) =
            split_model_ref(&model_ref, &default_provider, &default_model);
        let runtime_model = clean_text(
            profile
                .get("runtime_model")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let model_runtime = if runtime_model.is_empty() {
            model_name.clone()
        } else {
            runtime_model
        };
        let git_branch = clean_text(
            profile
                .get("git_branch")
                .and_then(Value::as_str)
                .unwrap_or("main"),
            180,
        );
        let git_tree_kind = clean_text(
            profile
                .get("git_tree_kind")
                .and_then(Value::as_str)
                .unwrap_or("master"),
            60,
        );
        let is_master = profile
            .get("is_master_agent")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| {
                let branch = git_branch.to_ascii_lowercase();
                let kind = git_tree_kind.to_ascii_lowercase();
                branch == "main" || branch == "master" || kind == "master" || kind == "main"
            });
        let auto_terminate_allowed = contract
            .get("auto_terminate_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(!is_master);
        let contract_remaining_ms = if auto_terminate_allowed {
            contract.get("remaining_ms").and_then(Value::as_i64)
        } else {
            None
        };
        let created_at = clean_text(
            profile
                .get("created_at")
                .and_then(Value::as_str)
                .or_else(|| contract.get("created_at").and_then(Value::as_str))
                .unwrap_or(""),
            80,
        );
        let updated_at = clean_text(
            profile
                .get("updated_at")
                .and_then(Value::as_str)
                .or_else(|| contract.get("updated_at").and_then(Value::as_str))
                .or_else(|| collab_row.and_then(|v| v.get("activated_at").and_then(Value::as_str)))
                .unwrap_or(""),
            80,
        );
        rows.push(json!({
            "id": agent_id,
            "agent_id": agent_id,
            "name": name,
            "role": role,
            "state": state,
            "model_provider": model_provider,
            "model_name": model_name,
            "runtime_model": model_runtime,
            "context_window": profile.get("context_window").cloned().unwrap_or(Value::Null),
            "context_window_tokens": profile.get("context_window_tokens").cloned().unwrap_or(Value::Null),
            "identity": identity,
            "avatar_url": profile.get("avatar_url").cloned().unwrap_or_else(|| json!("")),
            "system_prompt": profile.get("system_prompt").cloned().unwrap_or_else(|| json!("")),
            "fallback_models": profile.get("fallback_models").cloned().unwrap_or_else(|| json!([])),
            "git_branch": git_branch,
            "branch": git_branch,
            "git_tree_kind": git_tree_kind,
            "workspace_dir": profile.get("workspace_dir").cloned().unwrap_or(Value::Null),
            "workspace_rel": profile.get("workspace_rel").cloned().unwrap_or(Value::Null),
            "git_tree_ready": profile.get("git_tree_ready").cloned().unwrap_or_else(|| json!(true)),
            "git_tree_error": profile.get("git_tree_error").cloned().unwrap_or_else(|| json!("")),
            "is_master_agent": is_master,
            "created_at": created_at,
            "updated_at": updated_at,
            "contract": contract.clone(),
            "contract_expires_at": contract.get("expires_at").cloned().unwrap_or(Value::Null),
            "contract_remaining_ms": contract_remaining_ms.map(Value::from).unwrap_or(Value::Null),
            "auto_terminate_allowed": auto_terminate_allowed
        }));
    }
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows
}

fn agent_row_by_id(root: &Path, snapshot: &Value, agent_id: &str) -> Option<Value> {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return None;
    }
    build_agent_roster(root, snapshot, true)
        .into_iter()
        .find(|row| {
            clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or("")) == id
        })
}

fn update_profile_patch(root: &Path, agent_id: &str, patch: &Value) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    crate::dashboard_agent_state::upsert_profile(root, &id, patch)
}

fn upsert_contract_patch(root: &Path, agent_id: &str, patch: &Value) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    crate::dashboard_agent_state::upsert_contract(root, &id, patch)
}

fn session_payload(root: &Path, agent_id: &str) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let state = load_session_state(root, &id);
    let messages = session_messages(&state);
    let sessions = session_rows_payload(&state);
    json!({
        "ok": true,
        "agent_id": id,
        "active_session_id": state.get("active_session_id").cloned().unwrap_or_else(|| json!("default")),
        "messages": messages,
        "sessions": sessions,
        "session": state
    })
}

fn append_turn_message(root: &Path, agent_id: &str, user_text: &str, assistant_text: &str) {
    let _ = crate::dashboard_agent_state::append_turn(root, agent_id, user_text, assistant_text);
}

fn reset_active_session(root: &Path, agent_id: &str) -> Value {
    let id = clean_agent_id(agent_id);
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
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(row.get("session_id").and_then(Value::as_str).unwrap_or(""), 120);
            if sid == active_id {
                row["messages"] = Value::Array(Vec::new());
                row["updated_at"] = Value::String(crate::now_iso());
                break;
            }
        }
    }
    save_session_state(root, &id, &state);
    json!({
        "ok": true,
        "type": "dashboard_agent_session_reset",
        "agent_id": id,
        "active_session_id": active_id
    })
}

fn compact_active_session(root: &Path, agent_id: &str, request: &Value) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let target_window = request
        .get("target_context_window")
        .and_then(Value::as_i64)
        .unwrap_or(8192)
        .clamp(512, 2_000_000);
    let target_ratio = request
        .get("target_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.8)
        .clamp(0.2, 0.95);
    let min_recent_messages = request
        .get("min_recent_messages")
        .and_then(Value::as_u64)
        .unwrap_or(12)
        .clamp(2, 200) as usize;
    let max_messages = request
        .get("max_messages")
        .and_then(Value::as_u64)
        .unwrap_or(200)
        .clamp(20, 800) as usize;

    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let mut before_tokens = 0i64;
    let mut after_tokens = 0i64;
    let mut before_messages = 0usize;
    let mut after_messages = 0usize;
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(row.get("session_id").and_then(Value::as_str).unwrap_or(""), 120);
            if sid != active_id {
                continue;
            }
            if !row.get("messages").map(Value::is_array).unwrap_or(false) {
                row["messages"] = Value::Array(Vec::new());
            }
            let messages = row
                .get_mut("messages")
                .and_then(Value::as_array_mut)
                .expect("messages");
            before_messages = messages.len();
            before_tokens = messages
                .iter()
                .map(|item| {
                    let text = item
                        .get("text")
                        .and_then(Value::as_str)
                        .or_else(|| item.get("content").and_then(Value::as_str))
                        .unwrap_or("");
                    estimate_tokens(text)
                })
                .sum::<i64>();
            let target_tokens = ((target_window as f64) * target_ratio).round() as i64;
            if messages.len() > max_messages {
                let drain = messages.len().saturating_sub(max_messages);
                messages.drain(0..drain);
            }
            while messages.len() > min_recent_messages {
                let current_tokens = messages
                    .iter()
                    .map(|item| {
                        let text = item
                            .get("text")
                            .and_then(Value::as_str)
                            .or_else(|| item.get("content").and_then(Value::as_str))
                            .unwrap_or("");
                        estimate_tokens(text)
                    })
                    .sum::<i64>();
                if current_tokens <= target_tokens {
                    break;
                }
                messages.remove(0);
            }
            after_messages = messages.len();
            after_tokens = messages
                .iter()
                .map(|item| {
                    let text = item
                        .get("text")
                        .and_then(Value::as_str)
                        .or_else(|| item.get("content").and_then(Value::as_str))
                        .unwrap_or("");
                    estimate_tokens(text)
                })
                .sum::<i64>();
            row["updated_at"] = Value::String(crate::now_iso());
            break;
        }
    }
    save_session_state(root, &id, &state);
    json!({
        "ok": true,
        "type": "dashboard_agent_session_compact",
        "agent_id": id,
        "before_tokens": before_tokens,
        "after_tokens": after_tokens,
        "before_messages": before_messages,
        "after_messages": after_messages,
        "message": format!("Compaction complete: {} -> {} tokens", before_tokens, after_tokens)
    })
}

fn parse_agent_route(path_only: &str) -> Option<(String, Vec<String>)> {
    let prefix = "/api/agents/";
    if !path_only.starts_with(prefix) {
        return None;
    }
    let tail = path_only.trim_start_matches(prefix).trim_matches('/');
    if tail.is_empty() {
        return None;
    }
    let mut parts = tail
        .split('/')
        .map(|v| clean_text(v, 180))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    let agent_id = clean_agent_id(&parts.remove(0));
    if agent_id.is_empty() {
        return None;
    }
    Some((agent_id, parts))
}

fn decode_path_segment(raw: &str) -> String {
    let decoded = urlencoding::decode(raw)
        .ok()
        .map(|v| v.to_string())
        .unwrap_or_else(|| raw.to_string());
    clean_text(&decoded, 300)
}

fn git_tree_payload_for_agent(root: &Path, snapshot: &Value, agent_id: &str) -> Value {
    let roster = build_agent_roster(root, snapshot, true);
    let mut counts = HashMap::<String, i64>::new();
    let mut current = Value::Null;
    for row in &roster {
        let branch = clean_text(row.get("git_branch").and_then(Value::as_str).unwrap_or(""), 180);
        if branch.is_empty() {
            continue;
        }
        *counts.entry(branch.clone()).or_insert(0) += 1;
        if clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or("")) == clean_agent_id(agent_id) {
            current = row.clone();
        }
    }
    let mut branches = counts.keys().cloned().collect::<Vec<_>>();
    if !branches.iter().any(|row| row == "main") {
        branches.push("main".to_string());
        counts.insert("main".to_string(), 0);
    }
    branches.sort();
    let current_branch = clean_text(current.get("git_branch").and_then(Value::as_str).unwrap_or("main"), 180);
    let options = branches
        .iter()
        .map(|branch| {
            let kind = if branch == "main" || branch == "master" {
                "master"
            } else {
                "isolated"
            };
            json!({
                "branch": branch,
                "current": *branch == current_branch,
                "main": *branch == "main" || *branch == "master",
                "kind": kind,
                "in_use_by_agents": counts.get(branch).copied().unwrap_or(0)
            })
        })
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "current": {
            "git_branch": if current_branch.is_empty() { "main" } else { &current_branch },
            "git_tree_kind": if current_branch == "main" || current_branch == "master" { "master" } else { "isolated" },
            "workspace_dir": current.get("workspace_dir").cloned().unwrap_or(Value::Null),
            "workspace_rel": current.get("workspace_rel").cloned().unwrap_or(Value::Null),
            "git_tree_ready": true,
            "git_tree_error": ""
        },
        "options": options
    })
}

pub fn handle(root: &Path, method: &str, path: &str, body: &[u8], snapshot: &Value) -> Option<CompatApiResponse> {
    let path_only = path.split('?').next().unwrap_or(path);
    if let Some(payload) = crate::dashboard_terminal_broker::handle_http(root, method, path_only, body) {
        return Some(CompatApiResponse { status: 200, payload });
    }
    if let Some(response) = dashboard_compat_api_channels::handle(root, method, path_only, body) {
        return Some(response);
    }
    if let Some(response) = dashboard_skills_marketplace::handle(root, method, path, snapshot, body)
    {
        return Some(response);
    }

    if method == "GET" && path_only == "/api/agents/terminated" {
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_agent_state::terminated_entries(root),
        });
    }
    if method == "POST"
        && path_only.starts_with("/api/agents/")
        && path_only.ends_with("/revive")
    {
        let agent_id = path_only
            .trim_start_matches("/api/agents/")
            .trim_end_matches("/revive")
            .trim_matches('/');
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let role = request
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("analyst");
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_agent_state::revive_agent(root, agent_id, role),
        });
    }
    if method == "DELETE" && path_only == "/api/agents/terminated" {
        if query_value(path, "all")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::delete_all_terminated(root),
            });
        }
    }
    if method == "DELETE" && path_only.starts_with("/api/agents/terminated/") {
        let agent_id = path_only.trim_start_matches("/api/agents/terminated/").trim();
        return Some(CompatApiResponse {
            status: 200,
            payload: crate::dashboard_agent_state::delete_terminated(
                root,
                agent_id,
                query_value(path, "contract_id").as_deref(),
            ),
        });
    }

    if method == "GET" && path_only == "/api/agents" {
        let include_terminated = query_value(path, "include_terminated")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        return Some(CompatApiResponse {
            status: 200,
            payload: Value::Array(build_agent_roster(root, snapshot, include_terminated)),
        });
    }

    if method == "POST" && path_only == "/api/agents" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let manifest = clean_text(
            request
                .get("manifest_toml")
                .and_then(Value::as_str)
                .unwrap_or(""),
            20_000,
        );
        let manifest_fields = parse_manifest_fields(&manifest);
        let requested_name = clean_text(
            request
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("name").map(|v| v.as_str()))
                .unwrap_or(""),
            120,
        );
        let requested_role = clean_text(
            request
                .get("role")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("role").map(|v| v.as_str()))
                .unwrap_or("analyst"),
            60,
        );
        let role = if requested_role.is_empty() {
            "analyst".to_string()
        } else {
            requested_role
        };
        let name = if requested_name.is_empty() {
            "agent".to_string()
        } else {
            requested_name
        };
        let agent_id = make_agent_id(root, &name);
        let (default_provider, default_model) = extract_app_settings(snapshot);
        let model_provider = clean_text(
            request
                .get("provider")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("provider").map(|v| v.as_str()))
                .unwrap_or(&default_provider),
            80,
        );
        let model_name = clean_text(
            request
                .get("model")
                .and_then(Value::as_str)
                .or_else(|| manifest_fields.get("model").map(|v| v.as_str()))
                .unwrap_or(&default_model),
            120,
        );
        let model_override = if model_provider.is_empty() || model_name.is_empty() {
            "auto".to_string()
        } else {
            format!("{model_provider}/{model_name}")
        };
        let identity = if request.get("identity").map(Value::is_object).unwrap_or(false) {
            request.get("identity").cloned().unwrap_or_else(|| json!({}))
        } else {
            json!({
                "emoji": request.get("emoji").cloned().unwrap_or_else(|| json!("🧑‍💻")),
                "color": request.get("color").cloned().unwrap_or_else(|| json!("#2563EB")),
                "archetype": request.get("archetype").cloned().unwrap_or_else(|| json!("assistant")),
                "vibe": request.get("vibe").cloned().unwrap_or_else(|| json!(""))
            })
        };
        let profile_patch = json!({
            "agent_id": agent_id,
            "name": name,
            "role": role,
            "state": "Running",
            "model_override": model_override,
            "model_provider": model_provider,
            "model_name": model_name,
            "runtime_model": model_name,
            "system_prompt": request.get("system_prompt").cloned().unwrap_or_else(|| json!("")),
            "identity": identity,
            "fallback_models": request.get("fallback_models").cloned().unwrap_or_else(|| json!([])),
            "git_tree_kind": "master",
            "git_branch": "main",
            "workspace_dir": root.to_string_lossy().to_string(),
            "workspace_rel": "",
            "git_tree_ready": true,
            "git_tree_error": "",
            "is_master_agent": true
        });
        let _ = update_profile_patch(root, &agent_id, &profile_patch);
        let contract_obj = request.get("contract").cloned().unwrap_or_else(|| json!({}));
        let expiry_seconds = contract_obj
            .get("expiry_seconds")
            .and_then(Value::as_i64)
            .unwrap_or(3600)
            .clamp(1, 31 * 24 * 60 * 60);
        let auto_terminate_allowed = contract_obj
            .get("auto_terminate_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let contract_patch = json!({
            "agent_id": agent_id,
            "status": "active",
            "created_at": crate::now_iso(),
            "updated_at": crate::now_iso(),
            "owner": clean_text(contract_obj.get("owner").and_then(Value::as_str).unwrap_or("dashboard_auto"), 80),
            "mission": clean_text(contract_obj.get("mission").and_then(Value::as_str).unwrap_or("Assist with assigned mission."), 200),
            "termination_condition": clean_text(contract_obj.get("termination_condition").and_then(Value::as_str).unwrap_or("task_or_timeout"), 80),
            "expiry_seconds": expiry_seconds,
            "auto_terminate_allowed": auto_terminate_allowed,
            "conversation_hold": contract_obj.get("conversation_hold").and_then(Value::as_bool).unwrap_or(false),
            "expires_at": clean_text(contract_obj.get("expires_at").and_then(Value::as_str).unwrap_or(""), 80)
        });
        let _ = upsert_contract_patch(root, &agent_id, &contract_patch);
        append_turn_message(root, &agent_id, "", "");
        let row = agent_row_by_id(root, snapshot, &agent_id).unwrap_or_else(|| {
            json!({
                "id": agent_id,
                "name": name,
                "role": role,
                "state": "Running",
                "model_provider": model_provider,
                "model_name": model_name
            })
        });
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({
                "ok": true,
                "id": row.get("id").cloned().unwrap_or_else(|| json!("")),
                "agent_id": row.get("id").cloned().unwrap_or_else(|| json!("")),
                "name": row.get("name").cloned().unwrap_or_else(|| json!("agent")),
                "state": row.get("state").cloned().unwrap_or_else(|| json!("Running")),
                "model_provider": row.get("model_provider").cloned().unwrap_or_else(|| json!(default_provider)),
                "model_name": row.get("model_name").cloned().unwrap_or_else(|| json!(default_model)),
                "runtime_model": row.get("runtime_model").cloned().unwrap_or_else(|| json!(default_model)),
                "created_at": row.get("created_at").cloned().unwrap_or_else(|| json!(crate::now_iso()))
            }),
        });
    }

    if let Some((agent_id, segments)) = parse_agent_route(path_only) {
        let existing = agent_row_by_id(root, snapshot, &agent_id);
        if method == "GET" && segments.is_empty() {
            if let Some(row) = existing {
                return Some(CompatApiResponse {
                    status: 200,
                    payload: row,
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
            });
        }

        if method == "DELETE" && segments.is_empty() {
            if existing.is_none() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({"state": "Archived", "updated_at": crate::now_iso()}),
            );
            let _ = upsert_contract_patch(
                root,
                &agent_id,
                &json!({
                    "status": "terminated",
                    "termination_reason": "user_archived",
                    "terminated_at": crate::now_iso(),
                    "updated_at": crate::now_iso()
                }),
            );
            let _ = crate::dashboard_agent_state::archive_agent(root, &agent_id, "user_archive");
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_archive", "agent_id": agent_id}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "stop" {
            if existing.is_none() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
                });
            }
            let _ = upsert_contract_patch(
                root,
                &agent_id,
                &json!({
                    "status": "terminated",
                    "termination_reason": "stopped",
                    "terminated_at": crate::now_iso(),
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_stop", "agent_id": agent_id}),
            });
        }

        if existing.is_none() {
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "agent_not_found", "agent_id": agent_id}),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "session" {
            return Some(CompatApiResponse {
                status: 200,
                payload: session_payload(root, &agent_id),
            });
        }

        if method == "POST" && segments.len() == 2 && segments[0] == "session" && segments[1] == "reset" {
            return Some(CompatApiResponse {
                status: 200,
                payload: reset_active_session(root, &agent_id),
            });
        }

        if method == "POST" && segments.len() == 2 && segments[0] == "session" && segments[1] == "compact" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: compact_active_session(root, &agent_id, &request),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "sessions" {
            let payload = session_payload(root, &agent_id);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "agent_id": agent_id,
                    "active_session_id": payload.get("active_session_id").cloned().unwrap_or_else(|| json!("default")),
                    "sessions": payload.get("sessions").cloned().unwrap_or_else(|| json!([]))
                }),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "sessions" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let label = clean_text(request.get("label").and_then(Value::as_str).unwrap_or("Session"), 80);
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::create_session(root, &agent_id, &label),
            });
        }

        if method == "POST"
            && segments.len() == 3
            && segments[0] == "sessions"
            && segments[2] == "switch"
        {
            let session_id = decode_path_segment(&segments[1]);
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::switch_session(root, &agent_id, &session_id),
            });
        }

        if method == "DELETE" && segments.len() == 1 && segments[0] == "history" {
            let mut state = load_session_state(root, &agent_id);
            if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
                for row in rows.iter_mut() {
                    row["messages"] = Value::Array(Vec::new());
                    row["updated_at"] = Value::String(crate::now_iso());
                }
            }
            save_session_state(root, &agent_id, &state);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "type": "dashboard_agent_history_cleared", "agent_id": agent_id}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "message" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let message = clean_text(request.get("message").and_then(Value::as_str).unwrap_or(""), 8_000);
            if message.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "message_required"}),
                });
            }
            let row = agent_row_by_id(root, snapshot, &agent_id).unwrap_or_else(|| json!({}));
            let provider = clean_text(row.get("model_provider").and_then(Value::as_str).unwrap_or("auto"), 80);
            let model = clean_text(row.get("model_name").and_then(Value::as_str).unwrap_or("model"), 120);
            let response_text = format!("[{provider}/{model}] {message}");
            append_turn_message(root, &agent_id, &message, &response_text);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "agent_id": agent_id,
                    "response": response_text,
                    "provider": provider,
                    "model": model,
                    "runtime_model": model,
                    "input_tokens": estimate_tokens(&message),
                    "output_tokens": estimate_tokens(&message),
                    "cost_usd": 0.0,
                    "iterations": 1,
                    "tools": []
                }),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "suggestions" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let hint = clean_text(
                request
                    .get("user_hint")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("hint").and_then(Value::as_str))
                    .unwrap_or(""),
                220,
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_agent_state::suggestions(root, &agent_id, &hint),
            });
        }

        if method == "PATCH" && segments.len() == 1 && segments[0] == "config" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let mut patch = request.clone();
            if !patch.is_object() {
                patch = json!({});
            }
            if !patch.get("identity").map(Value::is_object).unwrap_or(false) {
                let emoji = clean_text(patch.get("emoji").and_then(Value::as_str).unwrap_or(""), 16);
                let color = clean_text(patch.get("color").and_then(Value::as_str).unwrap_or(""), 32);
                let archetype = clean_text(
                    patch.get("archetype").and_then(Value::as_str).unwrap_or(""),
                    80,
                );
                let vibe = clean_text(patch.get("vibe").and_then(Value::as_str).unwrap_or(""), 80);
                if !emoji.is_empty() || !color.is_empty() || !archetype.is_empty() || !vibe.is_empty() {
                    patch["identity"] = json!({
                        "emoji": emoji,
                        "color": color,
                        "archetype": archetype,
                        "vibe": vibe
                    });
                }
            }
            let _ = update_profile_patch(root, &agent_id, &patch);
            if patch.get("contract").map(Value::is_object).unwrap_or(false) {
                let _ = upsert_contract_patch(
                    root,
                    &agent_id,
                    patch.get("contract").unwrap_or(&json!({})),
                );
            } else if patch.get("expiry_seconds").is_some()
                || patch.get("termination_condition").is_some()
                || patch.get("auto_terminate_allowed").is_some()
            {
                let _ = upsert_contract_patch(root, &agent_id, &patch);
            }
            let row = agent_row_by_id(root, snapshot, &agent_id).unwrap_or_else(|| json!({"id": agent_id}));
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "agent": row}),
            });
        }

        if method == "PUT" && segments.len() == 1 && segments[0] == "model" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let requested = clean_text(request.get("model").and_then(Value::as_str).unwrap_or(""), 200);
            if requested.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "model_required"}),
                });
            }
            let (default_provider, default_model) = extract_app_settings(snapshot);
            let (provider, model) = split_model_ref(&requested, &default_provider, &default_model);
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({
                    "model_override": format!("{provider}/{model}"),
                    "model_provider": provider,
                    "model_name": model,
                    "runtime_model": model
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "agent_id": agent_id,
                    "provider": provider,
                    "model": model,
                    "runtime_model": model
                }),
            });
        }

        if method == "PUT" && segments.len() == 1 && segments[0] == "mode" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let mode = clean_text(request.get("mode").and_then(Value::as_str).unwrap_or(""), 40);
            let _ = update_profile_patch(root, &agent_id, &json!({"mode": mode, "updated_at": crate::now_iso()}));
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "mode": mode}),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "git-trees" {
            return Some(CompatApiResponse {
                status: 200,
                payload: git_tree_payload_for_agent(root, snapshot, &agent_id),
            });
        }

        if method == "POST" && segments.len() == 2 && segments[0] == "git-tree" && segments[1] == "switch" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let branch = clean_text(request.get("branch").and_then(Value::as_str).unwrap_or(""), 180);
            if branch.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "branch_required"}),
                });
            }
            let kind = if branch == "main" || branch == "master" {
                "master"
            } else {
                "isolated"
            };
            let _ = update_profile_patch(
                root,
                &agent_id,
                &json!({
                    "git_branch": branch,
                    "git_tree_kind": kind,
                    "git_tree_ready": true,
                    "git_tree_error": "",
                    "updated_at": crate::now_iso()
                }),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: git_tree_payload_for_agent(root, snapshot, &agent_id),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "files" {
            let dir = agent_files_dir(root, &agent_id);
            let mut rows = Vec::<Value>::new();
            let defaults = vec!["SOUL.md".to_string(), "SYSTEM.md".to_string()];
            for name in defaults {
                let path = dir.join(&name);
                rows.push(json!({
                    "name": name,
                    "exists": path.exists(),
                    "size": fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0)
                }));
            }
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let name = clean_text(path.file_name().and_then(|v| v.to_str()).unwrap_or(""), 180);
                    if name.is_empty() {
                        continue;
                    }
                    if rows.iter().any(|row| row.get("name").and_then(Value::as_str) == Some(name.as_str())) {
                        continue;
                    }
                    rows.push(json!({
                        "name": name,
                        "exists": true,
                        "size": fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0)
                    }));
                }
            }
            rows.sort_by(|a, b| {
                clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 180)
                    .cmp(&clean_text(b.get("name").and_then(Value::as_str).unwrap_or(""), 180))
            });
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "files": rows}),
            });
        }

        if (method == "GET" || method == "PUT")
            && segments.len() >= 2
            && segments[0] == "files"
        {
            let file_name = decode_path_segment(&segments[1..].join("/"));
            if file_name.is_empty() || file_name.contains("..") {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "invalid_file_name"}),
                });
            }
            let path = agent_files_dir(root, &agent_id).join(&file_name);
            if method == "GET" {
                if !path.exists() {
                    return Some(CompatApiResponse {
                        status: 404,
                        payload: json!({"ok": false, "error": "file_not_found"}),
                    });
                }
                let content = fs::read_to_string(&path).unwrap_or_default();
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "agent_id": agent_id, "name": file_name, "content": content}),
                });
            }
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let content = request
                .get("content")
                .and_then(Value::as_str)
                .map(|v| v.to_string())
                .unwrap_or_default();
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&path, content);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "name": file_name}),
            });
        }

        if method == "GET" && segments.len() == 1 && segments[0] == "tools" {
            let payload = read_json_loose(&agent_tools_path(root, &agent_id)).unwrap_or_else(|| {
                json!({"tool_allowlist": [], "tool_blocklist": []})
            });
            return Some(CompatApiResponse {
                status: 200,
                payload,
            });
        }

        if method == "PUT" && segments.len() == 1 && segments[0] == "tools" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let payload = json!({
                "tool_allowlist": request.get("tool_allowlist").cloned().unwrap_or_else(|| json!([])),
                "tool_blocklist": request.get("tool_blocklist").cloned().unwrap_or_else(|| json!([]))
            });
            write_json_pretty(&agent_tools_path(root, &agent_id), &payload);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "tool_filters": payload}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "clone" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let source = existing.unwrap_or_else(|| json!({}));
            let source_name = clean_text(source.get("name").and_then(Value::as_str).unwrap_or("agent"), 120);
            let new_name = clean_text(
                request
                    .get("new_name")
                    .and_then(Value::as_str)
                    .unwrap_or(&(source_name.clone() + "-copy")),
                120,
            );
            let new_id = make_agent_id(root, &new_name);
            let mut profile_patch = source.clone();
            profile_patch["name"] = Value::String(new_name.clone());
            profile_patch["agent_id"] = Value::String(new_id.clone());
            profile_patch["state"] = Value::String("Running".to_string());
            profile_patch["created_at"] = Value::String(crate::now_iso());
            profile_patch["updated_at"] = Value::String(crate::now_iso());
            let _ = update_profile_patch(root, &new_id, &profile_patch);
            let _ = upsert_contract_patch(
                root,
                &new_id,
                &json!({
                    "status": "active",
                    "created_at": crate::now_iso(),
                    "updated_at": crate::now_iso(),
                    "owner": "dashboard_clone",
                    "mission": format!("Assist with assigned mission for {}.", new_id),
                    "termination_condition": "task_or_timeout",
                    "expiry_seconds": 3600,
                    "auto_terminate_allowed": false
                }),
            );
            append_turn_message(root, &new_id, "", "");
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": new_id, "name": new_name}),
            });
        }

        if method == "POST" && segments.len() == 1 && segments[0] == "avatar" {
            let mime = clean_text(path, 1); // placeholder to avoid empty mime warnings below
            let _ = mime;
            let content_type = clean_text(
                query_value(path, "content_type")
                    .as_deref()
                    .unwrap_or(""),
                120,
            );
            let inferred = if content_type.is_empty() {
                "image/png".to_string()
            } else {
                content_type
            };
            let encoded = {
                use base64::engine::general_purpose::STANDARD;
                use base64::Engine;
                STANDARD.encode(body)
            };
            let avatar_url = format!("data:{};base64,{}", inferred, encoded);
            let _ = update_profile_patch(root, &agent_id, &json!({"avatar_url": avatar_url}));
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "agent_id": agent_id, "avatar_url": avatar_url}),
            });
        }
    }

    let usage = usage_from_snapshot(snapshot);
    let runtime = runtime_sync_summary(snapshot);
    let alerts_count = parse_non_negative_i64(snapshot.pointer("/health/alerts/count"), 0);
    let status = if snapshot.get("ok").and_then(Value::as_bool).unwrap_or(false) && alerts_count == 0 {
        "healthy"
    } else if snapshot.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        "degraded"
    } else {
        "critical"
    };

    if method == "GET" {
        let payload = match path_only {
            "/api/health" => json!({
                "ok": true,
                "status": status,
                "checks": snapshot.pointer("/health/checks").cloned().unwrap_or_else(|| json!({})),
                "alerts": snapshot.pointer("/health/alerts").cloned().unwrap_or_else(|| json!({"count": 0, "checks": []})),
                "dashboard_metrics": snapshot.pointer("/health/dashboard_metrics").cloned().unwrap_or_else(|| json!({})),
                "runtime_sync": runtime,
                "receipt_hash": snapshot.get("receipt_hash").cloned().unwrap_or(Value::Null),
                "ts": crate::now_iso()
            }),
            "/api/usage" => json!({"ok": true, "agents": usage["agents"].clone(), "summary": usage["summary"].clone(), "by_model": usage["models"].clone(), "daily": usage["daily"].clone()}),
            "/api/usage/summary" => json!({"ok": true, "summary": usage["summary"].clone()}),
            "/api/usage/by-model" => json!({"ok": true, "models": usage["models"].clone()}),
            "/api/usage/daily" => json!({"ok": true, "days": usage["daily"].clone()}),
            "/api/providers" => providers_payload(root, snapshot),
            "/api/models" => crate::dashboard_model_catalog::catalog_payload(root, snapshot),
            "/api/models/recommended" => crate::dashboard_model_catalog::route_decision_payload(
                root,
                snapshot,
                &json!({"task_type":"general","budget_mode":"balanced"}),
            ),
            "/api/route/decision" =>
                crate::dashboard_model_catalog::route_decision_payload(root, snapshot, &json!({})),
            "/api/channels" => dashboard_compat_api_channels::channels_payload(root),
            "/api/eyes" => read_eyes_payload(root),
            "/api/audit/recent" => {
                let entries = recent_audit_entries(root, snapshot);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"entries": entries}));
                json!({"ok": true, "entries": entries, "tip_hash": tip_hash})
            }
            "/api/audit/verify" => {
                let entries = recent_audit_entries(root, snapshot);
                let tip_hash = crate::deterministic_receipt_hash(&json!({"entries": entries}));
                json!({"ok": true, "valid": true, "entries": entries.len(), "tip_hash": tip_hash})
            }
            "/api/version" => {
                let version = read_json(&root.join("package.json"))
                    .and_then(|v| v.get("version").and_then(Value::as_str).map(str::to_string))
                    .unwrap_or_else(|| "0.1.0".to_string());
                json!({"ok": true, "version": version, "rust_authority": "rust_core_lanes"})
            }
            "/api/network/status" => json!({"ok": true, "enabled": true, "connected_peers": 0, "total_peers": 0, "runtime_sync": runtime}),
            "/api/peers" => json!({"ok": true, "peers": [], "connected": 0, "total": 0, "runtime_sync": runtime}),
            "/api/security" => json!({
                "ok": true,
                "mode": "strict",
                "fail_closed": true,
                "receipts_required": true,
                "checks": snapshot.pointer("/health/checks").cloned().unwrap_or_else(|| json!({})),
                "alerts": snapshot.pointer("/health/alerts").cloned().unwrap_or_else(|| json!({})),
                "runtime_sync": runtime
            }),
            "/api/tools" => json!({
                "ok": true,
                "tools": [
                    {"name": "protheus-ops", "category": "runtime"},
                    {"name": "infringd", "category": "runtime"},
                    {"name": "git", "category": "cli"},
                    {"name": "rg", "category": "cli"}
                ],
                "runtime_sync": runtime
            }),
            "/api/commands" => json!({
                "ok": true,
                "commands": [
                    {"command": "/status", "description": "Show runtime status and cockpit summary"},
                    {"command": "/queue", "description": "Show current queue pressure"},
                    {"command": "/context", "description": "Show context and attention state"},
                    {"command": "/model", "description": "Inspect or switch active model"},
                    {"command": "/file <path>", "description": "Render full file output in chat from workspace path"},
                    {"command": "/folder <path>", "description": "Render folder tree + downloadable archive in chat"}
                ]
            }),
            "/api/budget" => json!({
                "ok": true,
                "hourly_spend": 0,
                "daily_spend": usage.pointer("/summary/total_cost_usd").cloned().unwrap_or_else(|| json!(0)),
                "monthly_spend": usage.pointer("/summary/total_cost_usd").cloned().unwrap_or_else(|| json!(0)),
                "hourly_limit": 0,
                "daily_limit": 0,
                "monthly_limit": 0
            }),
            "/api/a2a/agents" => json!({"ok": true, "agents": []}),
            "/api/approvals" => approvals_payload(root),
            "/api/sessions" => json!({"ok": true, "sessions": snapshot.pointer("/agents/session_summaries/rows").cloned().unwrap_or_else(|| json!([]))}),
            "/api/workflows" => rows_from_array_store(root, WORKFLOWS_REL, "workflows"),
            "/api/cron/jobs" => rows_from_array_store(root, CRON_JOBS_REL, "jobs"),
            "/api/triggers" => rows_from_array_store(root, TRIGGERS_REL, "triggers"),
            "/api/schedules" => rows_from_array_store(root, CRON_JOBS_REL, "schedules"),
            "/api/comms/topology" => json!({
                "ok": true,
                "topology": {
                    "nodes": snapshot.pointer("/collab/dashboard/agents").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
                    "edges": 0,
                    "connected": true
                }
            }),
            "/api/comms/events" => json!({"ok": true, "events": []}),
            "/api/hands" | "/api/hands/active" => json!({"ok": true, "hands": [], "active": []}),
            "/api/profiles" => json!({"ok": true, "profiles": extract_profiles(root)}),
            "/api/update/check" => crate::dashboard_release_update::check_update(root),
            "/api/templates" => json!({
                "ok": true,
                "templates": [
                    {"id": "general-assistant", "name": "General Assistant", "provider": "auto", "model": "auto"},
                    {"id": "research-analyst", "name": "Research Analyst", "provider": "openai", "model": "gpt-5"},
                    {"id": "ops-reliability", "name": "Ops Reliability", "provider": "anthropic", "model": "claude-opus-4-20250514"}
                ]
            }),
            _ => return None,
        };
        return Some(CompatApiResponse { status: 200, payload });
    }

    if method == "POST" {
        if path_only == "/api/update/apply" {
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_release_update::apply_update(root),
            });
        }
        if path_only == "/api/route/decision" {
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            return Some(CompatApiResponse {
                status: 200,
                payload: crate::dashboard_model_catalog::route_decision_payload(
                    root, snapshot, &request,
                ),
            });
        }
        return None;
    }

    if method == "DELETE" {
        return None;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers_endpoint_uses_registry_rows() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &state_path(root.path(), PROVIDER_REGISTRY_REL),
            &json!({
                "type": "infring_dashboard_provider_registry",
                "providers": {
                    "ollama": {"id": "ollama", "display_name": "Ollama", "is_local": true, "needs_key": false},
                    "openai": {"id": "openai", "display_name": "OpenAI", "is_local": false, "needs_key": true}
                }
            }),
        );
        let out = handle(
            root.path(),
            "GET",
            "/api/providers",
            &[],
            &json!({"ok": true}),
        )
        .expect("providers");
        let rows = out
            .payload
            .get("providers")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn channels_endpoint_returns_catalog_defaults() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = handle(
            root.path(),
            "GET",
            "/api/channels",
            &[],
            &json!({"ok": true}),
        )
        .expect("channels");
        let rows = out
            .payload
            .get("channels")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.len() >= 40);
        assert!(rows.iter().any(|row| {
            row.get("name")
                .and_then(Value::as_str)
                .map(|v| v == "whatsapp")
                .unwrap_or(false)
        }));
    }

    #[test]
    fn channels_configure_and_test_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let configure = handle(
            root.path(),
            "POST",
            "/api/channels/discord/configure",
            br#"{"fields":{"bot_token":"abc","channel_id":"123"}}"#,
            &json!({"ok": true}),
        )
        .expect("configure");
        assert_eq!(configure.status, 200);
        let test = handle(
            root.path(),
            "POST",
            "/api/channels/discord/test",
            &[],
            &json!({"ok": true}),
        )
        .expect("test");
        assert_eq!(
            test.payload.get("status").and_then(Value::as_str),
            Some("ok")
        );
    }

    #[test]
    fn route_decision_endpoint_prefers_local_when_offline() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &state_path(root.path(), PROVIDER_REGISTRY_REL),
            &json!({
                "type": "infring_dashboard_provider_registry",
                "providers": {
                    "ollama": {
                        "id": "ollama",
                        "is_local": true,
                        "needs_key": false,
                        "auth_status": "ok",
                        "model_profiles": {
                            "smallthinker:4b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 4, "specialty":"general"}
                        }
                    },
                    "openai": {
                        "id": "openai",
                        "is_local": false,
                        "needs_key": true,
                        "auth_status": "set",
                        "model_profiles": {
                            "gpt-5": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 70, "specialty":"general"}
                        }
                    }
                }
            }),
        );
        let out = handle(
            root.path(),
            "POST",
            "/api/route/decision",
            br#"{"offline_required":true,"task_type":"general"}"#,
            &json!({"ok": true}),
        )
        .expect("route decision");
        assert_eq!(
            out.payload
                .get("selected")
                .and_then(|v| v.get("provider"))
                .and_then(Value::as_str),
            Some("ollama")
        );
    }

    #[test]
    fn whatsapp_qr_start_exposes_data_url() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = handle(
            root.path(),
            "POST",
            "/api/channels/whatsapp/qr/start",
            &[],
            &json!({"ok": true}),
        )
        .expect("qr");
        let url = out
            .payload
            .get("qr_data_url")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(url.starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn terminated_agent_endpoints_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = crate::dashboard_agent_state::upsert_contract(
            root.path(),
            "agent-a",
            &json!({
                "created_at": "2000-01-01T00:00:00Z",
                "expiry_seconds": 1,
                "status": "active"
            }),
        );
        let _ = crate::dashboard_agent_state::enforce_expired_contracts(root.path());

        let listed = handle(
            root.path(),
            "GET",
            "/api/agents/terminated",
            &[],
            &json!({"ok": true}),
        )
        .expect("terminated list");
        let rows = listed
            .payload
            .get("entries")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!rows.is_empty());

        let revived = handle(
            root.path(),
            "POST",
            "/api/agents/agent-a/revive",
            br#"{"role":"analyst"}"#,
            &json!({"ok": true}),
        )
        .expect("revive");
        assert_eq!(revived.payload.get("ok").and_then(Value::as_bool), Some(true));

        let after_revive = handle(
            root.path(),
            "GET",
            "/api/agents/terminated",
            &[],
            &json!({"ok": true}),
        )
        .expect("terminated list after revive");
        let rows_after = after_revive
            .payload
            .get("entries")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows_after.is_empty());

        let _ = crate::dashboard_agent_state::upsert_contract(
            root.path(),
            "agent-a",
            &json!({
                "created_at": "2000-01-01T00:00:00Z",
                "expiry_seconds": 1,
                "status": "active"
            }),
        );
        let _ = crate::dashboard_agent_state::enforce_expired_contracts(root.path());
        let deleted = handle(
            root.path(),
            "DELETE",
            "/api/agents/terminated/agent-a",
            &[],
            &json!({"ok": true}),
        )
        .expect("delete terminated");
        assert!(
            deleted
                .payload
                .get("removed_history_entries")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1
        );
    }

    #[test]
    fn terminal_endpoints_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let created = handle(
            root.path(),
            "POST",
            "/api/terminal/sessions",
            br#"{"id":"term-a"}"#,
            &json!({"ok": true}),
        )
        .expect("create");
        assert_eq!(created.payload.get("ok").and_then(Value::as_bool), Some(true));
        let listed = handle(
            root.path(),
            "GET",
            "/api/terminal/sessions",
            &[],
            &json!({"ok": true}),
        )
        .expect("list");
        assert_eq!(
            listed
                .payload
                .get("sessions")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        let ran = handle(
            root.path(),
            "POST",
            "/api/terminal/queue",
            br#"{"session_id":"term-a","command":"printf 'ok'"}"#,
            &json!({"ok": true}),
        )
        .expect("exec");
        assert_eq!(ran.payload.get("stdout").and_then(Value::as_str), Some("ok"));
    }
}
