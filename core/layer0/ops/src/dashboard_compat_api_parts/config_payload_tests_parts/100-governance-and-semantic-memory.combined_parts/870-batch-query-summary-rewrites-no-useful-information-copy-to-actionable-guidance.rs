fn batch_query_summary_rewrites_no_useful_information_copy_to_actionable_guidance() {
    let summary = summarize_tool_payload(
        "batch_query",
        &json!({
            "ok": true,
            "status": "ok",
            "query": "top AI agent frameworks",
            "summary": "Search returned no useful information."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(
        lowered.contains("usable tool findings") || lowered.contains("source-backed findings"),
        "unexpected summary: {summary}"
    );
    assert!(!lowered.contains("search returned no useful information"));
}

#[test]
