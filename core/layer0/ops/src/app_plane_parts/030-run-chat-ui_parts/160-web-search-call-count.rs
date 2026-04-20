fn chat_ui_web_search_call_count(tools: &[Value]) -> usize {
    tools
        .iter()
        .filter(|row| {
            chat_ui_tool_name_is_web_search(
                row.get("name")
                    .or_else(|| row.get("tool"))
                    .or_else(|| row.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            )
        })
        .count()
}
