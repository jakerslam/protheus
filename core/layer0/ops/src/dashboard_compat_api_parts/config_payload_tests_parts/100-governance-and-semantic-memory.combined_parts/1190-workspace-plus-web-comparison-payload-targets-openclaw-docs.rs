fn workflow_does_not_hydrate_openclaw_comparison_payload() {
    let input = normalize_inline_tool_execution_input(
        "web_search",
        &json!({"query":"OpenClaw AI agent system features capabilities"}),
        "compare this system to a named external system",
    );
    assert_eq!(
        input.get("query").and_then(Value::as_str),
        Some("OpenClaw AI agent system features capabilities")
    );
    assert!(input.get("queries").is_none(), "{input}");
}
