fn finalize_user_facing_response_replaces_ack_without_findings() {
    let finalized = finalize_user_facing_response("Web search completed.".to_string(), None);
    let lowered = finalized.to_ascii_lowercase();
    assert!(!lowered.contains("web search completed"));
    assert!(lowered.contains("usable tool findings"));
    assert!(!response_looks_like_tool_ack_without_findings(&finalized));
}

#[test]
