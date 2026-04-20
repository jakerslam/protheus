fn natural_web_intent_does_not_force_plain_workspace_peer_compare_into_web() {
    assert!(natural_web_intent_from_user_message("compare this system to openclaw").is_none());
    assert!(natural_web_intent_from_user_message("compare openclaw to this system/workspace").is_none());
}

#[test]
