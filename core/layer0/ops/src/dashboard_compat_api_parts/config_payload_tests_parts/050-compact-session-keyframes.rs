#[test]
fn session_compaction_emits_keyframes_and_archive() {
    let root = tempfile::tempdir().expect("tempdir");
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Keyframe","role":"analyst"}"#,
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
    let session_path = state_path(root.path(), AGENT_SESSIONS_DIR_REL).join(format!("{agent_id}.json"));
    let messages = (0..36)
        .map(|idx| {
            json!({
                "id": idx + 1,
                "role": if idx % 2 == 0 { "user" } else { "agent" },
                "text": format!("message-{idx} context payload for compaction keyframe coverage"),
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
                    "messages": messages
                }
            ]
        }),
    );

    let compact = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/session/compact"),
        br#"{"target_context_window":512,"target_ratio":0.4,"min_recent_messages":6}"#,
        &json!({"ok": true}),
    )
    .expect("compact");
    assert_eq!(compact.status, 200);
    assert_eq!(
        compact
            .payload
            .get("keyframes_emitted")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0,
        true
    );
    assert_eq!(
        compact
            .payload
            .get("removed_messages")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0,
        true
    );

    let session = handle(
        root.path(),
        "GET",
        &format!("/api/agents/{agent_id}/session"),
        &[],
        &json!({"ok": true}),
    )
    .expect("session");
    assert_eq!(session.status, 200);
    assert_eq!(
        session
            .payload
            .pointer("/session/sessions/0/context_keyframes")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false),
        true
    );
    assert_eq!(
        session
            .payload
            .pointer("/session/sessions/0/compaction_archives")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false),
        true
    );
}

#[test]
fn session_compaction_preserves_history_by_default() {
    let root = tempfile::tempdir().expect("tempdir");
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Preserve","role":"analyst"}"#,
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
    let session_path = state_path(root.path(), AGENT_SESSIONS_DIR_REL).join(format!("{agent_id}.json"));
    let messages = (0..64)
        .map(|idx| {
            json!({
                "id": idx + 1,
                "role": if idx % 2 == 0 { "user" } else { "agent" },
                "text": format!("preserve-{idx} {}", "context ".repeat(40)),
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
                    "messages": messages
                }
            ]
        }),
    );
    let before = load_session_state(root.path(), &agent_id);
    let before_count = all_session_messages(&before).len();

    let compact = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/session/compact"),
        br#"{"target_context_window":1024,"target_ratio":0.45,"min_recent_messages":8}"#,
        &json!({"ok": true}),
    )
    .expect("compact");
    assert_eq!(compact.status, 200);
    assert_eq!(
        compact
            .payload
            .get("persisted_to_session")
            .and_then(Value::as_bool),
        Some(false)
    );

    let after = load_session_state(root.path(), &agent_id);
    let after_count = all_session_messages(&after).len();
    assert_eq!(after_count, before_count);
}

#[test]
fn session_compaction_can_persist_when_explicitly_enabled() {
    let root = tempfile::tempdir().expect("tempdir");
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Persist","role":"analyst"}"#,
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
    let session_path = state_path(root.path(), AGENT_SESSIONS_DIR_REL).join(format!("{agent_id}.json"));
    let messages = (0..80)
        .map(|idx| {
            json!({
                "id": idx + 1,
                "role": if idx % 2 == 0 { "user" } else { "agent" },
                "text": format!("persist-{idx} {}", "token ".repeat(50)),
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
                    "messages": messages
                }
            ]
        }),
    );
    let before = load_session_state(root.path(), &agent_id);
    let before_count = all_session_messages(&before).len();

    let compact = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/session/compact"),
        br#"{"target_context_window":1024,"target_ratio":0.40,"min_recent_messages":6,"persist_compaction_to_session":true}"#,
        &json!({"ok": true}),
    )
    .expect("compact");
    assert_eq!(compact.status, 200);
    assert_eq!(
        compact
            .payload
            .get("persisted_to_session")
            .and_then(Value::as_bool),
        Some(true)
    );

    let after = load_session_state(root.path(), &agent_id);
    let after_count = all_session_messages(&after).len();
    assert_eq!(after_count < before_count, true);
}

#[test]
fn comms_events_endpoint_supports_kind_filter_and_contract_metadata() {
    let root = tempfile::tempdir().expect("tempdir");
    let snapshot = json!({"ok": true});
    let created = handle(
        root.path(),
        "POST",
        "/api/comms/task",
        br#"{"title":"Emit event","description":"for event filtering"}"#,
        &snapshot,
    )
    .expect("create task");
    assert_eq!(created.status, 200);
    let task_id = created
        .payload
        .pointer("/task/id")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(!task_id.is_empty());

    let events = handle(
        root.path(),
        "GET",
        &format!("/api/comms/events?kind=task_posted&task_id={task_id}"),
        &[],
        &snapshot,
    )
    .expect("events");
    assert_eq!(events.status, 200);
    assert_eq!(
        events
            .payload
            .get("contract_version")
            .and_then(Value::as_str),
        Some("dashboard_comms_v1")
    );
    let rows = events
        .payload
        .get("events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|row| {
        row.get("kind")
            .and_then(Value::as_str)
            .map(|kind| kind.eq_ignore_ascii_case("task_posted"))
            .unwrap_or(false)
    }));
    assert_eq!(
        events
            .payload
            .get("total_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= rows.len() as u64,
        true
    );
}

#[test]
fn templates_listing_includes_new_starter_roles() {
    let root = tempfile::tempdir().expect("tempdir");
    let payload = handle(root.path(), "GET", "/api/templates", &[], &json!({"ok": true}))
        .expect("templates list");
    assert_eq!(payload.status, 200);
    let rows = payload
        .payload
        .get("templates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(rows
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some("travel-assistant")));
    assert!(rows
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some("real-estate-agent")));
}

#[test]
fn template_detail_returns_manifest_toml_for_spawn_flow() {
    let root = tempfile::tempdir().expect("tempdir");
    let payload = handle(
        root.path(),
        "GET",
        "/api/templates/travel-assistant",
        &[],
        &json!({"ok": true}),
    )
    .expect("template detail");
    assert_eq!(payload.status, 200);
    let manifest = payload
        .payload
        .get("manifest_toml")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(manifest.contains("role = \"travel_assistant\""));
    assert!(manifest.contains("[model]"));
}
