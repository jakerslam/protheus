// SRS: V12-LIVE-EVAL-MONITOR-001
#[test]
fn live_eval_monitor_is_default_on_and_can_be_disabled_without_changing_chat() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"live-eval-monitor-agent","role":"assistant"}"#,
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
    assert!(!agent_id.is_empty());

    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [{"response": "Live monitor is watching, but not authoring chat."}], "calls": []}),
    );
    let monitored = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Confirm the live monitor switch."}"#,
        &snapshot,
    )
    .expect("monitored response");
    assert_eq!(
        monitored.payload.get("response").and_then(Value::as_str),
        Some("Live monitor is watching, but not authoring chat.")
    );
    assert_eq!(
        monitored
            .payload
            .pointer("/live_eval_monitor/enabled")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        monitored
            .payload
            .pointer("/live_eval_monitor/chat_injection_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        monitored
            .payload
            .pointer("/live_eval_monitor/issue_count")
            .and_then(Value::as_u64),
        Some(0)
    );

    write_json(
        &root
            .path()
            .join("local/state/ops/eval_live_monitor/config.json"),
        &json!({"enabled": false}),
    );
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [{"response": "Monitor disabled; chat still belongs to the LLM."}], "calls": []}),
    );
    let disabled = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Confirm the disabled live monitor switch."}"#,
        &snapshot,
    )
    .expect("disabled monitor response");
    assert_eq!(
        disabled.payload.get("response").and_then(Value::as_str),
        Some("Monitor disabled; chat still belongs to the LLM.")
    );
    assert_eq!(
        disabled
            .payload
            .pointer("/live_eval_monitor/enabled")
            .and_then(Value::as_bool),
        Some(false)
    );
}
