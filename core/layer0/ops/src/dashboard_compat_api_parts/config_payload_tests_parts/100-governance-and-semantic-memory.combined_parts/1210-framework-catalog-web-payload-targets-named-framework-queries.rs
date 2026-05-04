fn framework_catalog_query_pack_is_not_runtime_hydrated() {
    let input = normalize_inline_tool_execution_input(
        "web_search",
        &json!({"query":"top AI agentic frameworks"}),
        "Try to web search \"top AI agentic frameworks\" and return the results",
    );
    assert_eq!(
        input.get("query").and_then(Value::as_str),
        Some("top AI agentic frameworks")
    );
    assert!(input.get("queries").is_none(), "{input}");
}
