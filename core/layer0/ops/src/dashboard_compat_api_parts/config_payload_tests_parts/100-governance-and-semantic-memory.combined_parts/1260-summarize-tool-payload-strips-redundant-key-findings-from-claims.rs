fn summarize_tool_payload_strips_redundant_key_findings_from_claims() {
    let summary = summarize_tool_payload(
        "batch_query",
        &json!({
            "ok": true,
            "tool_pipeline": {
                "claim_bundle": {
                    "claims": [
                        {
                            "status": "supported",
                            "text": "Key findings: openclaw.ai: OpenClaw — Personal AI Assistant"
                        }
                    ]
                }
            }
        }),
    );
    assert_eq!(
        summary,
        "Key findings: openclaw.ai: OpenClaw — Personal AI Assistant"
    );
}

#[test]
