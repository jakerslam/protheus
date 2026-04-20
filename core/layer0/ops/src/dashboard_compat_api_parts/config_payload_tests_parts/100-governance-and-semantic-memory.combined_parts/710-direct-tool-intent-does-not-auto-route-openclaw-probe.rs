fn direct_tool_intent_does_not_auto_route_openclaw_probe() {
    assert!(
        direct_tool_intent_from_user_message(
            "compare openclaw to this system/workspace and do a test web fetch"
        )
        .is_none()
    );
}

#[test]
