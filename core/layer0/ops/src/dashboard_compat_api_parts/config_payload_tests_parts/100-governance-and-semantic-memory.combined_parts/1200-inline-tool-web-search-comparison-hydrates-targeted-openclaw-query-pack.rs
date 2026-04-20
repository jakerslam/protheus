fn inline_tool_web_search_comparison_hydrates_targeted_openclaw_query_pack() {
    let input = normalize_inline_tool_execution_input(
        "web_search",
        &json!({"query":"OpenClaw AI agent system features capabilities"}),
        "compare this system (infring) to openclaw",
    );
    assert_eq!(
        input.get("query").and_then(Value::as_str),
        Some("OpenClaw AI assistant architecture features docs")
    );
    let queries = input
        .get("queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(queries.len() >= 3, "{queries:?}");
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("site:openclaw.ai"))
            .unwrap_or(false)
    }));
}

#[test]
