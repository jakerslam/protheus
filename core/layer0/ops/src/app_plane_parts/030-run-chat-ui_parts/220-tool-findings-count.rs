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
    let result_text = clean(
        row.get("result")
            .or_else(|| row.pointer("/result/summary"))
            .or_else(|| row.pointer("/result/text"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        2_000,
    );
    if !result_text.is_empty()
        && !crate::tool_output_match_filter::matches_ack_placeholder(&result_text)
        && !result_text.contains("<function=")
    {
        return 1;
    }
    0
}
