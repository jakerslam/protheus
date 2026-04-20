fn natural_web_intent_routes_openclaw_comparison_to_batch_query() {
    let route = natural_web_intent_from_user_message(
        "compare openclaw to this system/workspace using web search"
    )
    .expect("route");
    assert_eq!(route.0, "batch_query");
    assert_eq!(
        route.1.get("source").and_then(Value::as_str),
        Some("web")
    );
    let query = route
        .1
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(query.to_ascii_lowercase().contains("openclaw"));
    assert!(query.to_ascii_lowercase().contains("workspace"));
}

#[test]
