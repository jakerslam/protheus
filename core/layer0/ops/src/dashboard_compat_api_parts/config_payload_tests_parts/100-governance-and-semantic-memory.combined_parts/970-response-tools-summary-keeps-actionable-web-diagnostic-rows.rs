fn response_tools_summary_keeps_actionable_web_diagnostic_rows() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "batch_query",
            "is_error": false,
            "result": "Search returned no useful comparison findings for infring vs openclaw."
        })],
        4,
    );
    let lowered = synthesized.to_ascii_lowercase();
    assert!(lowered.contains("retrieval-quality miss"));
    assert!(lowered.contains("batch query"));
}

#[test]
