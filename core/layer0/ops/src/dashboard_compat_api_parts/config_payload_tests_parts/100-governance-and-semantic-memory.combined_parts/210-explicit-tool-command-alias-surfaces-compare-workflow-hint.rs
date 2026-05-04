fn explicit_tool_command_alias_rejects_compare_shortcut() {
    let hints = chat_workflow_tool_hints_for_message("tool::compare:::compare named systems");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "tool_command_router"
    );
    assert_eq!(
        input.get("error").and_then(Value::as_str).unwrap_or(""),
        "unsupported_tool_command"
    );
}

#[test]
