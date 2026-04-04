use super::*;
use chrono::{Duration, Utc};
use std::fs;

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
    assert!(!terminated
        .get("terminated")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .is_empty());

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
    assert!(reupsert
        .get("contract")
        .and_then(Value::as_object)
        .map(|obj| !obj.contains_key("terminated_at"))
        .unwrap_or(false));
    let after = enforce_expired_contracts(root.path());
    let rows = after
        .get("terminated")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(rows.is_empty());
}

#[test]
fn terminated_entries_delete_and_revive_round_trip() {
    let root = tempfile::tempdir().expect("tempdir");
    let _ = upsert_contract(
        root.path(),
        "agent-zed",
        &json!({
            "created_at": "2000-01-01T00:00:00Z",
            "expiry_seconds": 1,
            "status": "active"
        }),
    );
    let _ = enforce_expired_contracts(root.path());
    let list = terminated_entries(root.path());
    let before = list
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(before.iter().any(|row| {
        row.get("agent_id")
            .and_then(Value::as_str)
            .map(|v| v == "agent-zed")
            .unwrap_or(false)
    }));

    let revived = revive_agent(root.path(), "agent-zed", "analyst");
    assert_eq!(revived.get("ok").and_then(Value::as_bool), Some(true));
    let list_after_revive = terminated_entries(root.path());
    let revived_rows = list_after_revive
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!revived_rows.iter().any(|row| {
        row.get("agent_id")
            .and_then(Value::as_str)
            .map(|v| v == "agent-zed")
            .unwrap_or(false)
    }));

    let _ = upsert_contract(
        root.path(),
        "agent-zed",
        &json!({
            "created_at": "2000-01-01T00:00:00Z",
            "expiry_seconds": 1,
            "status": "active"
        }),
    );
    let _ = enforce_expired_contracts(root.path());
    let deleted = delete_terminated(root.path(), "agent-zed", None);
    assert!(
        deleted
            .get("removed_history_entries")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 1
    );
}

#[test]
fn idle_contracts_terminate_even_when_expiry_auto_terminate_is_disabled() {
    let root = tempfile::tempdir().expect("tempdir");
    let created_at = (Utc::now() - Duration::hours(6)).to_rfc3339();
    let _ = upsert_contract(
        root.path(),
        "agent-idle",
        &json!({
            "created_at": created_at,
            "expiry_seconds": 31 * 24 * 60 * 60,
            "auto_terminate_allowed": false,
            "idle_timeout_seconds": 120,
            "idle_terminate_allowed": true,
            "status": "active"
        }),
    );

    let session = root
        .path()
        .join(AGENT_SESSIONS_DIR_REL)
        .join("agent-idle.json");
    if let Some(parent) = session.parent() {
        fs::create_dir_all(parent).expect("mkdir sessions");
    }
    fs::write(
        &session,
        serde_json::to_string_pretty(&json!({
            "type": "infring_dashboard_agent_session",
            "agent_id": "agent-idle",
            "active_session_id": "default",
            "sessions": [{
                "session_id": "default",
                "created_at": created_at,
                "updated_at": (Utc::now() - Duration::hours(2)).to_rfc3339(),
                "messages": [{
                    "role": "assistant",
                    "text": "stale",
                    "ts": (Utc::now() - Duration::hours(2)).to_rfc3339()
                }]
            }],
            "memory_kv": {}
        }))
        .expect("json"),
    )
    .expect("write session");

    let out = enforce_expired_contracts(root.path());
    let terminated = out
        .get("terminated")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(terminated.iter().any(|row| {
        row.get("agent_id")
            .and_then(Value::as_str)
            .map(|id| id == "agent-idle")
            .unwrap_or(false)
            && row
                .get("termination_reason")
                .and_then(Value::as_str)
                .map(|reason| reason == "idle_timeout")
                .unwrap_or(false)
    }));
}

#[test]
fn idle_termination_can_be_disabled_per_contract() {
    let root = tempfile::tempdir().expect("tempdir");
    let created_at = (Utc::now() - Duration::hours(6)).to_rfc3339();
    let _ = upsert_contract(
        root.path(),
        "agent-no-idle-kill",
        &json!({
            "created_at": created_at,
            "expiry_seconds": 31 * 24 * 60 * 60,
            "auto_terminate_allowed": false,
            "idle_timeout_seconds": 120,
            "idle_terminate_allowed": false,
            "status": "active"
        }),
    );

    let out = enforce_expired_contracts(root.path());
    let terminated = out
        .get("terminated")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(terminated.is_empty());
}

