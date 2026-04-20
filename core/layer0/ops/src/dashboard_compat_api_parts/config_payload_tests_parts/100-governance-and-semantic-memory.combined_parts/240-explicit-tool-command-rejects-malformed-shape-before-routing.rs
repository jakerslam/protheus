fn explicit_tool_command_rejects_malformed_shape_before_routing() {
    let hints = chat_workflow_tool_hints_for_message("tool::web_search::latest");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "tool_command_router"
    );
    assert_eq!(input.get("error").and_then(Value::as_str).unwrap_or(""), "tool_command_name_invalid");
}

#[test]
