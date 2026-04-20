fn system_diagnostic_failure_summary_is_not_generic_dead_end() {
    let summary = user_facing_tool_failure_summary(
        "system_diagnostic",
        &json!({"ok": false, "error": "request_read_failed"}),
    )
    .expect("summary");
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("diagnose manually"));
    assert!(!lowered.contains("couldn't complete `system_diagnostic` right now"));
}

#[test]
