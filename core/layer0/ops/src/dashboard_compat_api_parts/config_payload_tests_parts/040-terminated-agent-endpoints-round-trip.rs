#[test]
fn terminated_agent_endpoints_round_trip() {
    let root = tempfile::tempdir().expect("tempdir");
    let _ = crate::dashboard_agent_state::upsert_contract(
        root.path(),
        "agent-a",
        &json!({
            "created_at": "2000-01-01T00:00:00Z",
            "expiry_seconds": 1,
            "status": "active"
        }),
    );
    let _ = crate::dashboard_agent_state::enforce_expired_contracts(root.path());

    let listed = handle(
        root.path(),
        "GET",
        "/api/agents/terminated",
        &[],
        &json!({"ok": true}),
    )
    .expect("terminated list");
    let rows = listed
        .payload
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!rows.is_empty());

    let revived = handle(
        root.path(),
        "POST",
        "/api/agents/agent-a/revive",
        br#"{"role":"analyst"}"#,
        &json!({"ok": true}),
    )
    .expect("revive");
    assert_eq!(
        revived.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );

    let after_revive = handle(
        root.path(),
        "GET",
        "/api/agents/terminated",
        &[],
        &json!({"ok": true}),
    )
    .expect("terminated list after revive");
    let rows_after = after_revive
        .payload
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(rows_after.is_empty());

    let _ = crate::dashboard_agent_state::upsert_contract(
        root.path(),
        "agent-a",
        &json!({
            "created_at": "2000-01-01T00:00:00Z",
            "expiry_seconds": 1,
            "status": "active"
        }),
    );
    let _ = crate::dashboard_agent_state::enforce_expired_contracts(root.path());
    let deleted = handle(
        root.path(),
        "DELETE",
        "/api/agents/terminated/agent-a",
        &[],
        &json!({"ok": true}),
    )
    .expect("delete terminated");
    assert!(
        deleted
            .payload
            .get("removed_history_entries")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 1
    );
}

#[test]
fn archive_all_agents_endpoint_archives_visible_roster() {
    let root = tempfile::tempdir().expect("tempdir");
    let created_a = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Alpha","role":"operator"}"#,
        &json!({"ok": true}),
    )
    .expect("create alpha");
    let created_b = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Beta","role":"analyst"}"#,
        &json!({"ok": true}),
    )
    .expect("create beta");
    let alpha_id = clean_text(
        created_a
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let beta_id = clean_text(
        created_b
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!alpha_id.is_empty());
    assert!(!beta_id.is_empty());

    let archived = handle(
        root.path(),
        "POST",
        "/api/agents/archive-all",
        br#"{"reason":"test_archive_all"}"#,
        &json!({"ok": true}),
    )
    .expect("archive all");
    assert_eq!(archived.status, 200);
    assert_eq!(
        archived.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    let archived_ids = archived
        .payload
        .get("archived_agent_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let archived_labels: Vec<String> = archived_ids
        .iter()
        .filter_map(|row| row.as_str())
        .map(|row| row.to_string())
        .collect();
    assert!(archived_labels.contains(&alpha_id));
    assert!(archived_labels.contains(&beta_id));

    let listed =
        handle(root.path(), "GET", "/api/agents", &[], &json!({"ok": true})).expect("list active");
    let rows = listed.payload.as_array().cloned().unwrap_or_default();
    assert!(rows
        .iter()
        .all(
            |row| clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 180) != alpha_id
        ));
    assert!(rows
        .iter()
        .all(
            |row| clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 180) != beta_id
        ));
}

#[test]
fn archive_all_agents_endpoint_rejects_actor_scoped_requests() {
    let root = tempfile::tempdir().expect("tempdir");
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Parent","role":"operator"}"#,
        &json!({"ok": true}),
    )
    .expect("create parent");
    let actor_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!actor_id.is_empty());

    let denied = handle_with_headers(
        root.path(),
        "POST",
        "/api/agents/archive-all",
        br#"{}"#,
        &[("X-Actor-Agent-Id", actor_id.as_str())],
        &json!({"ok": true}),
    )
    .expect("archive-all denied");
    assert_eq!(denied.status, 403);
    assert_eq!(
        denied.payload.get("error").and_then(Value::as_str),
        Some("agent_manage_forbidden")
    );
}

