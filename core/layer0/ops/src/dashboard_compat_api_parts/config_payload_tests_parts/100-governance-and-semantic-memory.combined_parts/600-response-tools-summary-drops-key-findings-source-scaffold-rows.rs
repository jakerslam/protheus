fn response_tools_summary_drops_key_findings_source_scaffold_rows() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "batch_query",
            "is_error": false,
            "result": "Key findings for \"Infring AI vs competitors comparison 2024\": - Potential sources: hai.stanford.edu, artificialanalysis.ai, epoch.ai."
        })],
        4,
    );
    assert!(synthesized.is_empty());
}

#[test]
