fn workflow_decision_tree_blocks_status_check_turns_from_tool_calls() {
    let decision = workflow_turn_tool_decision_tree("did you do the web request??");
    assert_eq!(
        decision
            .get("status_check_message")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("requires_live_web")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
