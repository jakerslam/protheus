fn explicit_tool_command_surfaces_web_search_workflow_hint() {
    assert!(direct_tool_intent_from_user_message("tool::web_search:::latest ai agent benchmarks").is_none());
    let hints = chat_workflow_tool_hints_for_message("tool::web_search:::latest ai agent benchmarks");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "web_search"
    );
    assert_eq!(
        input.get("query").and_then(Value::as_str).unwrap_or(""),
        "latest ai agent benchmarks"
    );
    assert_eq!(
        input.get("source").and_then(Value::as_str).unwrap_or(""),
        "web"
    );
    assert_eq!(
        input.get("aperture").and_then(Value::as_str).unwrap_or(""),
        "medium"
    );
}

#[test]
