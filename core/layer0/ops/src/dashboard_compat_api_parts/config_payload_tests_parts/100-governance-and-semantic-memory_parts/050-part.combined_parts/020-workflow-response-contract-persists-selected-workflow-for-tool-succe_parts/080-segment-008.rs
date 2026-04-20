#[test]
fn workflow_repair_does_not_resurrect_prior_speculative_web_blocker_copy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-speculative-blocker-resurrection-guard-agent","role":"assistant"}"#,
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
                    "response": "I attempted to run those web searches but the system blocked the function calls from executing entirely. It appears the security controls are preventing any web search operations at the moment, regardless of topic. The system flagged this as an invalid response attempt rather than processing the queries."
                },
                {
                    "response": "I attempted to run those web searches but the system blocked the function calls from executing entirely. It appears the security controls are preventing any web search operations at the moment, regardless of topic. The system flagged this as an invalid response attempt rather than processing the queries."
                }
            ],
            "calls": []
        }),
    );
    let seed = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"try web search and report exactly what happened"}"#,
        &snapshot,
    )
    .expect("seed message response");
    assert_eq!(seed.status, 200);
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {"response": "I'll attempt the web search again to test current behavior."},
                {"response": "I don't have usable tool findings from this turn yet. Ask me to retry with a narrower query or a specific source URL."},
                {"response": "I don't have usable tool findings from this turn yet. Ask me to retry with a narrower query or a specific source URL."}
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"try again"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(
        !response_text.contains("blocked the function calls from executing entirely"),
        "{response_text}"
    );
    assert!(
        !response_text.contains("invalid response attempt"),
        "{response_text}"
    );
    let finalization_outcome = response
        .payload
        .pointer("/response_finalization/outcome")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(
        !finalization_outcome.contains("repaired_with_latest_assistant"),
        "{finalization_outcome}"
    );
}

fn latest_persisted_assistant_text_for_test(root: &Path, agent_id: &str) -> String {
    let state = load_session_state(root, agent_id);
    let active_session_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let sessions = state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for session in sessions {
        let session_id = clean_text(
            session.get("session_id").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        if session_id != active_session_id {
            continue;
        }
        let messages = session
            .get("messages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for row in messages.into_iter().rev() {
            if row.get("role").and_then(Value::as_str) != Some("assistant") {
                continue;
            }
            let text = clean_text(
                row.get("text")
                    .or_else(|| row.get("content"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                8_000,
            );
            if !text.is_empty() {
                return text;
            }
        }
    }
    String::new()
}

fn normalize_test_text_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]

