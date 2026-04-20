fn web_fetch_summary_suppresses_example_domain_placeholder_dump() {
    let summary = summarize_tool_payload(
        "web_fetch",
        &json!({
            "ok": true,
            "requested_url": "https://example.com",
            "summary": "Example Domain This domain is for use in documentation examples without needing permission."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("placeholder"));
    assert!(lowered.contains("example.com"));
    assert!(!lowered.contains("without needing permission"));
}

#[test]
