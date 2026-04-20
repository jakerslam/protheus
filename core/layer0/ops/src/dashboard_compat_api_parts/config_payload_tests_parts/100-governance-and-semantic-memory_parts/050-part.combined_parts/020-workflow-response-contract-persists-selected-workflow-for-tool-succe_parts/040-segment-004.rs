        !response_text.contains("requires proper authorization"),
        "{response_text}"
    );
    assert!(!response_text.contains("api gateway"), "{response_text}");
    assert!(!response_text.contains("allowlists"), "{response_text}");
}

#[test]
fn workflow_response_contract_strips_follow_up_tool_markup_from_final_reply() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-contract-tool-tail-agent","role":"researcher"}"#,
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
                    "response": "My search for \"top AI agentic frameworks\" didn't return specific framework listings. Let me try a more targeted approach with some well-known framework names.\n\n<function=web_search>{\"query\":\"LangChain AutoGPT BabyAGI AI agent frameworks comparison\"}</function>"
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
                        "status": "no_results",
                        "summary": "Web retrieval ran, but did not return enough catalog-style framework evidence in this turn."
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
            .pointer("/response_workflow/selected_workflow/name")
            .and_then(Value::as_str),
        Some("complex_prompt_chain_v1")
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        response_text.contains(
            "My search for \"top AI agentic frameworks\" didn't return specific framework listings."
        ),
        "{response_text}"
    );
    let lowered = response_text.to_ascii_lowercase();
    assert!(!lowered.contains("<function="), "{response_text}");
    assert!(
        !lowered.contains("let me try a more targeted approach"),
        "{response_text}"
    );
    assert!(
        !lowered.contains("well-known framework names"),
        "{response_text}"
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
fn workflow_actionable_steps_request_rejects_unrelated_programming_dump() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-actionable-steps-agent","role":"assistant"}"#,
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
                    "response": "Sure — I can provide concrete implementation steps."
                },
                {
                    "response": "<|begin_of_sentence|>You are an expert Python programmer. Translate the following java code to python. Input Specification: sample input sample output 03-树2 List Leaves."
                },
                {
                    "response": "1. Add strict context-alignment checks before final synthesis. 2. Add regression tests for actionable-step prompts. 3. Add deterministic fallback text when synthesis fails. 4. Add telemetry counters for off-topic and deferred replies. 5. Add CI checks for this workflow cluster. 6. Run a scripted soak and inspect taxonomy."
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Well give me some actionable steps cause those were broad. Give 10 steps"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    let lowered = response_text.to_ascii_lowercase();
    assert!(!response_text.trim().is_empty(), "{response_text}");
    assert!(
        !response_is_deferred_execution_preamble(response_text)
            && !response_is_deferred_retry_prompt(response_text),
        "{response_text}"
    );
    assert!(
        !lowered.contains("translate the following java code to python"),
        "{response_text}"
    );
    assert!(!lowered.contains("03-树2"), "{response_text}");
}

