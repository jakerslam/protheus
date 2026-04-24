#[test]
fn enforce_user_facing_finalization_contract_unwraps_internal_payload_dump() {
    let cards = vec![json!({
        "name": "web_search",
        "is_error": false,
        "result": "From web retrieval: benchmark summary with sources."
    })];
    let raw = json!({
        "agent_id": "agent-raw-dump",
        "response": "From web retrieval: benchmark summary with sources.",
        "turn_loop_tracking": {"ok": true},
        "turn_transaction": {"tool_execute": "complete"}
    })
    .to_string();
    let (finalized, report, outcome) = enforce_user_facing_finalization_contract(
        "summarize benchmark findings",
        raw,
        &cards,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(!finalized.trim_start().starts_with('{'));
    assert!(!lowered.contains("agent_id"));
    assert!(
        lowered.contains("benchmark summary")
            || lowered.contains("usable tool findings from this turn yet")
    );
    assert_eq!(
        report.get("completion_state").and_then(Value::as_str),
        Some("reported_no_findings")
    );
    assert!(
        outcome.contains("normalized_raw_payload_json") || outcome.contains("reported_no_findings"),
        "unexpected outcome={outcome}"
    );
}

#[test]
fn follow_up_suggestion_tool_intent_requires_query_for_infring_web_search_prompt() {
    let (tool, payload) =
        follow_up_suggestion_tool_intent_from_message("Run `infring web search` as the next safe step.")
            .expect("route");
    assert_eq!(tool, "tool_command_router");
    let message = payload.get("message").and_then(Value::as_str).unwrap_or("");
    assert!(message.contains("needs a query"));
    assert!(message.contains("top AI agent frameworks"));
}

#[test]
fn maybe_tooling_failure_fallback_rewrites_safe_step_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "Run `infring web search` as the next safe step.",
        &no_findings_user_facing_response(),
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("needs a query"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

#[test]
fn workflow_gated_turn_persists_finalization_in_session_history() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"persisted-tool-history-agent","role":"operator"}"#,
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
    let _ = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Run `infring web search` as the next safe step."}"#,
        &snapshot,
    )
    .expect("message");
    let state = crate::dashboard_agent_state::load_session(root.path(), &agent_id);
    let assistant = state
        .pointer("/session/sessions/0/messages")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().rev().find(|row| {
                row.get("role")
                    .and_then(Value::as_str)
                    .map(|role| role.eq_ignore_ascii_case("assistant"))
                    .unwrap_or(false)
            })
        })
        .cloned()
        .unwrap_or(Value::Null);
    assert_eq!(assistant.get("role").and_then(Value::as_str), Some("assistant"));
    let response_text = assistant.get("text").and_then(Value::as_str).unwrap_or("");
    assert!(response_text.contains("needs a query"), "{response_text}");
    assert_eq!(
        assistant
            .pointer("/response_workflow/contract")
            .and_then(Value::as_str),
        Some("agent_workflow_library_v1")
    );
    assert_eq!(
        assistant
            .pointer("/response_workflow/selected_workflow/name")
            .and_then(Value::as_str),
        Some("complex_prompt_chain_v1")
    );
    assert!(assistant.get("response_finalization").is_some());
}

#[test]
fn workflow_library_marks_final_llm_stage_for_tool_turns() {
    let workflow = run_turn_workflow_final_response(
        Path::new("."),
        "auto",
        "auto",
        &[],
        "Try to web search \"top AI agent frameworks\" and return the results",
        "model_inline_tool_execution",
        &[json!({
            "name": "batch_query",
            "input": "{\"source\":\"web\",\"query\":\"top AI agent frameworks\"}",
            "status": "ok",
            "is_error": false,
            "blocked": false,
            "result": "Web retrieval returned low-signal snippets without synthesis."
        })],
        &[],
        "Web retrieval returned low-signal snippets without synthesis.",
        "",
    );
    assert_eq!(
        workflow.get("contract").and_then(Value::as_str),
        Some("agent_workflow_library_v1")
    );
    assert_eq!(
        workflow
            .pointer("/selected_workflow/name")
            .and_then(Value::as_str),
        Some("complex_prompt_chain_v1")
    );
    assert_eq!(
        workflow
            .pointer("/final_llm_response/required")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        workflow
            .pointer("/workflow_gate/status")
            .and_then(Value::as_str),
        Some("presented")
    );
    assert_eq!(
        workflow
            .pointer("/final_llm_response/status")
            .and_then(Value::as_str),
        Some("skipped_test")
    );
    assert_eq!(
        workflow
            .pointer("/current_stage")
            .and_then(Value::as_str),
        Some("gate_6_llm_final_output")
    );
    assert_eq!(
        workflow
            .pointer("/visibility/system_injected_chat_text_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        workflow
            .pointer("/library/default_workflow")
            .and_then(Value::as_str),
        Some("simple_conversation_v1")
    );
    assert_eq!(
        workflow
            .pointer("/stage_statuses/0/stage")
            .and_then(Value::as_str),
        Some("gate_1_need_tool_access_menu")
    );
    assert_eq!(
        workflow
            .pointer("/stage_statuses/0/status")
            .and_then(Value::as_str),
        Some("answered_no")
    );
    assert_eq!(
        workflow
            .pointer("/stage_statuses/1/stage")
            .and_then(Value::as_str),
        Some("gate_6_llm_final_output")
    );
    assert_eq!(
        workflow
            .pointer("/stage_statuses/1/status")
            .and_then(Value::as_str),
        Some("skipped_test")
    );
}

#[test]
fn workflow_library_gate_applies_to_direct_answers_too() {
    let workflow = run_turn_workflow_final_response(
        Path::new("."),
        "auto",
        "auto",
        &[],
        "Just say hello normally",
        "model_direct_answer",
        &[],
        &[],
        "Hello there.",
        "",
    );
    assert_eq!(
        workflow.get("contract").and_then(Value::as_str),
        Some("agent_workflow_library_v1")
    );
    assert_eq!(
        workflow
            .pointer("/selected_workflow/name")
            .and_then(Value::as_str),
        Some("simple_conversation_v1")
    );
    assert_eq!(
        workflow
            .pointer("/workflow_gate/status")
            .and_then(Value::as_str),
        Some("presented")
    );
    assert_eq!(
        workflow
            .pointer("/final_llm_response/status")
            .and_then(Value::as_str),
        Some("skipped_test")
    );
    assert_eq!(
        workflow
            .pointer("/current_stage")
            .and_then(Value::as_str),
        Some("gate_6_llm_final_output")
    );
    assert_eq!(
        workflow
            .pointer("/current_stage_status")
            .and_then(Value::as_str),
        Some("skipped_test")
    );
    assert_eq!(
        workflow
            .pointer("/visibility/formats/ui")
            .and_then(Value::as_str),
        Some("Workflow complete; no tools selected and direct LLM answer is ready.")
    );
    assert_eq!(
        workflow
            .pointer("/visibility/system_injected_chat_text_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn direct_message_safe_step_prompt_returns_workflow_metadata() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let parent = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-metadata-agent","role":"operator"}"#,
        &snapshot,
    )
    .expect("parent create");
    let parent_id = clean_agent_id(
        parent
            .payload
            .get("agent_id")
            .or_else(|| parent.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        br#"{"message":"Run `infring web search` as the next safe step."}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/contract")
            .and_then(Value::as_str),
        Some("agent_workflow_library_v1")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/selected_workflow/name")
            .and_then(Value::as_str),
        Some("complex_prompt_chain_v1")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("skipped_test")
    );
}

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
