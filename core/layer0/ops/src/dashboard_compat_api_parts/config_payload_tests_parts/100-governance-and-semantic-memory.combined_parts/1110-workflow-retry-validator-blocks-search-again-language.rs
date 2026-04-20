fn workflow_retry_validator_blocks_search_again_language() {
    assert!(workflow_response_requests_more_tooling(
        "Let me search for more specific AI agent framework information using a narrower query."
    ));
    assert!(workflow_response_requests_more_tooling(
        "Retry with a narrower query or one specific source URL."
    ));
    assert!(!workflow_response_requests_more_tooling(
        "The web search ran, but it only returned low-signal snippets in this turn."
    ));
}

#[test]
