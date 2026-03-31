#[test]
fn session_compaction_emits_keyframes_and_archive() {
    let root = tempfile::tempdir().expect("tempdir");
    let agent_id = "agent-keyframe";
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
        "/api/agents/agent-keyframe/session/compact",
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
        "/api/agents/agent-keyframe/session",
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
