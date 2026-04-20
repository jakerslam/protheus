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
