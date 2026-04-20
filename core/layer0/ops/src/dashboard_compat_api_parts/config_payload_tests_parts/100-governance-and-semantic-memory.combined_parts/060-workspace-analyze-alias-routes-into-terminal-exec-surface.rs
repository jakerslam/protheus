fn workspace_analyze_alias_routes_into_terminal_exec_surface() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let routed = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-terminal",
        None,
        "workspace_analyze",
        &json!({"query":"effective loc"}),
    );
    assert_ne!(
        routed.get("error").and_then(Value::as_str),
        Some("unsupported_tool")
    );
    assert_ne!(
        routed.get("error").and_then(Value::as_str),
        Some("command_required")
    );
}

#[test]
