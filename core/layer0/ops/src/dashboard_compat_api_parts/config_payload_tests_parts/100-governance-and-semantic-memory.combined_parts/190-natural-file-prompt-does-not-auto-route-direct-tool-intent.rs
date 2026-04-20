fn natural_file_prompt_does_not_auto_route_direct_tool_intent() {
    assert!(direct_tool_intent_from_user_message("read file core/layer0/ops/src/main.rs").is_none());
}

#[test]