#[test]
fn roster_excludes_zombies_and_archived_profiles_surface_in_terminated() {
    let root = tempfile::tempdir().expect("tempdir");
    let _ = crate::dashboard_agent_state::upsert_profile(
        root.path(),
        "agent-live",
        &json!({
            "name": "Live",
            "role": "operator",
            "state": "Running"
        }),
    );
    let _ = crate::dashboard_agent_state::upsert_contract(
        root.path(),
        "agent-live",
        &json!({
            "status": "active",
            "created_at": "2099-01-01T00:00:00Z",
            "updated_at": "2099-01-01T00:00:00Z",
            "expires_at": "2099-12-31T00:00:00Z",
            "auto_terminate_allowed": false
        }),
    );
    let _ = crate::dashboard_agent_state::upsert_profile(
        root.path(),
        "agent-archived",
        &json!({
            "name": "Archived",
            "role": "analyst",
            "state": "Archived",
            "updated_at": "2026-01-02T00:00:00Z"
        }),
    );
    let _ = crate::dashboard_agent_state::upsert_contract(
        root.path(),
        "agent-archived",
        &json!({
            "status": "active",
            "created_at": "2099-01-01T00:00:00Z",
            "updated_at": "2099-01-01T00:00:00Z",
            "expires_at": "2099-12-31T00:00:00Z",
            "auto_terminate_allowed": false
        }),
    );
    let _ = crate::dashboard_agent_state::append_turn(
        root.path(),
        "agent-zombie",
        "zombie input",
        "zombie output",
    );
    let snapshot = json!({
        "ok": true,
        "collab": {
            "dashboard": {
                "agents": [
                    {
                        "shadow": "agent-zombie",
                        "role": "analyst",
                        "status": "running",
                        "activated_at": "2026-01-01T00:00:00Z"
                    }
                ]
            }
        },
        "agents": {
            "session_summaries": {
                "rows": [
                    {
                        "agent_id": "agent-zombie",
                        "message_count": 99,
                        "updated_at": "2026-01-03T00:00:00Z"
                    }
                ]
            }
        }
    });

    let listed = handle(root.path(), "GET", "/api/agents", &[], &snapshot).expect("agents");
    let rows = listed.payload.as_array().cloned().unwrap_or_default();
    let ids = rows
        .iter()
        .filter_map(|row| row.get("id").and_then(Value::as_str))
        .map(|value| clean_text(value, 180))
        .collect::<Vec<_>>();
    assert!(ids.iter().any(|id| id == "agent-live"));
    assert!(!ids.iter().any(|id| id == "agent-zombie"));
    assert!(!ids.iter().any(|id| id == "agent-archived"));

    let status = handle(root.path(), "GET", "/api/status", &[], &snapshot).expect("status");
    assert_eq!(
        status.payload.get("agent_count").and_then(Value::as_u64),
        Some(1)
    );

    let terminated =
        handle(root.path(), "GET", "/api/agents/terminated", &[], &snapshot).expect("terminated");
    let terminated_rows = terminated
        .payload
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(terminated_rows.iter().any(|row| {
        clean_text(
            row.get("agent_id").and_then(Value::as_str).unwrap_or(""),
            180,
        ) == "agent-archived"
    }));
}

#[test]
fn terminal_endpoints_round_trip() {
    let root = tempfile::tempdir().expect("tempdir");
    let created = handle(
        root.path(),
        "POST",
        "/api/terminal/sessions",
        br#"{"id":"term-a"}"#,
        &json!({"ok": true}),
    )
    .expect("create");
    assert_eq!(
        created.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    let listed = handle(
        root.path(),
        "GET",
        "/api/terminal/sessions",
        &[],
        &json!({"ok": true}),
    )
    .expect("list");
    assert_eq!(
        listed
            .payload
            .get("sessions")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(1)
    );
    let ran = handle(
        root.path(),
        "POST",
        "/api/terminal/queue",
        br#"{"session_id":"term-a","command":"printf 'ok'"}"#,
        &json!({"ok": true}),
    )
    .expect("exec");
    assert_eq!(
        ran.payload.get("stdout").and_then(Value::as_str),
        Some("ok")
    );
    assert_eq!(
        ran.payload.get("executed_command").and_then(Value::as_str),
        Some("printf 'ok'")
    );
    assert_eq!(
        ran.payload
            .get("command_translated")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        ran.payload
            .pointer("/permission_gate/verdict")
            .and_then(Value::as_str),
        Some("allow")
    );
    assert_eq!(
        ran.payload
            .pointer("/tool_summary/status")
            .and_then(Value::as_str),
        Some("ok")
    );
}

#[test]
fn agent_terminal_routes_through_command_router() {
    let root = tempfile::tempdir().expect("tempdir");
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Ops","role":"operator"}"#,
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

    let terminal = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/terminal"),
        br#"{"command":"printf 'ok'"}"#,
        &json!({"ok": true}),
    )
    .expect("terminal");
    assert_eq!(terminal.status, 200);
    assert_eq!(
        terminal.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        terminal.payload.get("stdout").and_then(Value::as_str),
        Some("ok")
    );
    assert_eq!(
        terminal
            .payload
            .get("executed_command")
            .and_then(Value::as_str),
        Some("printf 'ok'")
    );
    assert_eq!(
        terminal
            .payload
            .get("command_translated")
            .and_then(Value::as_bool),
        Some(false)
    );

    let translated = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/terminal"),
        br#"{"command":"infring daemon ping"}"#,
        &json!({"ok": true}),
    )
    .expect("translated");
    assert_eq!(translated.status, 200);
    assert_eq!(
        translated.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        translated
            .payload
            .get("executed_command")
            .and_then(Value::as_str),
        Some("protheus-ops daemon ping")
    );
    assert_eq!(
        translated
            .payload
            .get("command_translated")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        translated
            .payload
            .get("translation_reason")
            .and_then(Value::as_str),
        Some("translated_infring_cli_alias_to_protheus_ops")
    );
}

