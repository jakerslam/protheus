fn comparative_live_web_detector_matches_openclaw_vs_workspace_language() {
    assert!(message_requests_live_web_comparison(
        "compare this system (infring) to openclaw with web sources"
    ));
    assert!(message_requests_live_web_comparison(
        "compare openclaw to this system/workspace using web search"
    ));
}

#[test]
