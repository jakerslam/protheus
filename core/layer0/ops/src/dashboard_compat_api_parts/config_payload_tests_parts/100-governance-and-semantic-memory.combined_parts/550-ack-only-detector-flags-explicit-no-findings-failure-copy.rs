fn ack_only_detector_flags_explicit_no_findings_failure_copy() {
    assert!(!response_looks_like_tool_ack_without_findings(
        "The web search ran, but it only returned low-signal snippets in this turn."
    ));
    assert!(!response_looks_like_tool_ack_without_findings(
        "My search for top AI agentic frameworks 2024 didn't return specific framework listings or detailed comparisons."
    ));
    assert!(response_looks_like_tool_ack_without_findings(
        "From web retrieval: bing.com: OpenClaw — Personal AI Assistant — https://openclaw.ai/"
    ));
}

#[test]
