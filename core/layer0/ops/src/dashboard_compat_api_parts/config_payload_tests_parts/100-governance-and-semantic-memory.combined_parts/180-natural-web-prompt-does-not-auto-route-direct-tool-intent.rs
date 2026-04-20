fn natural_web_prompt_does_not_auto_route_direct_tool_intent() {
    assert!(
        direct_tool_intent_from_user_message(
            "Try to web search \"top AI agentic frameworks\" and return the results"
        )
        .is_none()
    );
}

#[test]
