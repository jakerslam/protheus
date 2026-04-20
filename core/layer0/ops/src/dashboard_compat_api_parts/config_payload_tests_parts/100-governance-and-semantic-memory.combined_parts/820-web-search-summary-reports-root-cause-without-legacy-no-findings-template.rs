fn web_search_summary_reports_root_cause_without_legacy_no_findings_template() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "ai assistant systems comparison 2024 capabilities landscape",
            "requested_url": "https://duckduckgo.com/html/?q=ai+assistant+systems+comparison",
            "domain": "duckduckgo.com",
            "summary": "AI assistant systems comparison 2024 capabilities landscape at DuckDuckGo All Regions Argentina Australia Safe search Any time"
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("low-signal"));
    assert!(lowered.contains("batch_query"));
    assert!(!lowered.contains("search response came from"));
    assert!(!lowered.contains("couldn't extract usable findings"));
}

#[test]
