fn ack_only_detector_flags_key_findings_source_scaffold_copy() {
    assert!(response_looks_like_tool_ack_without_findings(
        "Key findings for \"Infring AI vs competitors comparison 2024\": - Potential sources: hai.stanford.edu, artificialanalysis.ai, epoch.ai."
    ));
}

#[test]
