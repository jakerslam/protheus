#[test]
fn dashboard_system_prompt_focus_chain_routes_contract_wave_310() {
    let root = tempfile::tempdir().expect("tempdir");

    let inspect = run_action(
        root.path(),
        "dashboard.prompts.system.task.focusChain.fileUtils.inspect",
        &json!({"files": ["src/a.ts", "src/b.rs", "README"]}),
    );
    assert!(inspect.ok);
    let inspect_payload = inspect.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        inspect_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_task_focus_chain_file_utils_inspect")
    );
    assert_eq!(inspect_payload.get("file_count").and_then(Value::as_i64), Some(3));

    let compose = run_action(
        root.path(),
        "dashboard.prompts.system.task.focusChain.prompts.compose",
        &json!({
            "objective": "Preserve runtime authority",
            "constraints": ["fail closed", "no shell authority"]
        }),
    );
    assert!(compose.ok);
    let compose_payload = compose.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        compose_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_task_focus_chain_prompts_compose")
    );
    assert!(
        compose_payload
            .get("prompt_text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("fail closed")
    );

    let normalize = run_action(
        root.path(),
        "dashboard.prompts.system.task.focusChain.utils.normalize",
        &json!({"chain": ["B", "a", "b", "A"]}),
    );
    assert!(normalize.ok);
    let normalize_payload = normalize.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        normalize_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_task_focus_chain_utils_normalize")
    );
    assert_eq!(
        normalize_payload
            .get("normalized_chain")
            .and_then(Value::as_array)
            .map(|rows| rows.len() as i64),
        Some(2)
    );
}

#[test]
fn dashboard_system_prompt_task_analysis_routes_contract_wave_310() {
    let root = tempfile::tempdir().expect("tempdir");

    let latency = run_action(
        root.path(),
        "dashboard.prompts.system.task.latency.estimate",
        &json!({"model_ms": 30, "tool_ms": 20, "steps": 3}),
    );
    assert!(latency.ok);
    let latency_payload = latency.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        latency_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_task_latency_estimate")
    );
    assert_eq!(
        latency_payload.get("estimated_total_ms").and_then(Value::as_i64),
        Some(150)
    );

    let loops = run_action(
        root.path(),
        "dashboard.prompts.system.task.loopDetection.analyze",
        &json!({"sequence": ["a", "a", "b", "b", "c"]}),
    );
    assert!(loops.ok);
    let loops_payload = loops.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        loops_payload.get("repeated_edges").and_then(Value::as_i64),
        Some(2)
    );
    assert_eq!(
        loops_payload.get("has_loop_signal").and_then(Value::as_bool),
        Some(true)
    );

    let diff_plan = run_action(
        root.path(),
        "dashboard.prompts.system.task.multifileDiff.plan",
        &json!({
            "files": [
                {"path": "src/a.ts", "change": "modify"},
                {"path": "src/b.rs", "change": "create"}
            ]
        }),
    );
    assert!(diff_plan.ok);
    let diff_payload = diff_plan.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        diff_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_task_multifile_diff_plan")
    );
    assert_eq!(diff_payload.get("count").and_then(Value::as_i64), Some(2));

    let presentation = run_action(
        root.path(),
        "dashboard.prompts.system.task.presentationTypes.describe",
        &json!({"presentation_type": "timeline"}),
    );
    assert!(presentation.ok);
    let presentation_payload = presentation.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        presentation_payload.get("known").and_then(Value::as_bool),
        Some(true)
    );
}
