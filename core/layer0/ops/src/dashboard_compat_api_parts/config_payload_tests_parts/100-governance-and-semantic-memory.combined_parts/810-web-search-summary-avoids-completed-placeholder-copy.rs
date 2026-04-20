fn web_search_summary_avoids_completed_placeholder_copy() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "agent reliability",
            "summary": "safe search region picker noise only"
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(!lowered.contains("web search completed"));
    assert!(!lowered.contains("completed."));
    assert!(!lowered.trim().is_empty());
}

#[test]
