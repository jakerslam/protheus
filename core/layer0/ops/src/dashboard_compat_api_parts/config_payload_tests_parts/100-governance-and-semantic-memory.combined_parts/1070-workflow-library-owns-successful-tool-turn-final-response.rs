fn workflow_library_owns_successful_tool_turn_final_response() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-owned-tool-turn-agent","role":"researcher"}"#,
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
                    "response": "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"source\":\"web\",\"query\":\"top AI agentic frameworks\",\"aperture\":\"medium\"}."
                },
                {"response": "For top AI agentic frameworks, the fetched evidence highlighted LangGraph, OpenAI Agents SDK, and AutoGen."}
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "web_search",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "LangGraph, OpenAI Agents SDK, and AutoGen surfaced as top AI agentic frameworks in the fetched results."
                    }
                }
            ],
            "calls": []
        }),
    );
    let choose_tool = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Try to web search \"top AI agentic frameworks\" and return the results"}"#,
        &snapshot,
    )
    .expect("tool choice response");
    assert_eq!(choose_tool.status, 200);
    assert_eq!(
        choose_tool
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("web_search"),
        "{}",
        choose_tool.payload
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"yes"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("For top AI agentic frameworks, the fetched evidence highlighted LangGraph, OpenAI Agents SDK, and AutoGen.")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/workflow_system_fallback_used")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
