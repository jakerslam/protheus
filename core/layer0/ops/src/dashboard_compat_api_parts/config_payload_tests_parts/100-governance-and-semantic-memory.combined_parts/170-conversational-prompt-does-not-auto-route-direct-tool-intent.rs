fn conversational_prompt_does_not_auto_route_direct_tool_intent() {
    assert!(direct_tool_intent_from_user_message("what do you think of infring?").is_none());
}

#[test]
