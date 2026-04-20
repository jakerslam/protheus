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
