#[test]
fn execute_tool_recovery_emits_nexus_connection_metadata() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let out = execute_tool_call_with_recovery(
        root.path(),
        &snapshot,
        "agent-nexus-route",
        None,
        "file_read",
        &json!({"path":"README.md"}),
    );
    assert!(out.get("nexus_connection").is_some());
    assert_eq!(
        out.pointer("/nexus_connection/source")
            .and_then(Value::as_str),
        Some("client_ingress")
    );
    assert_eq!(
        out.pointer("/nexus_connection/delivery/allowed")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        out.pointer("/tool_pipeline/normalized_result/tool_name")
            .and_then(Value::as_str),
        Some("file_read")
    );
}

#[test]
fn summarize_tool_payload_prefers_claim_bundle_findings_when_available() {
    let payload = json!({
        "ok": true,
        "summary": "raw summary should not win",
        "tool_pipeline": {
            "claim_bundle": {
                "claims": [
                    {"status":"supported","text":"Framework A shows higher task completion consistency under constrained retries."},
                    {"status":"partial","text":"Framework B has better ecosystem coverage but weaker deterministic controls."},
                    {"status":"unsupported","text":"ignore me"}
                ]
            }
        }
    });
    let summary = summarize_tool_payload("web_search", &payload);
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.starts_with("key findings:"));
    assert!(lowered.contains("framework a"));
    assert!(lowered.contains("framework b"));
    assert!(!lowered.contains("ignore me"));
}
