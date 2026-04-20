fn chat_ui_receipt_status_count(diagnostics: &Value, status: &str) -> i64 {
    diagnostics
        .get("execution_receipts")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| {
                    clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 64)
                        .eq_ignore_ascii_case(status)
                })
                .count() as i64
        })
        .unwrap_or(0)
}
