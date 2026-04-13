#[test]
fn natural_web_intent_strips_return_the_results_suffix() {
    let route = natural_web_intent_from_user_message(
        "Try to web search \"top AI agentic frameworks\" and return the results",
    )
    .expect("route");
    assert_eq!(route.0, "batch_query");
    assert_eq!(
        route.1.get("query").and_then(Value::as_str),
        Some("top AI agentic frameworks")
    );
}

#[test]
fn inline_batch_query_input_is_normalized_before_execution() {
    let normalized = normalize_inline_tool_execution_input(
        "batch_query",
        &json!({
            "query": "Try to web search \"top AI agentic frameworks\" and return the results"
        }),
        "Try to web search \"top AI agentic frameworks\" and return the results",
    );
    assert_eq!(
        normalized.get("query").and_then(Value::as_str),
        Some("top AI agentic frameworks")
    );
    assert_eq!(normalized.get("source").and_then(Value::as_str), Some("web"));
    assert_eq!(
        normalized.get("aperture").and_then(Value::as_str),
        Some("medium")
    );
}

#[test]
fn natural_web_intent_does_not_force_plain_workspace_peer_compare_into_web() {
    assert!(natural_web_intent_from_user_message("compare this system to openclaw").is_none());
    assert!(natural_web_intent_from_user_message("compare openclaw to this system/workspace").is_none());
}

#[test]
fn response_tools_summary_keeps_actionable_web_diagnostic_rows() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "batch_query",
            "is_error": false,
            "result": "Search returned no useful comparison findings for infring vs openclaw."
        })],
        4,
    );
    let lowered = synthesized.to_ascii_lowercase();
    assert!(lowered.contains("retrieval-quality miss"));
    assert!(lowered.contains("batch query"));
}

#[test]
fn finalize_user_facing_response_keeps_actionable_web_diagnostic_copy() {
    let finalized = finalize_user_facing_response(
        "Web retrieval returned low-signal snippets without synthesis. Ask me to rerun with a narrower query and I will return a concise source-backed answer."
            .to_string(),
        None,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("low-signal snippets without synthesis"));
    assert!(!lowered.contains("don't have usable tool findings from this turn yet"));
}

#[test]
fn response_tools_failure_reason_includes_no_results_web_reason() {
    let reason = response_tools_failure_reason_for_user(
        &[json!({
            "name": "batch_query",
            "status": "no_results",
            "blocked": false,
            "is_error": false,
            "result": "Search providers ran, but only low-signal or low-relevance web results came back in this turn. Retry with a narrower query or one specific source URL for source-backed findings."
        })],
        4,
    );
    let lowered = reason.to_ascii_lowercase();
    assert!(lowered.contains("tool run hit issues"));
    assert!(lowered.contains("low-signal") || lowered.contains("source-backed findings"));
    assert!(!lowered.contains("don't have usable tool findings from this turn yet"));
}

#[test]
fn response_tools_failure_reason_includes_ok_status_low_signal_web_reason() {
    let reason = response_tools_failure_reason_for_user(
        &[json!({
            "name": "batch_query",
            "status": "ok",
            "blocked": false,
            "is_error": false,
            "result": "Web retrieval returned low-signal snippets without synthesis. Retry with a narrower query or one specific source URL for source-backed findings."
        })],
        4,
    );
    let lowered = reason.to_ascii_lowercase();
    assert!(lowered.contains("tool run hit issues"));
    assert!(lowered.contains("low-signal"));
    assert!(!lowered.contains("don't have usable tool findings from this turn yet"));
}