#[test]
fn manual_termination_condition_disables_auto_and_idle_termination_for_legacy_contracts() {
    let root = tempfile::tempdir().expect("tempdir");
    let created_at = (Utc::now() - Duration::hours(6)).to_rfc3339();
    let path = root.path().join(AGENT_CONTRACTS_REL);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir contracts");
    }
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "type": "infring_dashboard_agent_contracts",
            "updated_at": Utc::now().to_rfc3339(),
            "contracts": {
                "agent-permanent": {
                    "agent_id": "agent-permanent",
                    "contract_id": "contract-permanent",
                    "status": "active",
                    "created_at": created_at,
                    "updated_at": created_at,
                    "termination_condition": "manual",
                    "expiry_seconds": 1,
                    "auto_terminate_allowed": true,
                    "idle_timeout_seconds": 120,
                    "idle_terminate_allowed": true
                }
            },
            "terminated_history": []
        }))
        .expect("json"),
    )
    .expect("write contract state");

    let out = enforce_expired_contracts(root.path());
    let terminated = out
        .get("terminated")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(terminated.is_empty());
}

#[test]
fn indefinite_contract_patch_forces_manual_non_expiring_behavior() {
    let root = tempfile::tempdir().expect("tempdir");
    let created_at = (Utc::now() - Duration::hours(6)).to_rfc3339();
    let _ = upsert_contract(
        root.path(),
        "agent-indefinite",
        &json!({
            "created_at": created_at,
            "status": "active",
            "termination_condition": "task_or_timeout",
            "expiry_seconds": 1,
            "auto_terminate_allowed": true,
            "idle_timeout_seconds": 30,
            "idle_terminate_allowed": true,
            "indefinite": true
        }),
    );
    let out = enforce_expired_contracts(root.path());
    let terminated = out
        .get("terminated")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(terminated.is_empty());
    let contracts = read_json_file(&root.path().join(AGENT_CONTRACTS_REL)).unwrap_or_else(|| json!({}));
    let row = contracts
        .pointer("/contracts/agent-indefinite")
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert_eq!(
        row.get("termination_condition").and_then(Value::as_str),
        Some("manual")
    );
    assert_eq!(
        row.get("auto_terminate_allowed").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        row.get("idle_terminate_allowed").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn revive_preserves_non_expiring_permanent_lifecycle() {
    let root = tempfile::tempdir().expect("tempdir");
    let created_at = (Utc::now() - Duration::hours(4)).to_rfc3339();
    let _ = upsert_contract(
        root.path(),
        "agent-perma-revive",
        &json!({
            "created_at": created_at,
            "status": "terminated",
            "termination_reason": "idle_timeout",
            "termination_condition": "manual",
            "indefinite": true,
            "lifespan": "permanent",
            "expiry_seconds": 7200,
            "auto_terminate_allowed": false,
            "idle_timeout_seconds": 600,
            "idle_terminate_allowed": false
        }),
    );
    let revived = revive_agent(root.path(), "agent-perma-revive", "analyst");
    assert_eq!(revived.get("ok").and_then(Value::as_bool), Some(true));
    let contracts = read_json_file(&root.path().join(AGENT_CONTRACTS_REL)).unwrap_or_else(|| json!({}));
    let row = contracts
        .pointer("/contracts/agent-perma-revive")
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert_eq!(row.get("status").and_then(Value::as_str), Some("active"));
    assert_eq!(
        row.get("termination_condition").and_then(Value::as_str),
        Some("manual")
    );
    assert_eq!(row.get("indefinite").and_then(Value::as_bool), Some(true));
    assert_eq!(
        row.get("lifespan").and_then(Value::as_str),
        Some("permanent")
    );
    assert_eq!(
        row.get("auto_terminate_allowed").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        row.get("idle_terminate_allowed").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        row.get("expires_at").and_then(Value::as_str).unwrap_or(""),
        ""
    );
    assert!(row.get("terminated_at").is_none());
}
