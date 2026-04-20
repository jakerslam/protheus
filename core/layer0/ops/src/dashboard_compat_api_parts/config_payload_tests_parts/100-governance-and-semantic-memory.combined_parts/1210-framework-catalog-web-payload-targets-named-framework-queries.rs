fn framework_catalog_web_payload_targets_named_framework_queries() {
    let payload = framework_catalog_web_payload_from_query("top AI agentic frameworks")
        .expect("framework payload");
    assert_eq!(payload.get("source").and_then(Value::as_str), Some("web"));
    let query = payload.get("query").and_then(Value::as_str).unwrap_or("");
    assert!(query.contains("top AI agent frameworks"), "{query}");
    assert!(query.contains("LangGraph"), "{query}");
    assert!(query.contains("OpenAI Agents SDK"), "{query}");
    assert!(query.contains("official docs"), "{query}");
    assert!(!query.contains("vs"), "{query}");
    let queries = payload
        .get("queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(queries.len() >= 6, "{queries:?}");
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("CrewAI"))
            .unwrap_or(false)
    }));
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("landscape"))
            .unwrap_or(false)
    }));
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("site:openai.github.io/openai-agents-python"))
            .unwrap_or(false)
    }));
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("site:microsoft.github.io"))
            .unwrap_or(false)
    }));
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("site:github.com huggingface/smolagents"))
            .unwrap_or(false)
    }));
}

#[test]
