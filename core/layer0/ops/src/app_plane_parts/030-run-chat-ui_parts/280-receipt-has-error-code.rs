fn chat_ui_receipt_has_error_code(diagnostics: &Value, error_code: &str) -> bool {
    diagnostics
        .get("execution_receipts")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                clean(row.get("error_code").and_then(Value::as_str).unwrap_or(""), 128)
                    .eq_ignore_ascii_case(error_code)
            })
        })
        .unwrap_or(false)
}
