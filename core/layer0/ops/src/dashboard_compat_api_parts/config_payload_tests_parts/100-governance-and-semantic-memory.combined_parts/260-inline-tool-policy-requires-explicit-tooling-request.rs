fn inline_tool_policy_requires_explicit_tooling_request() {
    assert!(!inline_tool_calls_allowed_for_user_message(
        "what do you think of infring?"
    ));
    assert!(inline_tool_calls_allowed_for_user_message(
        "search the web for latest ai agent benchmarks"
    ));
    assert!(inline_tool_calls_allowed_for_user_message(
        "Try to web search \"top AI agentic frameworks\" and return the results"
    ));
    assert!(inline_tool_calls_allowed_for_user_message(
        "tool::web_search:::latest ai agent benchmarks"
    ));
    assert!(inline_tool_calls_allowed_for_user_message("/file core/layer0/ops/src/main.rs"));
    assert!(inline_tool_calls_allowed_for_user_message(
        "read file core/layer0/ops/src/main.rs"
    ));
    assert!(!inline_tool_calls_allowed_for_user_message(
        "just answer directly, dont use a tool call"
    ));
    assert!(!inline_tool_calls_allowed_for_user_message(
        "why do you keep trying tool calls and not synthesizing results"
    ));
}

#[test]
