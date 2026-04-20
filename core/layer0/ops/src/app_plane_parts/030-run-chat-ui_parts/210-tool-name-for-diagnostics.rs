fn tool_name_for_diagnostics(row: &Value) -> String {
    clean(
        row.get("tool")
            .or_else(|| row.get("name"))
            .or_else(|| row.get("type"))
            .or_else(|| row.pointer("/tool/name"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    )
    .to_ascii_lowercase()
}
