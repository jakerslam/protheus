// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const AGENT_PROFILES_REL: &str = "client/runtime/local/state/ui/infring_dashboard/agent_profiles.json";
const ARCHIVED_AGENTS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/archived_agents.json";
const AGENT_CONTRACTS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/agent_contracts.json";
const DEFAULT_EXPIRY_SECONDS: i64 = 86_400;
const MAX_EXPIRY_SECONDS: i64 = 31 * 24 * 60 * 60;

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

fn as_object_mut<'a>(root: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !root.get(key).map(Value::is_object).unwrap_or(false) {
        root[key] = json!({});
    }
    root.get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object shape")
}

fn as_array_mut<'a>(root: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    if !root.get(key).map(Value::is_array).unwrap_or(false) {
        root[key] = Value::Array(Vec::new());
    }
    root.get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("array shape")
}

fn profiles_path(root: &Path) -> PathBuf {
    root.join(AGENT_PROFILES_REL)
}

fn archived_path(root: &Path) -> PathBuf {
    root.join(ARCHIVED_AGENTS_REL)
}

fn contracts_path(root: &Path) -> PathBuf {
    root.join(AGENT_CONTRACTS_REL)
}

fn default_profiles_state() -> Value {
    json!({
        "type": "infring_dashboard_agent_profiles",
        "updated_at": now_iso(),
        "agents": {}
    })
}

fn load_profiles_state(root: &Path) -> Value {
    let mut state = read_json_file(&profiles_path(root)).unwrap_or_else(default_profiles_state);
    if !state.is_object() {
        state = default_profiles_state();
    }
    let _ = as_object_mut(&mut state, "agents");
    state
}

fn save_profiles_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(now_iso());
    write_json(&profiles_path(root), &state);
}

fn default_archived_state() -> Value {
    json!({
        "type": "infring_dashboard_archived_agents",
        "updated_at": now_iso(),
        "agents": {}
    })
}

fn load_archived_state(root: &Path) -> Value {
    let mut state = read_json_file(&archived_path(root)).unwrap_or_else(default_archived_state);
    if !state.is_object() {
        state = default_archived_state();
    }
    let _ = as_object_mut(&mut state, "agents");
    state
}

fn save_archived_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(now_iso());
    write_json(&archived_path(root), &state);
}

fn default_contract(agent_id: &str) -> Value {
    let now = now_iso();
    json!({
        "contract_id": format!(
            "contract-{}",
            crate::deterministic_receipt_hash(&json!({"agent_id": agent_id, "ts": now}))
                .chars()
                .take(16)
                .collect::<String>()
        ),
        "agent_id": agent_id,
        "mission": format!("Assist with assigned mission for {}.", agent_id),
        "owner": "dashboard_session",
        "status": "active",
        "termination_condition": "task_or_timeout",
        "expiry_seconds": DEFAULT_EXPIRY_SECONDS,
        "created_at": now,
        "updated_at": now,
        "expires_at": "",
        "auto_terminate_allowed": true
    })
}

fn default_contracts_state() -> Value {
    json!({
        "type": "infring_dashboard_agent_contracts",
        "updated_at": now_iso(),
        "contracts": {},
        "terminated_history": []
    })
}

fn load_contracts_state(root: &Path) -> Value {
    let mut state = read_json_file(&contracts_path(root)).unwrap_or_else(default_contracts_state);
    if !state.is_object() {
        state = default_contracts_state();
    }
    let _ = as_object_mut(&mut state, "contracts");
    let _ = as_array_mut(&mut state, "terminated_history");
    state
}

fn save_contracts_state(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(now_iso());
    write_json(&contracts_path(root), &state);
}

fn parse_expiry_seconds(value: Option<&Value>) -> i64 {
    value
        .and_then(Value::as_i64)
        .unwrap_or(DEFAULT_EXPIRY_SECONDS)
        .clamp(1, MAX_EXPIRY_SECONDS)
}

fn parse_ts(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|v| v.with_timezone(&Utc))
}

