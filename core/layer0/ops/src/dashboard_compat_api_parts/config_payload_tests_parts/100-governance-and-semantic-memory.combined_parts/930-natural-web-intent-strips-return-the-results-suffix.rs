fn natural_web_intent_strips_return_the_results_suffix() {
    let route = natural_web_intent_from_user_message(
        "Try to web search \"top AI agentic frameworks\" and return the results",
    )
    .expect("route");
    assert_eq!(route.0, "batch_query");
    assert_eq!(
        route.1.get("query").and_then(Value::as_str),
        Some("top AI agentic frameworks")
    );
}

#[test]
