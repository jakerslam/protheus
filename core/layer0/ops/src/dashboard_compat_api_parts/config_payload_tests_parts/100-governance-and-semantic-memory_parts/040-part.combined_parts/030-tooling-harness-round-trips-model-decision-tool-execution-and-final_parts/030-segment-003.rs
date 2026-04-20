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
        br#"{"message":"Run `infring web search` as the next safe step."}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        response_text.contains(SAFE_STEP_QUERY_HINT_040_COMBINED),
        "{response_text}"
    );
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

