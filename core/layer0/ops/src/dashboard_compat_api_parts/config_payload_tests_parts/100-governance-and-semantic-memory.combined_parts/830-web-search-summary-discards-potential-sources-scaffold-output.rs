fn web_search_summary_discards_potential_sources_scaffold_output() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "Infring AI agent platform capabilities features 2024",
            "summary": "Key findings for \"Infring AI agent platform capabilities features 2024\":\n- Potential sources: nlplogix.com, gartner.com, insightpartners.com.\n- Potential sources: salesforce.com, microsoft.com, lyzr.ai."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(!lowered.contains("potential sources:"));
    assert!(!lowered.contains("key findings for"));
    assert!(lowered.contains("low-signal") || lowered.contains("no extractable findings"));
}

#[test]
