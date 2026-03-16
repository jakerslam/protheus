// SPDX-License-Identifier: Apache-2.0
use protheus_ops_core::swarm_runtime;
use serde_json::Value;
use std::fs;

const SWARM_CONTRACT_IDS: &[&str] = &[
    "V6-SWARM-013",
    "V6-SWARM-014",
    "V6-SWARM-015",
    "V6-SWARM-016",
    "V6-SWARM-017",
    "V6-SWARM-018",
    "V6-SWARM-019",
];

fn run_swarm(root: &std::path::Path, args: &[String]) -> i32 {
    swarm_runtime::run(root, args)
}

fn read_state(path: &std::path::Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("read state")).expect("parse state")
}

#[test]
fn swarm_contract_ids_are_embedded_for_receipt_audit_evidence() {
    assert_eq!(SWARM_CONTRACT_IDS.len(), 7);
    assert!(SWARM_CONTRACT_IDS
        .iter()
        .all(|id| id.starts_with("V6-SWARM-0")));
}

#[test]
fn recursive_test_reaches_five_levels_with_parent_child_chain() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let args = vec![
        "test".to_string(),
        "recursive".to_string(),
        "--levels=5".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    let exit = run_swarm(root.path(), &args);
    assert_eq!(exit, 0, "recursive test command should succeed");

    let state = read_state(&state_path);
    let sessions = state
        .get("sessions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    assert_eq!(sessions.len(), 5, "expected 5 sessions for 5 levels");

    let max_depth = sessions
        .values()
        .filter_map(|session| session.get("depth").and_then(Value::as_u64))
        .max()
        .unwrap_or(0);
    assert_eq!(max_depth, 4);

    let with_parent = sessions
        .values()
        .filter(|session| {
            session
                .get("parent_id")
                .and_then(Value::as_str)
                .map(|id| !id.is_empty())
                .unwrap_or(false)
        })
        .count();
    assert_eq!(
        with_parent, 4,
        "all non-root sessions should have parent IDs"
    );
}

#[test]
fn byzantine_test_mode_enables_corrupted_reports() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let enable_args = vec![
        "byzantine-test".to_string(),
        "enable".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &enable_args), 0);

    let spawn_args = vec![
        "spawn".to_string(),
        "--task=swarm-test-3".to_string(),
        "--byzantine=1".to_string(),
        "--verify=1".to_string(),
        "--corruption-type=data_falsification".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &spawn_args), 0);

    let consensus_args = vec![
        "consensus-check".to_string(),
        "--task-id=swarm-test-3".to_string(),
        "--threshold=0.6".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &consensus_args), 0);

    let state = read_state(&state_path);
    assert_eq!(
        state
            .get("byzantine_test_mode")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        true,
        "expected byzantine mode enabled",
    );

    let sessions = state
        .get("sessions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let corrupted = sessions.values().any(|session| {
        session
            .get("report")
            .and_then(|value| value.get("corrupted"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    assert!(corrupted, "expected corrupted report in byzantine mode");
}

#[test]
fn concurrency_test_persists_detailed_spawn_metrics() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let args = vec![
        "test".to_string(),
        "concurrency".to_string(),
        "--agents=10".to_string(),
        "--metrics=detailed".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &args), 0);

    let state = read_state(&state_path);
    let sessions = state
        .get("sessions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    assert!(
        sessions.len() >= 10,
        "expected at least 10 sessions from concurrency test"
    );

    let metrics_complete = sessions.values().all(|session| {
        let Some(metrics) = session.get("metrics") else {
            return false;
        };
        metrics.get("queue_wait_ms").is_some()
            && metrics.get("execution_end_ms").is_some()
            && metrics.get("report_back_latency_ms").is_some()
    });
    assert!(
        metrics_complete,
        "expected detailed metrics on all sessions"
    );
}

#[test]
fn budget_enforcement_fail_hard_blocks_overrun() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");
    let args = vec![
        "spawn".to_string(),
        "--task=Write detailed exhaustive analysis with many references and examples".to_string(),
        "--token-budget=120".to_string(),
        "--on-budget-exhausted=fail".to_string(),
        "--adaptive-complexity=0".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    let exit = run_swarm(root.path(), &args);
    assert_eq!(exit, 2, "budget-overrun spawn should fail hard");

    let state = read_state(&state_path);
    let sessions = state
        .get("sessions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    assert_eq!(sessions.len(), 1, "expected failed session to be recorded");
    let exhausted = sessions.values().any(|session| {
        session
            .get("budget_telemetry")
            .and_then(|value| value.get("budget_exhausted"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    assert!(exhausted, "expected budget exhaustion in telemetry");
}

#[test]
fn budget_test_and_budget_report_emit_telemetry() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let test_args = vec![
        "test".to_string(),
        "budget".to_string(),
        "--budget=2000".to_string(),
        "--warning-at=0.5".to_string(),
        "--on-budget-exhausted=warn".to_string(),
        "--task=Read SOUL.md and summarize in three sentences".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &test_args), 0);

    let state = read_state(&state_path);
    let sessions = state
        .get("sessions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let session_id = sessions
        .keys()
        .next()
        .cloned()
        .expect("session id should exist");

    let report_args = vec![
        "budget-report".to_string(),
        format!("--session-id={session_id}"),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &report_args), 0);

    let telemetry_present = sessions.values().any(|session| {
        session
            .get("budget_telemetry")
            .and_then(|value| value.get("tool_breakdown"))
            .and_then(Value::as_object)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
    });
    assert!(
        telemetry_present,
        "expected per-tool budget telemetry to be persisted"
    );
}
