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
