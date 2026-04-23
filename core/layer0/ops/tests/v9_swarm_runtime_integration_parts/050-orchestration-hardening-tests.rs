use infring_ops_core::orchestration;
use serde_json::json;

fn run_orchestration(root: &std::path::Path, op: &str, payload: serde_json::Value) -> i32 {
    let args = vec![
        "invoke".to_string(),
        format!("--op={op}"),
        format!(
            "--payload-json={}",
            serde_json::to_string(&payload).expect("payload")
        ),
    ];
    orchestration::run(root, &args)
}

#[test]
fn orchestration_append_finding_dedupes_retry_payloads() {
    let root = tempfile::tempdir().expect("tempdir");
    let scratchpad_dir = root.path().join("orchestration-scratchpad");

    let payload = json!({
        "task_id": "swarm-orch-001",
        "finding": {
            "audit_id": "audit-001",
            "item_id": "item-001",
            "severity": "high",
            "status": "open",
            "location": "core/layer0/ops/src/swarm_runtime.rs:10",
            "evidence": [{ "type": "receipt", "value": "same" }],
            "timestamp": "2026-04-11T00:00:00Z"
        },
        "root_dir": scratchpad_dir
    });

    assert_eq!(
        run_orchestration(root.path(), "scratchpad.append_finding", payload.clone()),
        0
    );
    assert_eq!(
        run_orchestration(root.path(), "scratchpad.append_finding", payload),
        0
    );

    let stored = read_state(&scratchpad_dir.join("swarm-orch-001.json"));
    assert_eq!(
        stored
            .get("findings")
            .and_then(serde_json::Value::as_array)
            .map(|rows| rows.len()),
        Some(1)
    );
}

#[test]
fn orchestration_rejects_duplicate_scope_ids_before_taskgroup_creation() {
    let root = tempfile::tempdir().expect("tempdir");
    let scratchpad_dir = root.path().join("orchestration-scopes");

    let code = run_orchestration(
        root.path(),
        "coordinator.run",
        json!({
            "task_id": "scope-dup-001",
            "task_type": "swarm-audit",
            "agent_count": 2,
            "items": ["V6-SWARM-013"],
            "findings": [],
            "scopes": [
                { "scope_id": "scope-dup", "series": ["V6-SWARM"], "paths": ["core/*"] },
                { "scope_id": "scope-dup", "series": ["V6-SWARM"], "paths": ["tests/*"] }
            ],
            "root_dir": scratchpad_dir
        }),
    );

    assert_eq!(code, 1, "duplicate scope ids should fail closed");
    let entries = std::fs::read_dir(&scratchpad_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    assert!(
        entries.is_empty(),
        "taskgroup artifacts should not be created when scope ids collide"
    );
}
