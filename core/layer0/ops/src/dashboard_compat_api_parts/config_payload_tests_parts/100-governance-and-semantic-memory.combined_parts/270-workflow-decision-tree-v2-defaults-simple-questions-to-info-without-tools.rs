fn workflow_decision_tree_v2_defaults_simple_questions_to_info_without_tools() {
    let decision = workflow_turn_tool_decision_tree("what do you think about this idea?");
    assert_eq!(
        decision.get("contract").and_then(Value::as_str),
        Some("tool_decision_tree_v3")
    );
    assert_eq!(
        decision.get("route_classification").and_then(Value::as_str),
        Some("info")
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .pointer("/gates/gate_6/retry_limit")
            .and_then(Value::as_i64),
        Some(1)
    );
}

#[test]
