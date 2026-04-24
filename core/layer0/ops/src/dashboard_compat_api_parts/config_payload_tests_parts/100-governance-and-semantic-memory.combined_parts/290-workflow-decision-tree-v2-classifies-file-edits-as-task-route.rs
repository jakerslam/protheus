fn workflow_decision_tree_v2_classifies_file_edits_as_task_route() {
    let decision = workflow_turn_tool_decision_tree(
        "patch core/layer0/ops/src/main.rs to fix the gate",
    );
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_need_tool_access")
    );
    assert_eq!(
        decision
            .get("requires_file_mutation")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
