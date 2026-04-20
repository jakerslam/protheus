fn explicit_tool_command_alias_surfaces_compare_workflow_hint() {
    let hints = chat_workflow_tool_hints_for_message("tool::compare:::top AI agent frameworks");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "batch_query"
    );
    assert_eq!(
        input.get("query").and_then(Value::as_str).unwrap_or(""),
        "top AI agent frameworks"
    );
    assert_eq!(
        input.get("source").and_then(Value::as_str).unwrap_or(""),
        "web"
    );
}

#[test]
