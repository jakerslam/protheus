fn workflow_more_tooling_detector_matches_compare_follow_up_question() {
    assert!(workflow_response_requests_more_tooling(
        "Would you like me to search for specific OpenClaw technical documentation or architecture details to enable a more substantive comparison?"
    ));
}

#[test]
