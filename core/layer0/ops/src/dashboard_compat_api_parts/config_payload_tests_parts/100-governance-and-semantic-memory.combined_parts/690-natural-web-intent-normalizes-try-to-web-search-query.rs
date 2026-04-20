fn natural_web_intent_normalizes_try_to_web_search_query() {
    let route = natural_web_intent_from_user_message(
        "try to web search \"top AI agent frameworks\""
    )
    .expect("route");
    assert_eq!(route.0, "batch_query");
    assert_eq!(
        route.1.get("query").and_then(Value::as_str),
        Some("top AI agent frameworks")
    );
}

#[test]
