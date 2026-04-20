fn natural_web_intent_does_not_auto_route_peer_comparisons_without_explicit_web_request() {
    let route = natural_web_intent_from_user_message(
        "Compare Infring to its competitors and rank it among peers in a table",
    );
    assert!(route.is_none());
}

#[test]
