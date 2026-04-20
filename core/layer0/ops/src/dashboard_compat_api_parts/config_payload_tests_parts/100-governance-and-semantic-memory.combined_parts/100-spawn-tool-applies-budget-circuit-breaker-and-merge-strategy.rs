fn spawn_tool_applies_budget_circuit_breaker_and_merge_strategy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let out = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-parent",
        None,
        "spawn_subagents",
        &json!({
            "count": 8,
            "objective": "Parallelize a large analysis task",
            "merge_strategy": "voting",
            "budget_tokens": 1_000_000,
            "confirm": true,
            "approval_note": "user requested bounded spawn for analysis"
        }),
    );
    let effective = out
        .get("effective_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    assert!(effective <= 1);
    assert_eq!(
        out.pointer("/directive/merge_strategy")
            .and_then(Value::as_str)
            .unwrap_or(""),
        "voting"
    );
    assert!(out
        .pointer("/circuit_breakers/degraded")
        .and_then(Value::as_bool)
        .unwrap_or(false));
}

#[test]
