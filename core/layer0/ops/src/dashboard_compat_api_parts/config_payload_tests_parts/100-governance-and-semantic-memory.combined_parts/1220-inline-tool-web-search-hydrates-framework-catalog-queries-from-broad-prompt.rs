fn inline_tool_web_search_hydrates_framework_catalog_queries_from_broad_prompt() {
    let input = normalize_inline_tool_execution_input(
        "web_search",
        &json!({"query":"top AI agentic frameworks"}),
        "Try to web search \"top AI agentic frameworks\" and return the results",
    );
    let query = input.get("query").and_then(Value::as_str).unwrap_or("");
    assert!(query.contains("LangGraph"), "{query}");
    let queries = input
        .get("queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(queries.len() >= 6, "{queries:?}");
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("site:openai.github.io/openai-agents-python"))
            .unwrap_or(false)
    }));
}

#[test]
