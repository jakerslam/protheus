fn web_tooling_harness_surfaces_no_results_with_final_llm_synthesis() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"web-tooling-no-results-agent","role":"researcher"}"#,
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
                {
                    "response": "Yes. Tool family: Web Search / Fetch. Tool: Web search. Request payload: {\"source\":\"web\",\"query\":\"top AI agentic frameworks\",\"aperture\":\"medium\"}"
                }
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": false,
                        "status": "no_results",
                        "summary": "Web retrieval returned low-signal snippets without synthesis. Retry with a narrower query or a specific source URL.",
                        "error": "search_providers_exhausted"
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
        br#"{"message":"Try to web search \"top AI agentic frameworks\" and return the results"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/tools/0/name")
            .and_then(Value::as_str),
        Some("batch_query"),
        "expected web search tool execution; payload={}",
        response.payload
    );
    assert_eq!(
        response
            .payload
            .pointer("/tools/0/status")
            .and_then(Value::as_str),
        Some("no_results")
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    let lowered = response_text.to_ascii_lowercase();
    assert!(
        !response_text.trim().is_empty(),
        "expected synthesized no-results reply; payload={}; chat_calls={:?}",
        response.payload,
        read_json(&governance_test_chat_script_path(root.path()))
    );
    assert!(
        lowered.contains("low-signal") || lowered.contains("source-backed answer"),
        "{response_text}"
    );
    assert!(!response_is_no_findings_placeholder(response_text));
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
}

#[test]