pub fn archived_agent_ids(root: &Path) -> HashSet<String> {
    let state = load_archived_state(root);
    state
        .get("agents")
        .and_then(Value::as_object)
        .map(|rows| {
            rows.keys()
                .map(|row| normalize_agent_id(row))
                .filter(|row| !row.is_empty())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default()
}

pub fn merge_profiles_into_collab(root: &Path, collab_payload: &mut Value, default_team: &str) {
    let profiles_state = load_profiles_state(root);
    let profiles = profiles_state
        .get("agents")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if profiles.is_empty() {
        return;
    }
    let archived = archived_agent_ids(root);
    if !collab_payload.get("dashboard").map(Value::is_object).unwrap_or(false) {
        collab_payload["dashboard"] = json!({
            "version": "v1",
            "team": default_team,
            "agents": [],
            "tasks": [],
            "handoff_history": []
        });
    }
    if !collab_payload["dashboard"]
        .get("agents")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        collab_payload["dashboard"]["agents"] = Value::Array(Vec::new());
    }
    let rows = collab_payload["dashboard"]["agents"]
        .as_array_mut()
        .expect("agents array");
    let mut existing = rows
        .iter()
        .filter_map(|row| row.get("shadow").and_then(Value::as_str))
        .map(normalize_agent_id)
        .collect::<HashSet<_>>();

    for (raw_id, profile) in profiles {
        let agent_id = normalize_agent_id(&raw_id);
        if agent_id.is_empty() || archived.contains(&agent_id) || existing.contains(&agent_id) {
            continue;
        }
        let role = profile
            .get("role")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 60))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "analyst".to_string());
        let status = profile
            .get("state")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 40))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "inactive".to_string());
        rows.push(json!({
            "shadow": agent_id,
            "role": role,
            "status": status,
            "activated_at": profile
                .get("updated_at")
                .cloned()
                .unwrap_or_else(|| Value::String(String::new())),
            "source": "profile_state"
        }));
        existing.insert(agent_id);
    }
}

pub fn upsert_profile(root: &Path, agent_id: &str, patch: &Value) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_profiles_state(root);
    let agents = as_object_mut(&mut state, "agents");
    let mut current = agents.get(&id).cloned().unwrap_or_else(|| json!({}));
    if !current.is_object() {
        current = json!({});
    }
    if let Some(obj) = patch.as_object() {
        for (key, value) in obj {
            if matches!(
                key.as_str(),
                "role" | "name" | "emoji" | "avatar_url" | "state" | "description" | "lifespan"
            ) {
                current[key] = value.clone();
            }
        }
    }
    if !current
        .get("created_at")
        .and_then(Value::as_str)
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        current["created_at"] = Value::String(now_iso());
    }
    current["updated_at"] = Value::String(now_iso());
    agents.insert(id.clone(), current.clone());
    save_profiles_state(root, state);
    json!({"ok": true, "type": "dashboard_agent_profile", "agent_id": id, "profile": current})
}

pub fn archive_agent(root: &Path, agent_id: &str, reason: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_archived_state(root);
    let agents = as_object_mut(&mut state, "agents");
    agents.insert(
        id.clone(),
        json!({
            "reason": clean_text(reason, 120),
            "archived_at": now_iso()
        }),
    );
    save_archived_state(root, state);
    json!({"ok": true, "type": "dashboard_agent_archive", "agent_id": id})
}

pub fn unarchive_agent(root: &Path, agent_id: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_archived_state(root);
    let agents = as_object_mut(&mut state, "agents");
    let removed = agents.remove(&id).is_some();
    save_archived_state(root, state);
    json!({"ok": true, "type": "dashboard_agent_unarchive", "agent_id": id, "removed": removed})
}

pub fn upsert_contract(root: &Path, agent_id: &str, patch: &Value) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_contracts_state(root);
    let contracts = as_object_mut(&mut state, "contracts");
    let mut contract = contracts
        .get(&id)
        .cloned()
        .unwrap_or_else(|| default_contract(&id));
    let mut saw_status_patch = false;
    let mut saw_lifecycle_patch = false;
    if let Some(obj) = patch.as_object() {
        for (key, value) in obj {
            if matches!(
                key.as_str(),
                "mission" | "owner" | "termination_condition" | "expires_at" | "expiry_seconds"
            ) {
                saw_lifecycle_patch = true;
            }
            if matches!(
                key.as_str(),
                "mission"
                    | "owner"
                    | "termination_condition"
                    | "expires_at"
                    | "status"
                    | "termination_reason"
                    | "created_at"
            ) {
                contract[key] = value.clone();
            }
            if key == "status" {
                saw_status_patch = true;
            }
            if key == "expiry_seconds" {
                contract["expiry_seconds"] = Value::from(parse_expiry_seconds(Some(value)));
            }
            if key == "auto_terminate_allowed" {
                saw_lifecycle_patch = true;
                contract["auto_terminate_allowed"] = Value::Bool(value.as_bool().unwrap_or(true));
            }
        }
    }
    let existing_status = contract
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("active");
    if !saw_status_patch && saw_lifecycle_patch && existing_status == "terminated" {
        contract["status"] = Value::String("active".to_string());
        contract["created_at"] = Value::String(now_iso());
        contract["updated_at"] = Value::String(now_iso());
        if !patch
            .as_object()
            .map(|obj| obj.contains_key("expires_at"))
            .unwrap_or(false)
        {
            contract["expires_at"] = Value::String(String::new());
        }
        if let Some(obj) = contract.as_object_mut() {
            obj.remove("terminated_at");
            obj.remove("termination_reason");
        }
    }
    if contract.get("expiry_seconds").is_none() {
        contract["expiry_seconds"] = Value::from(DEFAULT_EXPIRY_SECONDS);
    }
    if contract
        .get("created_at")
        .and_then(Value::as_str)
        .map(|v| v.is_empty())
        .unwrap_or(true)
    {
        contract["created_at"] = Value::String(now_iso());
    }
    contract["updated_at"] = Value::String(now_iso());
    contracts.insert(id.clone(), contract.clone());
    save_contracts_state(root, state);
    json!({"ok": true, "type": "dashboard_agent_contract", "agent_id": id, "contract": contract})
}

