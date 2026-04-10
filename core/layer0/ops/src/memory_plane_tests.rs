// SPDX-License-Identifier: Apache-2.0
use super::*;
use serde_json::{json, Value};

fn args(rows: &[&str]) -> Vec<String> {
    rows.iter().map(|row| row.to_string()).collect()
}

fn first_entry_value(entries: &[Value], key: &str) -> Option<i64> {
    entries
        .first()
        .and_then(|row| row.get("entry"))
        .and_then(|entry| entry.get("value"))
        .and_then(|value| value.get(key))
        .and_then(Value::as_i64)
}

#[test]
fn causal_graph_record_and_blame_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let policy = default_policy();
    graph_record_payload(
        dir.path(),
        &policy,
        &args(&[
            "--event-id=e1",
            "--summary=root",
            "--actor=planner",
            "--apply=1",
        ]),
    )
    .expect("record root");
    graph_record_payload(
        dir.path(),
        &policy,
        &args(&[
            "--event-id=e2",
            "--summary=child",
            "--actor=executor",
            "--caused-by=e1",
            "--apply=1",
        ]),
    )
    .expect("record child");

    let blame = graph_blame_payload(dir.path(), &["--event-id=e2".to_string()]).expect("blame");
    let ancestry = blame
        .get("ancestry")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!ancestry.is_empty());
    assert_eq!(
        ancestry[0]
            .get("event_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        "e1"
    );
}

#[test]
fn federation_sync_resolves_with_vector_counter() {
    let dir = tempfile::tempdir().expect("tempdir");
    let policy = default_policy();
    let sync1 = federation_sync_payload(
        dir.path(),
        &policy,
        &args(&[
            "--device-id=d1",
            "--entries-json=[{\"key\":\"k\",\"value\":{\"v\":1},\"counter\":1}]",
            "--apply=1",
        ]),
    )
    .expect("sync1");
    assert_eq!(sync1.get("accepted").and_then(Value::as_u64), Some(1));

    let sync2 = federation_sync_payload(
        dir.path(),
        &policy,
        &args(&[
            "--device-id=d1",
            "--entries-json=[{\"key\":\"k\",\"value\":{\"v\":2},\"counter\":2}]",
            "--apply=1",
        ]),
    )
    .expect("sync2");
    assert_eq!(sync2.get("replaced").and_then(Value::as_u64), Some(1));

    let pull = federation_pull_payload(dir.path(), &[]);
    let entries = pull
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let value = first_entry_value(&entries, "v");
    assert_eq!(value, Some(2));
}

#[test]
fn unified_heap_status_exposes_governance_matrices() {
    let payload = unified_heap_status_payload();
    assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        payload.get("type").and_then(Value::as_str),
        Some("unified_memory_heap_status")
    );
    assert!(payload
        .get("receipt_hash")
        .and_then(Value::as_str)
        .is_some());
    let matrices = payload
        .get("matrices")
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert!(matrices
        .get("scope_authority")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
    assert!(matrices
        .get("trust_state_transition")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
    assert!(matrices
        .get("owner_export_redaction")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
    assert!(matrices
        .get("task_fabric_lease_cas")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
}
