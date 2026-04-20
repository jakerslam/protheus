fn workspace_plus_web_comparison_payload_targets_openclaw_docs() {
    let payload = workspace_plus_web_comparison_web_payload_from_message(
        "compare this system (infring) to openclaw",
    )
    .expect("comparison payload");
    assert_eq!(payload.get("source").and_then(Value::as_str), Some("web"));
    assert_eq!(
        payload.get("query").and_then(Value::as_str),
        Some("OpenClaw AI assistant architecture features docs")
    );
    let queries = payload
        .get("queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!queries.is_empty());
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("site:openclaw.ai"))
            .unwrap_or(false)
    }));
}

#[test]
