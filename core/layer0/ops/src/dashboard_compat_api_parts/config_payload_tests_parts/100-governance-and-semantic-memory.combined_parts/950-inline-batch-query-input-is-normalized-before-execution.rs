fn inline_batch_query_input_is_normalized_before_execution() {
    let normalized = normalize_inline_tool_execution_input(
        "batch_query",
        &json!({
            "query": "Try to web search \"top AI agentic frameworks\" and return the results"
        }),
        "Try to web search \"top AI agentic frameworks\" and return the results",
    );
    assert_eq!(
        normalized.get("query").and_then(Value::as_str),
        Some("top AI agentic frameworks")
    );
    assert_eq!(normalized.get("source").and_then(Value::as_str), Some("web"));
    assert_eq!(
        normalized.get("aperture").and_then(Value::as_str),
        Some("medium")
    );
}

#[test]
