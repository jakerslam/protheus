#[test]
fn compare_workflow_completes_missing_web_evidence_from_latent_candidates() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"compare-workflow-latent-agent","role":"researcher"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "<function=workspace_analyze>{\"path\":\".\",\"query\":\"compare this system (infring) to openclaw\",\"full\":true}</function>"
                },
                {
                    "response": "Using both local and external evidence, Infring centers workflow-gated synthesis while OpenClaw emphasizes governed web/media tooling."
                }
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "terminal_exec",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "Local workspace evidence shows workflow-gated synthesis via complex_prompt_chain_v1."
                    }
                },
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "External web evidence highlights OpenClaw's governed web/media tooling."
                    }
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"compare this system (infring) to openclaw"}"#,
        &snapshot,
    )
    .expect("message response");
    let tool_names = response
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get("name").and_then(Value::as_str).map(ToString::to_string))
        .collect::<Vec<_>>();
    assert_eq!(tool_names, vec!["workspace_analyze".to_string(), "batch_query".to_string()]);
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(response_text.contains("Infring"), "{response_text}");
    assert!(response_text.contains("OpenClaw"), "{response_text}");
}

#[test]
fn workflow_more_tooling_detector_matches_compare_follow_up_question() {
    assert!(workflow_response_requests_more_tooling(
        "Would you like me to search for specific OpenClaw technical documentation or architecture details to enable a more substantive comparison?"
    ));
}

#[test]
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
