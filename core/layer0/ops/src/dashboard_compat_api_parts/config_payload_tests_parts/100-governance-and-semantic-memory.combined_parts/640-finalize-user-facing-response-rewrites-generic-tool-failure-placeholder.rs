fn finalize_user_facing_response_rewrites_generic_tool_failure_placeholder() {
    let finalized = finalize_user_facing_response(
        "I couldn't complete system_diagnostic right now.".to_string(),
        None,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("doctor --json"));
    assert!(!lowered.contains("couldn't complete system_diagnostic right now"));
}

#[test]
