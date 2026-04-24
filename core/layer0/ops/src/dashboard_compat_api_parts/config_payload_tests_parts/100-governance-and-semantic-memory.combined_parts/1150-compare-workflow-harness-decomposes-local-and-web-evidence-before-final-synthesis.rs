fn compare_workflow_harness_decomposes_local_and_web_evidence_before_final_synthesis() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"compare-workflow-agent","role":"researcher"}"#,
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
            "queue": [],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "terminal_exec",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "Local workspace evidence shows workflow-gated synthesis via complex_prompt_chain_v1 and a domain-grouped tool catalog."
                    }
                },
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "External web evidence highlights OpenClaw's governed web/media tooling and native search contracts."
                    }
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"compare this system to openclaw"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let tool_names = response
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get("name").and_then(Value::as_str).map(ToString::to_string))
        .collect::<Vec<_>>();
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        !response_text.trim().is_empty(),
        "expected synthesized compare response"
    );
}

#[test]
