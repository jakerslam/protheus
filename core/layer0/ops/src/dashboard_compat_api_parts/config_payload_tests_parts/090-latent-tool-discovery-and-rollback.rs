#[test]
fn workspace_hints_and_latent_tool_candidates_surface_security_paths() {
    let root = tempfile::tempdir().expect("tempdir");
    let src_dir = root.path().join("src").join("api");
    let _ = fs::create_dir_all(&src_dir);
    let _ = fs::write(src_dir.join("security_gate.rs"), "pub fn gate() {}");
    let _ = fs::write(src_dir.join("billing_router.rs"), "pub fn route() {}");

    let profile = json!({
        "workspace_dir": root.path().to_string_lossy().to_string()
    });
    let hints = workspace_file_hints_for_message(
        root.path(),
        Some(&profile),
        "I'm worried about security in this API module",
        5,
    );
    assert!(!hints.is_empty());
    let joined_paths = hints
        .iter()
        .filter_map(|row| row.get("path").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    assert!(joined_paths.contains("security_gate.rs"));

    let candidates =
        latent_tool_candidates_for_message("Please audit the security of this API code", &hints);
    let tools = candidates
        .iter()
        .filter_map(|row| row.get("tool").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert!(tools.contains(&"terminal_exec"));
    assert!(tools.contains(&"file_read"));
}

#[test]
fn direct_intent_parser_supports_undo_routes() {
    let slash = direct_tool_intent_from_user_message("/undo").expect("slash undo");
    assert_eq!(slash.0, "session_rollback_last_turn");

    let natural = direct_tool_intent_from_user_message("undo that").expect("natural undo");
    assert_eq!(natural.0, "session_rollback_last_turn");
}

#[test]
fn rollback_tool_removes_recent_turn_and_writes_archive() {
    let root = tempfile::tempdir().expect("tempdir");
    let session_path = state_path(root.path(), AGENT_SESSIONS_DIR_REL).join("agent-rollback.json");
    write_json(
        &session_path,
        &json!({
            "agent_id": "agent-rollback",
            "active_session_id": "default",
            "sessions": [
                {
                    "session_id": "default",
                    "updated_at": crate::now_iso(),
                    "messages": [
                        {"role":"user","text":"first turn"},
                        {"role":"assistant","text":"first reply"}
                    ]
                }
            ]
        }),
    );

    let result = execute_tool_call_by_name(
        root.path(),
        &json!({"ok": true}),
        "agent-rollback",
        None,
        "session_rollback_last_turn",
        &json!({}),
    );
    assert_eq!(result.get("ok").and_then(Value::as_bool), Some(true));
    assert!(
        result
            .get("removed_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 1
    );

    let state = load_session_state(root.path(), "agent-rollback");
    assert_eq!(all_session_messages(&state).len(), 0);
    assert!(state
        .pointer("/sessions/0/rollback_archives")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
}

#[test]
fn memory_prompt_context_partitions_semantic_and_prunes_stale_episodic() {
    let stale = (Utc::now() - chrono::Duration::days(21)).to_rfc3339();
    let recent = (Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
    let state = json!({
        "memory_kv": {
            "fact.user_name": "Jay",
            "ephemeral.old_note": {"text":"old transient memory", "captured_at": stale},
            "ephemeral.recent_note": {"text":"new transient memory", "captured_at": recent}
        }
    });
    let prompt = memory_kv_prompt_context(&state, 24);
    assert!(prompt.contains("Pinned semantic memory"));
    assert!(prompt.contains("fact.user_name"));
    assert!(prompt.contains("Recent episodic memory"));
    assert!(prompt.contains("ephemeral.recent_note"));
    assert!(!prompt.contains("ephemeral.old_note"));
}
