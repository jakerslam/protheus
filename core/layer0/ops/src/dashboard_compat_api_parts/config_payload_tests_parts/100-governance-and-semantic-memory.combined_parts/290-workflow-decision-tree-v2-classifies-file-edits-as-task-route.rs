fn workflow_decision_tree_v2_classifies_file_edits_as_task_route() {
    let decision = workflow_turn_tool_decision_tree(
        "patch core/layer0/ops/src/main.rs to fix the gate",
    );
    assert_eq!(
        decision.get("route_classification").and_then(Value::as_str),
        Some("task")
    );
    assert_eq!(
        decision
            .get("requires_file_mutation")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        decision
            .get("recommended_tool_family")
            .and_then(Value::as_str),
        Some("file_tools")
    );
}

#[test]
