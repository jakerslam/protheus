    assert!(!response_text.contains("configuration restrictions"), "{response_text}");
    assert!(
        !response_text.contains("blocking tool execution attempts"),
        "{response_text}"
    );
}

#[test]

fn workflow_retry_executes_latent_web_candidate_after_ack_only_format_block_copy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-ack-only-format-retry-agent","role":"researcher"}"#,
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
                    "response": "I attempted the web search again with the exact function call format, but the system rejected it with an \"ack_only\" response, meaning it recognized the function but didn't execute it. The platform is actively blocking external tool execution. The rejection appears to be a deliberate system policy rather than a technical failure."
                },
                {
                    "response": "I reran this through live retrieval and found current framework coverage across LangGraph, OpenAI Agents SDK, and AutoGen."
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
                        "ok": true,
                        "status": "ok",
                        "summary": "LangGraph, OpenAI Agents SDK, and AutoGen surfaced in this rerun."
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
        br#"{"message":"Test the web capability again and summarize findings."}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_selected_workflow_and_visible_response(
        &response,
        "I reran this through live retrieval and found current framework coverage across LangGraph, OpenAI Agents SDK, and AutoGen.",
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(!response_text.contains("ack_only"), "{response_text}");
    assert!(
        !response_text.contains("function call format"),
        "{response_text}"
    );
    assert!(
        !response_text.contains("didn't execute"),
        "{response_text}"
    );
}

#[test]
fn workflow_retry_executes_latent_web_candidate_after_security_controls_block_copy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-security-controls-web-retry-agent","role":"researcher"}"#,
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
                    "response": "I attempted the web search and fetch operations again using the exact function call format, but the system continues to block external tool execution. The platform recognizes the function requests, but consistently prevents actual execution, likely due to the security controls you've implemented. The system requires proper authorization before executing external calls. Since you're the security team, you might want to check API gateway configurations, external service allowlists, execution policy settings, and sandbox-vs-production mode differences."
                },
                {
                    "response": "I reran live retrieval and found current web-tooling coverage with source-backed results for LangGraph, OpenAI Agents SDK, and CrewAI."
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
                        "ok": true,
                        "status": "ok",
                        "summary": "LangGraph, OpenAI Agents SDK, and CrewAI surfaced in this rerun."
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
        br#"{"message":"try the web tooling again"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert!(response
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .any(|row| row.get("name").and_then(Value::as_str) == Some("batch_query"))
        })
        .unwrap_or(false));
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(!response_text.is_empty(), "{response_text}");
    assert!(
        response_text.contains("langgraph")
            || response_text.contains("openai agents sdk")
            || response_text.contains("crewai"),
        "{response_text}"
    );
    assert!(!response_text.contains("security controls"), "{response_text}");
    assert!(
