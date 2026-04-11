use super::*;

#[test]
fn template_governance_handles_missing_contract_with_receipt() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    let parsed = ParsedArgs {
        flags: std::collections::HashMap::new(),
        positional: Vec::new(),
    };
    let payload = run_template_governance(root, &parsed, false);
    assert_eq!(
        payload.get("type").and_then(Value::as_str),
        Some("research_plane_template_governance")
    );
    assert!(payload.get("receipt_hash").is_some());
}

#[test]
fn pipeline_rejects_unknown_stage_when_strict() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    let parsed = ParsedArgs {
        flags: std::collections::HashMap::from([
            ("items-json".to_string(), r#"[{"url":"https://example.com","title":"Example"}]"#.to_string()),
            ("pipeline-json".to_string(), r#"[{"stage":"ship-it"}]"#.to_string()),
        ]),
        positional: Vec::new(),
    };
    let payload = run_pipeline(root, &parsed, true);
    assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(false));
    assert!(payload
        .get("errors")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().any(|row| row.as_str() == Some("pipeline_stage_not_allowed:ship-it")))
        .unwrap_or(false));
}

#[test]
fn pipeline_csv_export_escapes_commas_and_quotes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    let export_path = root.join("artifacts").join("items.csv");
        let parsed = ParsedArgs {
        flags: std::collections::HashMap::from([
            (
                "items-json".to_string(),
                r#"[{"url":"https://example.com","title":"Hello, \"world\""}]"#.to_string(),
            ),
            ("pipeline-json".to_string(), r#"[{"stage":"validate","required_fields":["url","title"]}]"#.to_string()),
            ("export-format".to_string(), "csv".to_string()),
            ("export-path".to_string(), export_path.display().to_string()),
        ]),
        positional: Vec::new(),
    };
    let payload = run_pipeline(root, &parsed, true);
    assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
    let body = std::fs::read_to_string(&export_path).expect("csv");
    assert!(body.contains("\"Hello, \"\"world\"\"\""));
    assert!(!body.contains("\\\"world\\\""));
}
