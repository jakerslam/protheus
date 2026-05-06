fn create_context_test_agent(root: &std::path::Path) -> String {
    let created = handle(
        root,
        "POST",
        "/api/agents",
        br#"{"name":"CtxProbe","role":"analyst"}"#,
        &json!({"ok": true}),
    )
    .expect("create agent");
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());
    agent_id
}

fn synthetic_session_messages(
    count: usize,
    prefix: &str,
    repeated_token: &str,
    repeat_count: usize,
) -> Vec<Value> {
    (0..count)
        .map(|idx| {
            json!({
                "id": idx + 1,
                "role": if idx % 2 == 0 { "user" } else { "agent" },
                "text": format!("{prefix}-{idx} {}", format!("{repeated_token} ").repeat(repeat_count)),
                "ts": crate::now_iso()
            })
        })
        .collect::<Vec<_>>()
}

fn write_agent_session_messages(root: &std::path::Path, agent_id: &str, messages: Vec<Value>) {
    let session_path = state_path(root, AGENT_SESSIONS_DIR_REL).join(format!("{agent_id}.json"));
    write_json(
        &session_path,
        &json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [
                {
                    "session_id": "default",
                    "updated_at": crate::now_iso(),
                    "messages": messages
                }
            ]
        }),
    );
}

#[test]
fn context_command_reports_isolated_usage_for_fresh_agent() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());

    let context = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/command"),
        br#"{"command":"context","silent":true}"#,
        &json!({"ok": true}),
    )
    .expect("context command");
    assert_eq!(context.status, 200);
    assert_eq!(
        context.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        context
            .payload
            .get("context_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(-1),
        0
    );
    assert_eq!(
        context
            .payload
            .get("context_ratio")
            .and_then(Value::as_f64)
            .unwrap_or(1.0)
            <= 0.01,
        true
    );
    assert_eq!(
        context
            .payload
            .get("context_pressure")
            .and_then(Value::as_str),
        Some("low")
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/pre_generation_pruning_enabled")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn context_command_does_not_mutate_session_history_by_default() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());
    write_agent_session_messages(
        root.path(),
        &agent_id,
        synthetic_session_messages(220, "history-preserve", "token", 800),
    );

    let before_state = load_session_state(root.path(), &agent_id);
    let before_count = all_session_messages(&before_state).len();
    assert_eq!(before_count > 0, true);

    let context = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/command"),
        br#"{"command":"context","silent":true,"context_pool_limit_tokens":32000}"#,
        &json!({"ok": true}),
    )
    .expect("context command");
    assert_eq!(context.status, 200);

    let after_state = load_session_state(root.path(), &agent_id);
    let after_count = all_session_messages(&after_state).len();
    assert_eq!(after_count, before_count);
}

#[test]
fn message_auto_compacts_when_context_usage_reaches_threshold() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());
    let _ = update_profile_patch(
        root.path(),
        &agent_id,
        &json!({"context_window": 512, "context_window_tokens": 512}),
    );
    write_agent_session_messages(
        root.path(),
        &agent_id,
        synthetic_session_messages(80, "context-bloat", "alpha", 40),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"run context compaction","active_context_target_tokens":512,"active_context_min_recent_messages":4,"auto_compact_threshold_ratio":0.95,"auto_compact_target_ratio":0.45}"#,
        &json!({"ok": true}),
    )
    .expect("message");
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/context_pool/emergency_compact/triggered")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        response
            .payload
            .pointer("/context_pool/emergency_compact/persisted_to_history")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/context_pool/emergency_compact/removed_messages")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0,
        true
    );
    assert_eq!(
        response
            .payload
            .get("context_ratio")
            .and_then(Value::as_f64)
            .unwrap_or(1.0)
            > 0.0,
        true
    );

    let context_after = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/command"),
        br#"{"command":"context","silent":true}"#,
        &json!({"ok": true}),
    )
    .expect("context after");
    assert_eq!(context_after.status, 200);
    let session_after_state = load_session_state(root.path(), &agent_id);
    let message_count_after = all_session_messages(&session_after_state).len();
    assert_eq!(message_count_after >= 80, true);
}

#[test]
fn context_command_prunes_pool_when_limit_exceeded() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());
    write_agent_session_messages(
        root.path(),
        &agent_id,
        synthetic_session_messages(260, "pool-prune", "token", 220),
    );

    let context = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/command"),
        br#"{"command":"context","silent":true,"context_pool_limit_tokens":32000}"#,
        &json!({"ok": true}),
    )
    .expect("context command");
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/pre_generation_pruning_enabled")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/pre_generation_pruned")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/pool_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX)
            <= 32000,
        true
    );
}
