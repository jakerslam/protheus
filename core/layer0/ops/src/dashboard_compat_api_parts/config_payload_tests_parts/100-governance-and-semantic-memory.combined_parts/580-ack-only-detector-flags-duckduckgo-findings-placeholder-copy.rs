fn ack_only_detector_flags_duckduckgo_findings_placeholder_copy() {
    assert!(response_looks_like_tool_ack_without_findings(
        "I couldn't extract usable findings for this yet. The search response came from https://duckduckgo.com/html/?q=agent+systems"
    ));
}

#[test]
