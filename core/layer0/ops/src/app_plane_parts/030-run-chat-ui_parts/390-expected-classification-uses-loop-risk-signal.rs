fn chat_ui_expected_classification_uses_loop_risk_signal() {
    let classification = chat_ui_expected_classification_from_diagnostics(
        &json!({
            "total_calls": 3,
            "execution_receipts": [
                {"status": "low_signal", "error_code": "web_tool_low_signal"},
                {"status": "low_signal", "error_code": "web_tool_low_signal"},
                {"status": "low_signal", "error_code": "web_tool_low_signal"}
            ],
            "loop_risk": {
                "detected": true
            }
        }),
        true,
        3,
    );
    assert_eq!(classification, "low_signal");
}
