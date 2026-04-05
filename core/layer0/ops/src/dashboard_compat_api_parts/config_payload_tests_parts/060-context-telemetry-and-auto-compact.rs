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
    let session_path = state_path(root.path(), AGENT_SESSIONS_DIR_REL).join(format!("{agent_id}.json"));
    let dense_messages = (0..220)
        .map(|idx| {
            json!({
                "id": idx + 1,
                "role": if idx % 2 == 0 { "user" } else { "agent" },
                "text": format!("history-preserve-{idx} {}", "token ".repeat(800)),
                "ts": crate::now_iso()
            })
        })
        .collect::<Vec<_>>();
    write_json(
        &session_path,
        &json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [
                {
                    "session_id": "default",
                    "updated_at": crate::now_iso(),
                    "messages": dense_messages
                }
            ]
        }),
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
    let session_path = state_path(root.path(), AGENT_SESSIONS_DIR_REL).join(format!("{agent_id}.json"));
    let noisy_messages = (0..80)
        .map(|idx| {
            json!({
                "id": idx + 1,
                "role": if idx % 2 == 0 { "user" } else { "agent" },
                "text": format!("context-bloat-{idx} {}", "alpha ".repeat(40)),
                "ts": crate::now_iso()
            })
        })
        .collect::<Vec<_>>();
    write_json(
        &session_path,
        &json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [
                {
                    "session_id": "default",
                    "updated_at": crate::now_iso(),
                    "messages": noisy_messages
                }
            ]
        }),
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
    let session_path = state_path(root.path(), AGENT_SESSIONS_DIR_REL).join(format!("{agent_id}.json"));
    let dense_messages = (0..260)
        .map(|idx| {
            json!({
                "id": idx + 1,
                "role": if idx % 2 == 0 { "user" } else { "agent" },
                "text": format!("pool-prune-{idx} {}", "token ".repeat(220)),
                "ts": crate::now_iso()
            })
        })
        .collect::<Vec<_>>();
    write_json(
        &session_path,
        &json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [
                {
                    "session_id": "default",
                    "updated_at": crate::now_iso(),
                    "messages": dense_messages
                }
            ]
        }),
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

#[test]
fn context_command_emergency_compacts_before_saturation() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());
    let _ = update_profile_patch(
        root.path(),
        &agent_id,
        &json!({"context_window": 512, "context_window_tokens": 512}),
    );
    let session_path = state_path(root.path(), AGENT_SESSIONS_DIR_REL).join(format!("{agent_id}.json"));
    let noisy_messages = (0..120)
        .map(|idx| {
            json!({
                "id": idx + 1,
                "role": if idx % 2 == 0 { "user" } else { "agent" },
                "text": format!("context-pressure-{idx} {}", "alpha ".repeat(80)),
                "ts": crate::now_iso()
            })
        })
        .collect::<Vec<_>>();
    write_json(
        &session_path,
        &json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [
                {
                    "session_id": "default",
                    "updated_at": crate::now_iso(),
                    "messages": noisy_messages
                }
            ]
        }),
    );

    let context = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/command"),
        br#"{"command":"context","silent":true,"active_context_target_tokens":512,"active_context_min_recent_messages":4,"auto_compact_threshold_ratio":0.95,"auto_compact_target_ratio":0.45}"#,
        &json!({"ok": true}),
    )
    .expect("context command");
    assert_eq!(context.status, 200);
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/emergency_compact/triggered")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/emergency_compact/removed_messages")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0,
        true
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/emergency_compact/after_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX)
            < context
                .payload
                .pointer("/context_pool/emergency_compact/before_tokens")
                .and_then(Value::as_i64)
                .unwrap_or(i64::MIN),
        true
    );
}

#[test]
fn message_ignores_unrelated_passive_memory_when_term_index_is_missing() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());
    let attention_path = root
        .path()
        .join("client/runtime/local/state/attention/queue.jsonl");
    if let Some(parent) = attention_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let unrelated = json!({
        "ts": crate::now_iso(),
        "source": format!("agent:{agent_id}"),
        "source_type": "passive_memory_turn",
        "severity": "info",
        "summary": "SQL-Data-Exploration Data Exploration in SQL for Covid-19 Data Project Overview Data Source Tools Used",
        "raw_event": {
            "agent_id": agent_id,
            "memory_kind": "passive_turn",
            "user_text": "legacy row without indexed terms",
            "assistant_text": "legacy row without indexed terms"
        }
    });
    let encoded = serde_json::to_string(&unrelated).expect("encode attention row");
    std::fs::write(&attention_path, format!("{encoded}\n")).expect("write attention queue");

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"code me a reverse linked list"}"#,
        &json!({"ok": true}),
    )
    .expect("message");
    assert_eq!(response.status, 200);
    let text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(
        !text.contains("sql-data-exploration"),
        "unrelated passive-memory project summary should never leak into response text"
    );
    assert!(
        !text.contains("project overview"),
        "template-section drift should be filtered before prompt assembly"
    );
    assert!(
        !text.contains("covid-19"),
        "legacy unrelated context row should not steer coding request replies"
    );
}

#[test]
fn context_defaults_to_minimum_recent_window_floor() {
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
        context
            .payload
            .pointer("/context_pool/min_recent_messages")
            .and_then(Value::as_u64),
        Some(28),
    );
}

#[test]
fn memory_recall_prefers_active_session_earliest_turn_for_first_chat_queries() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());
    let _ = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"first-marker-alpha: we discussed memory continuity"}"#,
        &json!({"ok": true}),
    )
    .expect("seed first");
    let _ = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"later-marker-beta: we then discussed tool routing"}"#,
        &json!({"ok": true}),
    )
    .expect("seed second");
    let recall = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"can you remember our first chat?"}"#,
        &json!({"ok": true}),
    )
    .expect("recall");
    let text = recall
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(text.contains("first-marker-alpha"));
}

#[test]
fn memory_recall_stays_scoped_to_active_session_history() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());
    let _ = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"default-session-marker-alpha"}"#,
        &json!({"ok": true}),
    )
    .expect("seed default");
    let second = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/sessions"),
        br#"{"label":"Other"}"#,
        &json!({"ok": true}),
    )
    .expect("create second");
    let sid = clean_text(
        second
            .payload
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    assert!(!sid.is_empty());
    let _ = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/sessions/{sid}/switch"),
        &[],
        &json!({"ok": true}),
    )
    .expect("switch second");
    let _ = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"other-session-marker-beta"}"#,
        &json!({"ok": true}),
    )
    .expect("seed second");
    let _ = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/sessions/default/switch"),
        &[],
        &json!({"ok": true}),
    )
    .expect("switch default");
    let recall = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"remember our first chat in this session"}"#,
        &json!({"ok": true}),
    )
    .expect("recall");
    let text = recall
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(text.contains("default-session-marker-alpha"));
    assert!(!text.contains("other-session-marker-beta"));
}
