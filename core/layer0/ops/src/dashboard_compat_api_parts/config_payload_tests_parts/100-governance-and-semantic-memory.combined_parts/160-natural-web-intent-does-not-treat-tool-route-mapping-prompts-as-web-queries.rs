fn natural_web_intent_does_not_treat_tool_route_mapping_prompts_as_web_queries() {
    let route =
        natural_web_intent_from_user_message("Map `tool::web_search` into a supported route");
    assert!(route.is_none());
}

#[test]
