fn chat_ui_low_signal_hint_from_row(row: &Value) -> bool {
    for candidate in [
        clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 240),
        clean(row.get("error").and_then(Value::as_str).unwrap_or(""), 240),
        clean(row.get("result").and_then(Value::as_str).unwrap_or(""), 1_200),
        chat_ui_tool_text_blob(row),
    ] {
        let code = crate::tool_output_match_filter::normalize_web_tooling_error_code(&candidate);
        if code == "web_tool_low_signal" {
            return true;
        }
    }
    false
}
