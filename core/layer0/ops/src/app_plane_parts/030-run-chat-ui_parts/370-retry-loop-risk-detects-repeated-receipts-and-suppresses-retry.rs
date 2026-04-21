fn chat_ui_retry_loop_risk_detects_repeated_receipts_and_suppresses_retry() {
    let loop_risk = chat_ui_retry_loop_risk_from_diagnostics(&json!({
        "execution_receipts": [
            {"status": "low_signal", "error_code": "web_tool_low_signal"},
            {"status": "low_signal", "error_code": "web_tool_low_signal"},
            {"status": "low_signal", "error_code": "web_tool_low_signal"}
        ]
    }));
    assert_eq!(
        loop_risk.get("detected").and_then(Value::as_bool),
        Some(true),
        "{loop_risk}"
    );
    assert_eq!(
        loop_risk
            .get("max_duplicate_signature_count")
            .and_then(Value::as_i64),
        Some(3),
        "{loop_risk}"
    );
    let (recommended, strategy, lane) = chat_ui_apply_loop_risk_to_retry(
        true,
        "narrow_query",
        "immediate",
        &loop_risk,
    );
    assert!(!recommended, "retry should be suppressed when loop-risk is detected");
    assert_eq!(strategy, "halt_on_loop_risk");
    assert_eq!(lane, "manual_intervention");
}
