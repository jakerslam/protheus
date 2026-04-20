
#[test]
fn roster_excludes_zombies_and_archived_profiles_surface_in_terminated() {
    let root = terminated_temp_root();
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
    let root = terminated_temp_root();
    let created = handle(
        root.path(),
        "POST",
        "/api/terminal/sessions",
        br#"{"id":"term-a"}"#,
        &terminated_ok_snapshot(),
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
        &terminated_ok_snapshot(),
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
        &terminated_ok_snapshot(),
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
