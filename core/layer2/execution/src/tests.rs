use super::*;

fn sample_yaml() -> String {
    serde_json::json!({
        "workflow_id": "phase2_parity_demo",
        "deterministic_seed": "seed_a",
        "steps": [
            { "id": "collect", "kind": "task", "action": "collect_data", "command": "collect --source=eyes" },
            { "id": "score", "kind": "task", "action": "score", "command": "score --strategy=deterministic" },
            { "id": "ship", "kind": "task", "action": "ship", "command": "ship --mode=canary" }
        ]
    })
    .to_string()
}

#[test]
fn deterministic_replay_is_stable() {
    let yaml = sample_yaml();
    let a = run_workflow(&yaml);
    let b = run_workflow(&yaml);
    assert_eq!(a, b);
    assert_eq!(a.status, "completed");
}

#[test]
fn pause_and_resume_cycle() {
    let paused_yaml = serde_json::json!({
        "workflow_id": "pause_resume_demo",
        "deterministic_seed": "seed_b",
        "pause_after_step": "score",
        "steps": [
            { "id": "collect", "kind": "task", "action": "collect_data", "command": "collect --source=eyes" },
            { "id": "score", "kind": "task", "action": "score", "command": "score --strategy=deterministic" },
            { "id": "ship", "kind": "task", "action": "ship", "command": "ship --mode=canary" }
        ]
    })
    .to_string();
    let paused = run_workflow(&paused_yaml);
    assert_eq!(paused.status, "paused");
    assert_eq!(paused.state.cursor, 2);

    let resumed_yaml = serde_json::json!({
        "workflow_id": "pause_resume_demo",
        "deterministic_seed": "seed_b",
        "resume": paused.state,
        "steps": [
            { "id": "collect", "kind": "task", "action": "collect_data", "command": "collect --source=eyes" },
            { "id": "score", "kind": "task", "action": "score", "command": "score --strategy=deterministic" },
            { "id": "ship", "kind": "task", "action": "ship", "command": "ship --mode=canary" }
        ]
    })
    .to_string();
    let resumed = run_workflow(&resumed_yaml);
    assert_eq!(resumed.status, "completed");
    assert_eq!(resumed.state.cursor, 3);
}

#[test]
fn parse_failure_returns_failed_receipt() {
    let receipt = run_workflow("workflow_id: [invalid");
    assert_eq!(receipt.status, "failed");
    assert_eq!(receipt.workflow_id, "invalid_workflow");
}

#[test]
fn ffi_roundtrip_returns_json_receipt() {
    let yaml = CString::new(sample_yaml()).unwrap();
    let out_ptr = unsafe { run_workflow_ffi(yaml.as_ptr()) };
    assert!(!out_ptr.is_null());
    let text = unsafe { CStr::from_ptr(out_ptr) }
        .to_str()
        .unwrap()
        .to_string();
    unsafe { execution_core_string_free(out_ptr) };
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["status"], "completed");
    assert_eq!(parsed["workflow_id"], "phase2_parity_demo");
}

#[test]
fn ffi_null_pointer_returns_failed_payload() {
    let out_ptr = unsafe { run_workflow_ffi(std::ptr::null()) };
    assert!(!out_ptr.is_null());
    let text = unsafe { CStr::from_ptr(out_ptr) }
        .to_str()
        .unwrap()
        .to_string();
    unsafe { execution_core_string_free(out_ptr) };
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["status"], "failed");
}
