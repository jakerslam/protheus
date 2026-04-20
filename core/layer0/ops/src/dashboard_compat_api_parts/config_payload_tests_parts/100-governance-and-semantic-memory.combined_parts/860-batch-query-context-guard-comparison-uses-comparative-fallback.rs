fn batch_query_context_guard_comparison_uses_comparative_fallback() {
    let summary = summarize_tool_payload(
        "batch_query",
        &json!({
            "ok": true,
            "status": "ok",
            "query": "compare openclaw to this system/workspace",
            "source": "web",
            "summary": "Context overflow: estimated context size exceeds safe threshold during tool loop."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("infring is strongest"));
    assert!(lowered.contains("source-backed ranked table"));
}

#[test]
