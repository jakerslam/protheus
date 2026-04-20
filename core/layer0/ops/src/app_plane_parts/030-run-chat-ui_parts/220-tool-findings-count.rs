fn tool_findings_count(row: &Value) -> usize {
    for key in ["findings", "results", "items", "citations", "sources"] {
        if let Some(count) = row
            .get(key)
            .or_else(|| row.pointer(&format!("/result/{key}")))
            .and_then(Value::as_array)
            .map(|rows| rows.len())
        {
            return count;
        }
    }
    0
}
