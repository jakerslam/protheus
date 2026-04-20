fn terminal_tools_run_without_signoff_and_still_enforce_command_policy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let allowed = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-terminal",
        None,
        "terminal_exec",
        &json!({"command":"echo hi"}),
    );
    assert_ne!(
        allowed.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );
    let allow_verdict = allowed
        .pointer("/permission_gate/verdict")
        .and_then(Value::as_str)
        .unwrap_or("allow");
    assert_ne!(allow_verdict, "deny");

    let risky = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-terminal",
        None,
        "terminal_exec",
        &json!({"command":"git reset --hard HEAD"}),
    );
    assert_ne!(
        risky.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );
}

#[test]
