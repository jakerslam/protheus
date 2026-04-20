fn comparative_detector_matches_peer_ranking_language() {
    assert!(message_requests_comparative_answer(
        "find out how Infring ranks among its peers"
    ));
    assert!(message_requests_comparative_answer(
        "compare infring versus top competitors"
    ));
    assert!(!message_requests_comparative_answer(
        "top AI agent frameworks"
    ));
}

#[test]
