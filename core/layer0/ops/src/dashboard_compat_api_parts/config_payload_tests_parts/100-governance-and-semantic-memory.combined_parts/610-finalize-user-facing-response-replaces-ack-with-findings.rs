fn finalize_user_facing_response_replaces_ack_with_findings() {
    let finalized = finalize_user_facing_response(
        "Web search completed.".to_string(),
        Some("Here's what I found:\n- arxiv.org/abs/2601.12345".to_string()),
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(!lowered.contains("web search completed"));
    assert!(lowered.contains("here's what i found"));
    assert!(!response_looks_like_tool_ack_without_findings(&finalized));
}

#[test]
