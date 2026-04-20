fn chat_ui_surface_error_code_hint_from_row(row: &Value) -> Option<String> {
    let mut saw_degraded = false;
    let mut saw_unavailable = false;
    for candidate in [
        clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 240),
        clean(row.get("error").and_then(Value::as_str).unwrap_or(""), 240),
        clean(row.get("result").and_then(Value::as_str).unwrap_or(""), 1_200),
        chat_ui_tool_text_blob(row),
    ] {
        let code = crate::tool_output_match_filter::normalize_web_tooling_error_code(&candidate);
        if code == "web_tool_surface_unavailable" {
            saw_unavailable = true;
        } else if code == "web_tool_surface_degraded" {
            saw_degraded = true;
        }
    }
    if saw_unavailable {
        Some("web_tool_surface_unavailable".to_string())
    } else if saw_degraded {
        Some("web_tool_surface_degraded".to_string())
    } else {
        None
    }
}
