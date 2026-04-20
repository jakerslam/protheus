fn chat_ui_retry_loop_risk_keeps_retry_when_receipts_are_diverse() {
    let loop_risk = chat_ui_retry_loop_risk_from_diagnostics(&json!({
        "execution_receipts": [
            {"status": "ok", "error_code": ""},
            {"status": "error", "error_code": "web_tool_timeout"},
            {"status": "ok", "error_code": ""}
        ]
    }));
    assert_eq!(
        loop_risk.get("detected").and_then(Value::as_bool),
        Some(false),
        "{loop_risk}"
    );
    let (recommended, strategy, lane) = chat_ui_apply_loop_risk_to_retry(
        true,
        "retry_with_backoff",
        "delayed",
        &loop_risk,
    );
    assert!(recommended, "retry should remain available when no loop-risk is detected");
    assert_eq!(strategy, "retry_with_backoff");
    assert_eq!(lane, "delayed");
}

#[cfg(test)]
#[test]
