fn ack_only_detector_flags_speculative_web_blocker_explanations() {
    let draft = "I understand you're looking for a comparison between this platform and OpenClaw, but I'm currently unable to access web search functionality to gather the necessary information. The system is blocking tool execution attempts, which prevents me from retrieving current details.\n\nBased on the system behavior I'm observing, likely reasons include Configuration Restrictions, Authentication Issues, Rate Limiting, or intentional sandboxed design.";
    assert!(response_looks_like_tool_ack_without_findings(draft));
}

#[test]
