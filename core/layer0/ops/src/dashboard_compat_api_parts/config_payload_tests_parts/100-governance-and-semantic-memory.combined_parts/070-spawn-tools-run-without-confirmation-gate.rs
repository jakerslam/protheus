fn spawn_tools_run_without_confirmation_gate() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let out = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-spawn",
        None,
        "spawn_subagents",
        &json!({
            "count": 2,
            "objective": "parallelize architecture diagnostics"
        }),
    );
    let error = out.get("error").and_then(Value::as_str).unwrap_or("");
    assert_ne!(error, "tool_explicit_signoff_required");
    assert_ne!(error, "tool_confirmation_required");
    assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
}

#[test]
