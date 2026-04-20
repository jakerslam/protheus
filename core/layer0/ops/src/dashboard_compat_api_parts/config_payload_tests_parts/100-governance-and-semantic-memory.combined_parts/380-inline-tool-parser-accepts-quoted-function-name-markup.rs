fn inline_tool_parser_accepts_quoted_function_name_markup() {
    let response = "<function=\"web_search\">{\"query\":\"top AI agentic frameworks 2024\",\"source\":\"web\",\"aperture\":\"medium\"}</function>";
    let (cleaned, calls) = extract_inline_tool_calls(response, 4);
    assert!(cleaned.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "web_search");
    assert_eq!(calls[0].1.get("query").and_then(Value::as_str), Some("top AI agentic frameworks 2024"));
}

#[test]
