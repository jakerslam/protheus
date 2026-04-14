#[test]
fn classify_detects_publish_publicly() {
    let classification = classify_value(payload_obj(&json!({
        "tool_name": "gh",
        "command_text": "publish blog post to medium"
    })));
    assert_eq!(
        classification.get("type").and_then(Value::as_str),
        Some(ACTION_PUBLISH_PUBLICLY)
    );
    assert_eq!(
        classification.get("risk").and_then(Value::as_str),
        Some(RISK_HIGH)
    );
}

#[test]
fn auto_classify_preserves_tags_and_summary_shape() {
    let out = run_command(
        "auto-classify",
        payload_obj(&json!({
            "tool_name": "bash",
            "command_text": "rm -rf tmp/build",
            "payload": { "path": "tmp/build" }
        })),
    )
    .unwrap();
    let envelope = out.get("envelope").unwrap();
    assert_eq!(
        envelope.get("type").and_then(Value::as_str),
        Some(ACTION_DELETE_DATA)
    );
    assert_eq!(
        envelope.pointer("/tags/0").and_then(Value::as_str),
        Some(ACTION_DELETE_DATA)
    );
    assert!(envelope
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("")
        .starts_with("delete_data:"));
}
