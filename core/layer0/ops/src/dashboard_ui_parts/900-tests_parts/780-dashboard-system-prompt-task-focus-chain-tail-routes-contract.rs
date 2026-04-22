#[test]
fn dashboard_system_prompt_task_focus_chain_tail_routes_contract_wave_780() {
    let root = tempfile::tempdir().expect("tempdir");

    let file_utils = run_action(
        root.path(),
        "dashboard.prompts.system.task.focusChain.fileUtils.describe",
        &json!({"utility": "path_normalize"}),
    );
    assert!(file_utils.ok);
    assert_eq!(
        file_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("utility")
            .and_then(Value::as_str),
        Some("path_normalize")
    );

    let focus_chain_index = run_action(
        root.path(),
        "dashboard.prompts.system.task.focusChain.index.describe",
        &json!({"index_scope": "focus_chain"}),
    );
    assert!(focus_chain_index.ok);
    assert_eq!(
        focus_chain_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("index_scope")
            .and_then(Value::as_str),
        Some("focus_chain")
    );

    let focus_chain_prompts = run_action(
        root.path(),
        "dashboard.prompts.system.task.focusChain.prompts.describe",
        &json!({"prompt_profile": "default"}),
    );
    assert!(focus_chain_prompts.ok);
    assert_eq!(
        focus_chain_prompts
            .payload
            .unwrap_or_else(|| json!({}))
            .get("prompt_profile")
            .and_then(Value::as_str),
        Some("default")
    );

    let focus_chain_utils = run_action(
        root.path(),
        "dashboard.prompts.system.task.focusChain.utils.describe",
        &json!({"helper": "focus_chain_utils"}),
    );
    assert!(focus_chain_utils.ok);
    assert_eq!(
        focus_chain_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("helper")
            .and_then(Value::as_str),
        Some("focus_chain_utils")
    );

    let task_index = run_action(
        root.path(),
        "dashboard.prompts.system.task.index.describe",
        &json!({"index_scope": "task"}),
    );
    assert!(task_index.ok);
    assert_eq!(
        task_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("index_scope")
            .and_then(Value::as_str),
        Some("task")
    );

    let task_latency = run_action(
        root.path(),
        "dashboard.prompts.system.task.latency.describe",
        &json!({"latency_profile": "balanced"}),
    );
    assert!(task_latency.ok);
    assert_eq!(
        task_latency
            .payload
            .unwrap_or_else(|| json!({}))
            .get("latency_profile")
            .and_then(Value::as_str),
        Some("balanced")
    );

    let loop_detection = run_action(
        root.path(),
        "dashboard.prompts.system.task.loopDetection.describe",
        &json!({"detection_mode": "strict"}),
    );
    assert!(loop_detection.ok);
    assert_eq!(
        loop_detection
            .payload
            .unwrap_or_else(|| json!({}))
            .get("detection_mode")
            .and_then(Value::as_str),
        Some("strict")
    );

    let message_state = run_action(
        root.path(),
        "dashboard.prompts.system.task.messageState.describe",
        &json!({"message_state": "active"}),
    );
    assert!(message_state.ok);
    assert_eq!(
        message_state
            .payload
            .unwrap_or_else(|| json!({}))
            .get("message_state")
            .and_then(Value::as_str),
        Some("active")
    );

    let multifile_diff = run_action(
        root.path(),
        "dashboard.prompts.system.task.multifileDiff.describe",
        &json!({"diff_mode": "summary"}),
    );
    assert!(multifile_diff.ok);
    assert_eq!(
        multifile_diff
            .payload
            .unwrap_or_else(|| json!({}))
            .get("diff_mode")
            .and_then(Value::as_str),
        Some("summary")
    );

    let presentation_types = run_action(
        root.path(),
        "dashboard.prompts.system.task.presentationTypes.describe",
        &json!({"presentation_type": "default"}),
    );
    assert!(presentation_types.ok);
    assert_eq!(
        presentation_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("presentation_type")
            .and_then(Value::as_str),
        Some("default")
    );
}
