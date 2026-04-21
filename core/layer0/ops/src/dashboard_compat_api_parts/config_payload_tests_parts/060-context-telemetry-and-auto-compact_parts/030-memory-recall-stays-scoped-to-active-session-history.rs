
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

#[test]
fn context_command_reports_recent_floor_reinjection_when_pool_trim_is_aggressive() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());
    let session_path =
        state_path(root.path(), AGENT_SESSIONS_DIR_REL).join(format!("{agent_id}.json"));
    let dense_messages = (0..240)
        .map(|idx| {
            json!({
                "id": idx + 1,
                "role": if idx % 2 == 0 { "user" } else { "agent" },
                "text": format!("floor-reinject-{idx} {}", "token ".repeat(1200)),
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
        br#"{"command":"context","silent":true,"context_pool_limit_tokens":32000,"active_context_min_recent_messages":64}"#,
        &json!({"ok": true}),
    )
    .expect("context command");
    assert_eq!(context.status, 200);
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_enforced")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        context
            .payload
            .pointer("/context_pool/recent_floor_injected")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_target")
            .and_then(Value::as_u64),
        Some(64)
    );
    assert!(
        context
            .payload
            .pointer("/context_pool/recent_floor_missing_before")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
    );
    assert!(
        context
            .payload
            .pointer("/context_pool/recent_floor_coverage_before")
            .and_then(Value::as_f64)
            .unwrap_or(1.0)
            < 1.0
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_satisfied")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_coverage_after")
            .and_then(Value::as_f64),
        Some(1.0)
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_active_missing")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_active_satisfied")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_active_coverage")
            .and_then(Value::as_f64),
        Some(1.0)
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_continuity_status")
            .and_then(Value::as_str),
        Some("ready")
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_continuity_action")
            .and_then(Value::as_str),
        Some("none")
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_continuity_reason")
            .and_then(Value::as_str),
        Some("none")
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_continuity_retryable")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn context_command_reports_continuity_degraded_when_emergency_window_undercuts_recent_floor() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = create_context_test_agent(root.path());
    let session_path =
        state_path(root.path(), AGENT_SESSIONS_DIR_REL).join(format!("{agent_id}.json"));
    let dense_messages = (0..240)
        .map(|idx| {
            json!({
                "id": idx + 1,
                "role": if idx % 2 == 0 { "user" } else { "agent" },
                "text": format!("floor-degraded-{idx} {}", "token ".repeat(1200)),
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
        br#"{"command":"context","silent":true,"context_pool_limit_tokens":32000,"active_context_min_recent_messages":64,"active_context_target_tokens":120000,"auto_compact_target_ratio":0.4,"emergency_min_recent_messages":8}"#,
        &json!({"ok": true}),
    )
    .expect("context command");
    assert_eq!(context.status, 200);
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_active_satisfied")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_continuity_status")
            .and_then(Value::as_str),
        Some("degraded")
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_continuity_action")
            .and_then(Value::as_str),
        Some("raise_active_context_floor_or_target")
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_continuity_reason")
            .and_then(Value::as_str),
        Some("active_recent_floor_missing")
    );
    assert_eq!(
        context
            .payload
            .pointer("/context_pool/recent_floor_continuity_retryable")
            .and_then(Value::as_bool),
        Some(true)
    );
}
