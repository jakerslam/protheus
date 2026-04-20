fn batch_query_summary_rewrites_unsynthesized_domain_dump_to_structured_evidence() {
    let summary = summarize_tool_payload(
        "batch_query",
        &json!({
            "ok": true,
            "status": "ok",
            "summary": "Web benchmark synthesis: bing.com: compare [A with B] vs compare A [with B] | WordReference Forums — https://forum.wordreference.com/threads/compare-a-with-b-vs-compare-a-with-b.4047424/",
            "evidence_refs": [
                {
                    "title": "OpenClaw — Personal AI Assistant",
                    "locator": "https://openclaw.ai/"
                }
            ]
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("batch query evidence"));
    assert!(!lowered.contains("bing.com: compare"));
}

#[test]
