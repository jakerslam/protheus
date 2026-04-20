fn natural_web_intent_routes_test_web_fetch_probe_to_example_dot_com() {
    let route = natural_web_intent_from_user_message("do a test web fetch").expect("route");
    assert_eq!(route.0, "web_fetch");
    assert_eq!(
        route.1.get("url").and_then(Value::as_str),
        Some("https://example.com")
    );
    assert_eq!(
        route.1.get("diagnostic").and_then(Value::as_str),
        Some("natural_language_test_web_fetch")
    );
}

#[test]