#[test]
fn agent_terminal_blocks_policy_denied_command_with_structured_summary() {
    let root = tempfile::tempdir().expect("tempdir");
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Ops","role":"operator"}"#,
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

    let blocked = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/terminal"),
        br#"{"command":"git reset --hard HEAD"}"#,
        &json!({"ok": true}),
    )
    .expect("blocked");
    assert_eq!(blocked.status, 200);
    assert_eq!(
        blocked.payload.get("ok").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        blocked.payload.get("blocked").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        blocked
            .payload
            .pointer("/permission_gate/verdict")
            .and_then(Value::as_str),
        Some("deny")
    );
    assert_eq!(
        blocked
            .payload
            .pointer("/tool_summary/status")
            .and_then(Value::as_str),
        Some("blocked")
    );
    assert!(blocked
        .payload
        .get("recovery_hints")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
}

#[test]
fn agent_command_endpoint_routes_runtime_queries_in_core() {
    let root = tempfile::tempdir().expect("tempdir");
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Ops","role":"operator"}"#,
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
        context.payload.get("command").and_then(Value::as_str),
        Some("context")
    );
    assert!(context
        .payload
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("Context window:"));

    let queue = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/command"),
        br#"{"command":"queue"}"#,
        &json!({
            "ok": true,
            "runtime_sync": {
                "summary": {
                    "queue_depth": 4,
                    "conduit_signals": 2,
                    "backpressure_level": "normal"
                }
            }
        }),
    )
    .expect("queue command");
    assert_eq!(queue.status, 200);
    assert_eq!(queue.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        queue.payload.get("command").and_then(Value::as_str),
        Some("queue")
    );
    assert!(queue
        .payload
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("Queue depth: 4"));
}

#[test]
fn session_backed_agents_drive_roster_sessions_and_usage() {
    let root = tempfile::tempdir().expect("tempdir");
    let _ = crate::dashboard_agent_state::append_turn(
        root.path(),
        "chat-ui-default-agent",
        "hello there",
        "hi back",
    );

    let listed =
        handle(root.path(), "GET", "/api/agents", &[], &json!({"ok": true})).expect("list agents");
    let rows = listed.payload.as_array().cloned().unwrap_or_default();
    assert!(rows.iter().any(|row| {
        row.get("id")
            .and_then(Value::as_str)
            .map(|value| value == "chat-ui-default-agent")
            .unwrap_or(false)
    }));

    let session = handle(
        root.path(),
        "GET",
        "/api/agents/chat-ui-default-agent/session",
        &[],
        &json!({"ok": true}),
    )
    .expect("session");
    assert_eq!(
        session.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        session
            .payload
            .get("messages")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(2)
    );

    let summaries = handle(
        root.path(),
        "GET",
        "/api/sessions",
        &[],
        &json!({"ok": true}),
    )
    .expect("session summaries");
    assert!(summaries
        .payload
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("agent_id")
                    .and_then(Value::as_str)
                    .map(|value| value == "chat-ui-default-agent")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false));

    let usage = handle(root.path(), "GET", "/api/usage", &[], &json!({"ok": true})).expect("usage");
    assert!(usage
        .payload
        .get("agents")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("agent_id")
                    .and_then(Value::as_str)
                    .map(|value| value == "chat-ui-default-agent")
                    .unwrap_or(false)
                    && row.get("total_tokens").and_then(Value::as_i64).unwrap_or(0) > 0
            })
        })
        .unwrap_or(false));

    let summary = handle(
        root.path(),
        "GET",
        "/api/usage/summary",
        &[],
        &json!({"ok": true}),
    )
    .expect("usage summary");
    assert_eq!(
        summary.payload.get("call_count").and_then(Value::as_i64),
        Some(1)
    );
}

