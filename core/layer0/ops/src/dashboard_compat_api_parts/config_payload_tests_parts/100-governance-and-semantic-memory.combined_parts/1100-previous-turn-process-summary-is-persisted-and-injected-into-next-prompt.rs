fn previous_turn_process_summary_is_persisted_and_injected_into_next_prompt() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"process-summary-memory-agent","role":"assistant"}"#,
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
        &json!({
            "queue": [
                {"response": "First turn answer."},
                {"response": "Second turn answer."}
            ],
            "calls": []
        }),
    );
    let first = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Give me a short direct answer."}"#,
        &snapshot,
    )
    .expect("first message response");
    assert_eq!(first.status, 200);
    assert_eq!(
        first.payload
            .pointer("/process_summary/contract")
            .and_then(Value::as_str),
        Some("turn_process_summary_v1")
    );
    let second = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Now continue from that."}"#,
        &snapshot,
    )
    .expect("second message response");
    assert_eq!(second.status, 200);
    assert_eq!(
        second
            .payload
            .pointer("/process_summary/contract")
            .and_then(Value::as_str),
        Some("turn_process_summary_v1")
    );
    let chat_calls = read_json(&governance_test_chat_script_path(root.path()))
        .and_then(|value| value.get("calls").cloned())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    assert!(chat_calls.len() >= 2, "{chat_calls:?}");
    let second_system_prompt = clean_text(
        chat_calls[1]
            .get("system_prompt")
            .and_then(Value::as_str)
            .unwrap_or(""),
        4_000,
    );
    assert!(
        second_system_prompt
            .to_ascii_lowercase()
            .contains("previous-turn process summary"),
        "{second_system_prompt}"
    );
}

#[test]
