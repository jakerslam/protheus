fn response_tools_summary_preserves_batch_query_domains_in_key_findings() {
    let summary = response_tools_summary_for_user(
        &[json!({
            "name": "batch_query",
            "result": "Key findings: docs.openclaw.ai: OpenClaw is a self-hosted gateway that connects chat apps to AI; openclaw.ai: OpenClaw personal assistant overview."
        })],
        4,
    );
    assert!(summary.contains("docs.openclaw.ai"), "{summary}");
    assert_ne!(summary.trim(), "Here's what I found:\n- batch query: docs.");
}

#[test]