#[test]
fn active_collab_agent_is_not_hidden_by_stale_terminated_contract() {
    let root = tempfile::tempdir().expect("tempdir");
    let _ = crate::dashboard_agent_state::upsert_profile(
        root.path(),
        "agent-live",
        &json!({"name":"Jarvis","role":"analyst","updated_at":"2026-03-28T00:00:00Z"}),
    );
    let _ = crate::dashboard_agent_state::upsert_contract(
        root.path(),
        "agent-live",
        &json!({
            "status": "terminated",
            "created_at": "2026-03-28T00:00:00Z",
            "updated_at": "2026-03-28T00:00:00Z"
        }),
    );
    let snapshot = json!({
        "ok": true,
        "collab": {
            "dashboard": {
                "agents": [
                    {
                        "shadow": "agent-live",
                        "status": "active",
                        "role": "analyst",
                        "activated_at": "2026-03-29T00:00:00Z"
                    }
                ]
            }
        }
    });

    let listed = handle(root.path(), "GET", "/api/agents", &[], &snapshot).expect("agents");
    assert!(listed
        .payload
        .as_array()
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("id")
                    .and_then(Value::as_str)
                    .map(|value| value == "agent-live")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false));
}

#[test]
fn actor_agent_management_is_scoped_to_descendants() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());

    let parent = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Parent","role":"director"}"#,
        &json!({"ok": true}),
    )
    .expect("create parent");
    let parent_id = clean_text(
        parent
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!parent_id.is_empty());

    let child = handle(
        root.path(),
        "POST",
        "/api/agents",
        format!(
            "{{\"name\":\"Child\",\"role\":\"analyst\",\"parent_agent_id\":\"{}\"}}",
            parent_id
        )
        .as_bytes(),
        &json!({"ok": true}),
    )
    .expect("create child");
    let child_id = clean_text(
        child
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!child_id.is_empty());

    let sibling = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Sibling","role":"analyst"}"#,
        &json!({"ok": true}),
    )
    .expect("create sibling");
    let sibling_id = clean_text(
        sibling
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!sibling_id.is_empty());

    let allowed = handle_with_headers(
        root.path(),
        "POST",
        &format!("/api/agents/{child_id}/stop"),
        br#"{}"#,
        &[("X-Actor-Agent-Id", parent_id.as_str())],
        &json!({"ok": true}),
    )
    .expect("parent manages child");
    assert_eq!(allowed.status, 200);
    assert_eq!(
        allowed.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );

    let denied = handle_with_headers(
        root.path(),
        "POST",
        &format!("/api/agents/{child_id}/start"),
        br#"{}"#,
        &[("X-Actor-Agent-Id", sibling_id.as_str())],
        &json!({"ok": true}),
    )
    .expect("sibling denied");
    assert_eq!(denied.status, 403);
    assert_eq!(
        denied.payload.get("error").and_then(Value::as_str),
        Some("agent_manage_forbidden")
    );
}

#[test]
fn direct_slash_tool_routes_through_agent_message() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let _ = fs::create_dir_all(root.path().join("notes"));
    let _ = fs::write(root.path().join("notes/plan.txt"), "ship it");

    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Tool Agent","role":"operator"}"#,
        &json!({"ok": true}),
    )
    .expect("create");
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());

    let out = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"/file notes/plan.txt"}"#,
        &json!({"ok": true}),
    )
    .expect("slash tool");
    assert_eq!(out.status, 200);
    assert_eq!(out.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert!(out
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("ship it"));
    assert!(out
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("name")
                    .and_then(Value::as_str)
                    .map(|name| name == "file_read")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false));
}

#[test]
fn direct_search_slash_routes_through_web_search_tool() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());

    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Search Agent","role":"researcher"}"#,
        &json!({"ok": true}),
    )
    .expect("create");
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());

    let out = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"/search infringing runtime architecture"}"#,
        &json!({"ok": true}),
    )
    .expect("search slash tool");
    assert!(matches!(out.status, 200 | 400));
    assert!(out
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("name")
                    .and_then(Value::as_str)
                    .map(|name| name == "web_search")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false));
}

#[test]
fn natural_language_web_search_intent_routes_without_slash() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());

    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Search Agent 2","role":"researcher"}"#,
        &json!({"ok": true}),
    )
    .expect("create");
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());

    let out = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"search the web for robust websocket reconnect patterns"}"#,
        &json!({"ok": true}),
    )
    .expect("natural search tool");
    assert!(matches!(out.status, 200 | 400));
    assert!(out
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("name")
                    .and_then(Value::as_str)
                    .map(|name| name == "web_search")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false));
}

#[test]
fn message_turns_capture_memory_and_attention_receipt() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());

    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Memory Agent","role":"analyst"}"#,
        &json!({"ok": true}),
    )
    .expect("create");
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());

    let out = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Remember this exactly: launch codename is aurora-7"}"#,
        &json!({"ok": true}),
    )
    .expect("message");
    assert_eq!(out.status, 200);
    let memory_capture_ok = out
        .payload
        .pointer("/memory_capture/ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    assert!(memory_capture_ok);
    let queued = out
        .payload
        .pointer("/attention_queue/queued")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let staged = out
        .payload
        .pointer("/attention_queue/staged")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    assert!(queued || staged);
}
