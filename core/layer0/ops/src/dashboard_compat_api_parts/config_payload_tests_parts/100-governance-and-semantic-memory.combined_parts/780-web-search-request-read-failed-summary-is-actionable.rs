fn web_search_request_read_failed_summary_is_actionable() {
    let summary = user_facing_tool_failure_summary(
        "web_search",
        &json!({"ok": false, "error": "request_read_failed:Resource temporarily unavailable (os error 35)"}),
    )
    .expect("summary");
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("retry transient failures"));
    assert!(lowered.contains("doctor --json"));
    assert!(lowered.contains("request_read_failed"));
    assert_eq!(
        deterministic_tool_retry_backoff_ms("web_search"),
        vec![180, 360, 720]
    );
}

#[test]
