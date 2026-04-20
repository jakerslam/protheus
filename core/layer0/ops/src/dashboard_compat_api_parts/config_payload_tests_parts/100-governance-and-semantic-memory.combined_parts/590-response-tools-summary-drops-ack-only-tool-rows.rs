fn response_tools_summary_drops_ack_only_tool_rows() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "web_search",
            "is_error": false,
            "result": "Web search completed."
        })],
        4,
    );
    assert!(synthesized.is_empty());
}

#[test]
