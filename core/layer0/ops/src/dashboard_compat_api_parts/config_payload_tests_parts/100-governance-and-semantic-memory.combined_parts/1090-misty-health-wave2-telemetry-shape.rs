// SRS: V12-MISTY-HEALTH-WAVE2-001

fn assert_misty_wave2_payload_contract(payload: &Value) {
    assert_eq!(
        payload
            .pointer("/workflow_visibility/contract")
            .and_then(Value::as_str),
        Some("workflow_visibility_payload_v1")
    );
    assert_eq!(
        payload
            .pointer("/workflow_visibility/chat_injection_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/workflow_visibility/system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/response_workflow/visibility/system_injected_chat_text_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/process_summary/workflow_visibility/contract")
            .and_then(Value::as_str),
        Some("workflow_visibility_payload_v1")
    );
    assert_eq!(
        payload
            .pointer("/process_summary/system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/live_eval_monitor/chat_injection_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    let response = payload.get("response").and_then(Value::as_str).unwrap_or("");
    for forbidden in [
        "workflow finalization edge",
        "workflow_route",
        "task_or_info_route",
        "chat_injection_allowed",
        "workflow_visibility_payload_v1",
    ] {
        assert!(
            !response.contains(forbidden),
            "visible response leaked telemetry marker {forbidden:?}: {response}"
        );
    }
}

#[test]
fn misty_wave2_payload_exposes_diagnostics_without_chat_injection() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave2-telemetry-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [{"response": "The workflow status is visible in diagnostics, while this chat text stays mine."}],
            "calls": []
        }),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"hey"}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("The workflow status is visible in diagnostics, while this chat text stays mine.")
    );
    assert_misty_wave2_payload_contract(&response.payload);
}

#[test]
fn misty_wave2_disabled_live_eval_monitor_still_blocks_chat_injection() {
    let root = governance_temp_root();
    let config_dir = root.path().join("local/state/ops/eval_live_monitor");
    std::fs::create_dir_all(&config_dir).expect("live eval config dir");
    write_json(&config_dir.join("config.json"), &json!({"enabled": false}));
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave2-disabled-monitor-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [{"response": "Monitor disabled is still telemetry-only, not chat control."}],
            "calls": []
        }),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"answer normally without tools"}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/live_eval_monitor/enabled")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_misty_wave2_payload_contract(&response.payload);
}
