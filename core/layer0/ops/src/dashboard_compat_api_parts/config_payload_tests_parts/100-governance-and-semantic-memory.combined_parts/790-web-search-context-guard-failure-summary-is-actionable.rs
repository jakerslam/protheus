fn web_search_context_guard_failure_summary_is_actionable() {
    let summary = user_facing_tool_failure_summary(
        "web_search",
        &json!({"ok": false, "error": "Context overflow: estimated context size exceeds safe threshold during tool loop."}),
    )
    .expect("summary");
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("fit safely in context"));
    assert!(lowered.contains("narrower query"));
}

#[test]
