fn finalize_user_facing_response_never_leaks_tool_status_text() {
    let finalized = finalize_user_facing_response(
        "Tool call finished.".to_string(),
        Some("Tool call finished.".to_string()),
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(!lowered.contains("tool call finished"));
    assert!(!response_looks_like_tool_ack_without_findings(&finalized));
}

#[test]
