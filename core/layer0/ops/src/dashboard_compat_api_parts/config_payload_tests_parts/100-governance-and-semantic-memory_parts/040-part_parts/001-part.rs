const SAFE_STEP_PROMPT_BODY_JSON_001: &[u8] =
    br#"{"message":"Run `infring web search` as the next safe step."}"#;
const SAFE_STEP_QUERY_HINT_001: &str = "needs a query";

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
    assert_eq!(
        response
            .payload
            .pointer("/tools/0/status")
            .and_then(Value::as_str),
        Some("timeout")
    );
    assert_eq!(
        response
            .payload
            .pointer("/tools/0/is_error")
            .and_then(Value::as_bool),
        Some(true)
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    let lowered = response_text.to_ascii_lowercase();
    assert!(
        !response_text.trim().is_empty(),
        "expected a synthesized failure response"
    );
    assert!(
        lowered.contains("timeout"),
        "expected timeout detail in final response: {response_text}"
    );
    assert!(
        lowered.contains("batch_query") || lowered.contains("search"),
        "expected tool context in final response: {response_text}"
    );
    assert!(!response_is_no_findings_placeholder(response_text));
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
    assert_eq!(tool_calls.len(), 4);
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
    let final_user_message = model_calls
        .last()
        .and_then(|row| row.get("user_message"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(final_user_message.contains("Recorded tool outcomes"));
    assert!(final_user_message.contains("provider timeout after 30s"));
}

#[test]
fn maybe_tooling_failure_fallback_rewrites_web_file_better_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "does the web or file tooling seem any better?",
        &no_findings_user_facing_response(),
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("routing/finalization miss"));
    assert!(lowered.contains("web_search"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

#[test]
fn maybe_tooling_failure_fallback_rewrites_route_mapping_suggestion_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "Run Improve command-to-route mapping for higher supported tool hit rate",
        &no_findings_user_facing_response(),
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("implementation task"));
    assert!(lowered.contains("command-to-route mapping"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

#[test]
fn maybe_tooling_failure_fallback_rewrites_spawn_route_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "Implement a supported Rust route for `tool::spawn_subagents`",
        &no_findings_user_facing_response(),
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("runtime-route implementation task"));
    assert!(lowered.contains("spawn_subagents"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

#[test]
fn direct_message_safe_step_prompt_returns_actionable_query_required_copy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let parent = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"follow-up-suggestion-agent","role":"operator"}"#,
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
    assert!(!parent_id.is_empty());
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        SAFE_STEP_PROMPT_BODY_JSON_001,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(response_text.contains(SAFE_STEP_QUERY_HINT_001), "{response_text}");
    assert!(!response_is_no_findings_placeholder(response_text));
    assert!(
        response
            .payload
            .get("tools")
            .and_then(Value::as_array)
            .map(|rows| rows.is_empty())
            .unwrap_or(true)
    );
    let workflow_status = response
        .payload
        .pointer("/response_workflow/final_llm_response/status")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(workflow_status, "skipped_test");
}

#[test]
fn tool_completion_contract_rewrites_ack_to_findings_from_tool_cards() {
    let cards = vec![json!({
        "name": "web_search",
        "is_error": false,
        "result": "Web search findings for \"runtime reliability\": https://example.com/reliability-overview"
    })];
    let (finalized, report) = enforce_tool_completion_contract("Web search completed.".to_string(), &cards);
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("here's what i found"));
    assert!(!lowered.contains("web search completed"));
    assert!(!lowered.contains("source-backed findings in this turn"));
    assert_eq!(
        report.get("completion_state").and_then(Value::as_str),
        Some("reported_findings")
    );
    assert_eq!(
        report.get("final_ack_only").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn tool_completion_contract_rewrites_ack_to_explicit_no_findings_when_results_are_low_signal() {
    let cards = vec![json!({
        "name": "web_search",
        "is_error": false,
        "result": "Web search completed."
    })];
    let (finalized, report) =
        enforce_tool_completion_contract("Web search completed.".to_string(), &cards);
    assert_eq!(finalized, no_findings_user_facing_response());
    assert_eq!(
        report.get("completion_state").and_then(Value::as_str),
        Some("reported_no_findings")
    );
    assert_eq!(
        report.get("final_no_findings").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn tool_completion_contract_rewrites_unverified_execution_claim_when_no_tools_exist() {
    let (finalized, report) = enforce_tool_completion_contract(
        "Batch execution initiated - 5 concurrent searches running. This demonstrates the full pipeline."
            .to_string(),
        &[],
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("usable tool findings from this turn yet"));
    assert!(!lowered.contains("batch execution initiated"));
    assert_eq!(
        report.get("completion_state").and_then(Value::as_str),
        Some("not_applicable")
    );
}

#[test]
fn response_ack_detector_flags_batch_execution_scaffold_copy() {
    assert!(response_looks_like_tool_ack_without_findings(
        "Batch execution initiated - 5 concurrent searches running with concurrency limiting. This demonstrates the full pipeline from command parsing to response formatting."
    ));
}

#[test]
fn tool_completion_contract_preserves_actionable_failure_reason_when_no_findings_exist() {
    let cards = vec![json!({
        "name": "web_search",
        "is_error": true,
        "result": "I need your confirmation before running `web_search`."
    })];
    let (finalized, report) = enforce_tool_completion_contract(
        "I need your confirmation before running `web_search`.".to_string(),
        &cards,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("confirmation"));
    assert!(!lowered.contains("web search completed"));
    assert_eq!(
        report.get("completion_state").and_then(Value::as_str),
        Some("reported_reason")
    );
}

#[test]
fn enforce_user_facing_finalization_contract_uses_tool_failure_reason_when_payload_is_unsynthesized() {
    let cards = vec![json!({
        "name": "web_search",
        "is_error": true,
        "status": "timeout",
        "result": "provider timeout after 30s"
    })];
    let (finalized, report, outcome) = enforce_user_facing_finalization_contract(
        "what happened with the web tooling",
        "I completed the tool call, but no synthesized response was available yet. Check the tool details below.".to_string(),
        &cards,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(
        lowered.contains("provider timeout"),
        "finalized={finalized} outcome={outcome} report={report}"
    );
    assert!(!response_is_no_findings_placeholder(&finalized));
    assert_eq!(
        report.get("completion_state").and_then(Value::as_str),
        Some("reported_reason")
    );
    assert!(outcome.contains("tool_failure_reason"));
}

#[test]
fn relevant_recall_context_surfaces_older_thread_facts_for_continuity() {
    let pooled_messages = vec![
        json!({"role":"user","text":"Remember that cobalt sunrise is our fallback phrase.","ts":"2026-04-01T00:00:00Z"}),
        json!({"role":"assistant","text":"Stored. Cobalt sunrise is the fallback phrase.","ts":"2026-04-01T00:00:01Z"}),
        json!({"role":"user","text":"Also track dashboard reconnect reliability fixes.","ts":"2026-04-01T00:00:02Z"}),
        json!({"role":"assistant","text":"I will keep reconnect fixes and fallback phrase in scope.","ts":"2026-04-01T00:00:03Z"}),
    ];
    let active_messages = vec![
        json!({"role":"user","text":"How do we improve reconnect reliability next?","ts":"2026-04-01T00:05:00Z"}),
    ];
    let context = historical_relevant_recall_prompt_context(
        &pooled_messages,
        &active_messages,
        "Use the fallback phrase and reconnect plan from earlier",
        8,
        2400,
    );
    let lowered = context.to_ascii_lowercase();
    assert!(lowered.contains("relevant long-thread recall"));
    assert!(lowered.contains("fallback phrase") || lowered.contains("cobalt sunrise"));
    assert!(lowered.contains("reconnect"));
}

#[test]
fn relevant_recall_context_skips_external_framework_identity_bleed_for_infring_turns() {
    let pooled_messages = vec![
        json!({"role":"user","text":"so how do you think that infring can be better?","ts":"2026-04-01T00:00:00Z"}),
        json!({"role":"assistant","text":"As an infring agent, I can help improve areas or functionalities within the external sample framework.","ts":"2026-04-01T00:00:01Z"}),
        json!({"role":"assistant","text":"Infring orchestration should improve context isolation and tool-path reliability.","ts":"2026-04-01T00:00:02Z"}),
    ];
    let active_messages = vec![
        json!({"role":"user","text":"How can infring improve next?","ts":"2026-04-01T00:05:00Z"}),
    ];
    let context = historical_relevant_recall_prompt_context(
        &pooled_messages,
        &active_messages,
        "How can infring improve next?",
        8,
        2400,
    );
    let lowered = context.to_ascii_lowercase();
    assert!(lowered.contains("context isolation"));
    assert!(!lowered.contains("within the external sample framework"));
}

#[test]
