fn inline_tool_execution_replaces_low_signal_cleaned_text_with_tool_fallback_lines() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let response = "<function=spawn_subagents>{\"count\":2,\"objective\":\"parallelize analysis\"}</function>\nFrom web retrieval: bing.com: compare [A with B] vs compare A [with B]";
    let (text, cards, pending_confirmation, suppressed) = execute_inline_tool_calls(
        root.path(),
        &snapshot,
        "agent-inline-low-signal",
        None,
        response,
        "parallelize this with a swarm",
        true,
    );
    assert!(!suppressed);
    assert_eq!(cards.len(), 1);
    assert!(pending_confirmation.is_none());
    let lowered = text.to_ascii_lowercase();
    assert!(lowered.contains("spawned"));
    assert!(!lowered.contains("from web retrieval:"));
    assert!(!lowered.contains("bing.com: compare"));
}

#[test]
