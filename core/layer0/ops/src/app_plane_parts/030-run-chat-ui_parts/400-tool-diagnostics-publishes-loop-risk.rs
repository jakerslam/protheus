fn chat_ui_tool_diagnostics_publishes_loop_risk() {
    let diagnostics = chat_ui_tool_diagnostics(&[
        json!({"tool": "batch_query", "status": "low_signal", "error": "web_tool_low_signal"}),
        json!({"tool": "batch_query", "status": "low_signal", "error": "web_tool_low_signal"}),
        json!({"tool": "batch_query", "status": "low_signal", "error": "web_tool_low_signal"}),
    ]);
    assert_eq!(
        diagnostics
            .pointer("/loop_risk/detected")
            .and_then(Value::as_bool),
        Some(true),
        "{diagnostics}"
    );
    assert_eq!(
        diagnostics
            .pointer("/loop_risk/max_duplicate_signature_count")
            .and_then(Value::as_i64),
        Some(3),
        "{diagnostics}"
    );
}
