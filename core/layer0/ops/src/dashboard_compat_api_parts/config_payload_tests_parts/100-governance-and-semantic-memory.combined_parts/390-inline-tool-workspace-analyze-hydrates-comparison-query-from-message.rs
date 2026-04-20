fn inline_tool_workspace_analyze_hydrates_comparison_query_from_message() {
    let input = normalize_inline_tool_execution_input(
        "workspace_analyze",
        &json!({}),
        "compare this system (infring) to openclaw",
    );
    assert_eq!(input.get("path").and_then(Value::as_str), Some("."));
    assert_eq!(input.get("full").and_then(Value::as_bool), Some(true));
    assert!(input
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("compare this system"));
}

#[test]
