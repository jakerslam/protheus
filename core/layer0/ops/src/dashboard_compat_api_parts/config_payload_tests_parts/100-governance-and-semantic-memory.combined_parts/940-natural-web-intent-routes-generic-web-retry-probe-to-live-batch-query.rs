fn natural_web_intent_routes_generic_web_retry_probe_to_live_batch_query() {
    let route = natural_web_intent_from_user_message("try the web tooling again").expect("route");
    assert_eq!(route.0, "batch_query");
    assert_eq!(
        route.1.get("query").and_then(Value::as_str),
        Some("latest ai developments")
    );
    assert_eq!(route.1.get("source").and_then(Value::as_str), Some("web"));
    assert_eq!(
        route.1.get("diagnostic").and_then(Value::as_str),
        Some("natural_language_web_retry_probe")
    );
}

#[test]
