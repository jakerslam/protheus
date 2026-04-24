fn workflow_decision_tree_v2_defaults_simple_questions_to_info_without_tools() {
    let decision = workflow_turn_tool_decision_tree("what do you think about this idea?");
    assert_eq!(
        decision.get("contract").and_then(Value::as_str),
        Some("tool_decision_tree_v3")
    );
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_need_tool_access")
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("auto_decisions_disabled")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        decision
            .pointer("/gates/gate_6/retry_limit")
            .and_then(Value::as_i64),
        Some(1)
    );
}

#[test]
