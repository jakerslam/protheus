fn explicit_tool_command_alias_surfaces_fetch_workflow_hint() {
    let hints = chat_workflow_tool_hints_for_message("tool::fetch:::https://example.com");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "web_fetch"
    );
    assert_eq!(
        input.get("url").and_then(Value::as_str).unwrap_or(""),
        "https://example.com"
    );
}

#[test]
