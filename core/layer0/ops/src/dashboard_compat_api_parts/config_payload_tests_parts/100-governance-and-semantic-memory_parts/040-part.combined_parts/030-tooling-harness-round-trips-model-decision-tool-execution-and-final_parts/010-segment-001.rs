
#[test]
fn web_tooling_harness_round_trips_model_decision_tool_execution_and_final_synthesis() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"web-tooling-harness-agent","role":"researcher"}"#,
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
                    "response": "<function=batch_query>{\"source\":\"web\",\"query\":\"top AI agentic frameworks\",\"aperture\":\"medium\"}</function>"
                },
                {
                    "response": "The fetched results point to LangGraph, OpenAI Agents SDK, and AutoGen as top AI agentic frameworks in this run."
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
                        "query": "top AI agentic frameworks",
                        "summary": "LangGraph, OpenAI Agents SDK, and AutoGen surfaced as top AI agentic frameworks in the fetched results."
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
        Some("batch_query")
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(response_text.contains("LangGraph"), "{response_text}");
    assert!(response_text.contains("OpenAI Agents SDK"), "{response_text}");
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );

    let tool_calls = read_json(&governance_test_tool_script_path(root.path()))
        .and_then(|value| value.get("calls").cloned())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(
        tool_calls[0].get("tool").and_then(Value::as_str),
        Some("batch_query")
    );

    let model_calls = read_json(&governance_test_chat_script_path(root.path()))
        .and_then(|value| value.get("calls").cloned())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    assert!(
        model_calls.len() >= 2,
        "expected at least two model passes, got {}",
        model_calls.len()
    );
    let first_user_message = model_calls[0]
        .get("user_message")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(first_user_message.contains("top AI agentic frameworks"));
    let final_user_message = model_calls
        .last()
        .and_then(|row| row.get("user_message"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(final_user_message.contains("Recorded tool outcomes"));
    assert!(final_user_message.contains("LangGraph"));
    assert!(final_user_message.contains("OpenAI Agents SDK"));
}

#[test]

fn web_tooling_harness_surfaces_timeout_failure_with_final_llm_synthesis() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"web-tooling-failure-harness-agent","role":"researcher"}"#,
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
                    "response": "<function=batch_query>{\"source\":\"web\",\"query\":\"top AI agentic frameworks\",\"aperture\":\"medium\"}</function>"
                }
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &{
            let timeout_payload = json!({
                "ok": false,
                "status": "timeout",
                "error": "provider timeout after 30s",
                "summary": "provider timeout after 30s"
            });
            let timeout_queue =
                vec![json!({"tool": "batch_query", "payload": timeout_payload.clone()}); 4];
            json!({
                "queue": timeout_queue,
                "calls": []
            })
        },
    );

