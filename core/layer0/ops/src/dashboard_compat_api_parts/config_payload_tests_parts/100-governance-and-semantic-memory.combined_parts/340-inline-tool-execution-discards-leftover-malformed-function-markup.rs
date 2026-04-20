fn inline_tool_execution_discards_leftover_malformed_function_markup() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let response = "<function=web_search>{\"query\":\"test search functionality\"}</function> <function=web_fetch>{\"url\":\"https://example.";
    let (text, cards, pending_confirmation, suppressed) = execute_inline_tool_calls(
        root.path(),
        &snapshot,
        "agent-inline-malformed-tail",
        None,
        response,
        "test search functionality",
        true,
    );
    assert!(!suppressed);
    assert_eq!(cards.len(), 1);
    assert!(pending_confirmation.is_none());
    assert!(!text.contains("<function="), "{text}");
    assert!(!text.to_ascii_lowercase().contains("web_fetch"), "{text}");
    assert!(!text.trim().is_empty(), "{text}");
}

#[test]