pub fn enforce_expired_contracts(root: &Path) -> Value {
    let mut state = load_contracts_state(root);
    let now = Utc::now();
    let mut terminated = Vec::<Value>::new();
    {
        let contracts = as_object_mut(&mut state, "contracts");
        for (agent_id, contract) in contracts.iter_mut() {
            let status = contract.get("status").and_then(Value::as_str).unwrap_or("active");
            if status != "active" {
                continue;
            }
            if !contract
                .get("auto_terminate_allowed")
                .and_then(Value::as_bool)
                .unwrap_or(true)
            {
                continue;
            }
            let expires_at = contract.get("expires_at").and_then(Value::as_str).unwrap_or("");
            let created_at = contract
                .get("created_at")
                .and_then(Value::as_str)
                .unwrap_or("");
            let expiry_seconds = parse_expiry_seconds(contract.get("expiry_seconds"));
            let created_ts = parse_ts(created_at).unwrap_or(now);
            let expiry_ts = if let Some(ts) = parse_ts(expires_at) {
                ts
            } else {
                created_ts + chrono::Duration::seconds(expiry_seconds)
            };
            if now >= expiry_ts {
                contract["status"] = Value::String("terminated".to_string());
                contract["termination_reason"] = Value::String("contract_expired".to_string());
                contract["terminated_at"] = Value::String(now_iso());
                contract["updated_at"] = Value::String(now_iso());
                let row = json!({
                    "agent_id": agent_id,
                    "contract_id": contract
                        .get("contract_id")
                        .cloned()
                        .unwrap_or(Value::String(String::new())),
                    "termination_reason": "contract_expired",
                    "terminated_at": contract
                        .get("terminated_at")
                        .cloned()
                        .unwrap_or(Value::String(now_iso()))
                });
                terminated.push(row);
            }
        }
    }
    {
        let history = as_array_mut(&mut state, "terminated_history");
        for row in &terminated {
            history.push(row.clone());
        }
    }
    save_contracts_state(root, state);
    json!({"ok": true, "type": "dashboard_contract_enforcement", "terminated": terminated})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_contracts_terminate() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = upsert_contract(
            root.path(),
            "agent-a",
            &json!({
                "created_at": "2000-01-01T00:00:00Z",
                "expiry_seconds": 1,
                "status": "active"
            }),
        );
        let out = enforce_expired_contracts(root.path());
        let terminated = out
            .get("terminated")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!terminated.is_empty());
    }

    #[test]
    fn upsert_lifecycle_reactivates_terminated_contract() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = upsert_contract(
            root.path(),
            "agent-revive",
            &json!({
                "created_at": "2000-01-01T00:00:00Z",
                "expiry_seconds": 1,
                "status": "active"
            }),
        );
        let terminated = enforce_expired_contracts(root.path());
        assert!(
            !terminated
                .get("terminated")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .is_empty()
        );

        let reupsert = upsert_contract(
            root.path(),
            "agent-revive",
            &json!({
                "mission": "restart",
                "expiry_seconds": 3600,
                "auto_terminate_allowed": true
            }),
        );
        assert_eq!(
            reupsert
                .get("contract")
                .and_then(|v| v.get("status"))
                .and_then(Value::as_str),
            Some("active")
        );
        assert!(
            reupsert
                .get("contract")
                .and_then(Value::as_object)
                .map(|obj| !obj.contains_key("terminated_at"))
                .unwrap_or(false)
        );
        let after = enforce_expired_contracts(root.path());
        let rows = after
            .get("terminated")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.is_empty());
    }
}
