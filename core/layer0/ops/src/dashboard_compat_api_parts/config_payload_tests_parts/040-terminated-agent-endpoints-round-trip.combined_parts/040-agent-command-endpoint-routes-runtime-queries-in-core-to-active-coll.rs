
#[test]
fn agent_command_endpoint_routes_runtime_queries_in_core() {
    let root = terminated_temp_root();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Ops","role":"operator"}"#,
        &terminated_ok_snapshot(),
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
        &terminated_ok_snapshot(),
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
    let root = terminated_temp_root();
    let _ = crate::dashboard_agent_state::append_turn(
        root.path(),
        "chat-ui-default-agent",
        "hello there",
        "hi back",
    );

    let listed =
        handle(root.path(), "GET", "/api/agents", &[], &terminated_ok_snapshot()).expect("list agents");
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
        &terminated_ok_snapshot(),
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
        &terminated_ok_snapshot(),
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

    let usage = handle(root.path(), "GET", "/api/usage", &[], &terminated_ok_snapshot()).expect("usage");
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
        &terminated_ok_snapshot(),
    )
    .expect("usage summary");
    assert_eq!(
        summary.payload.get("call_count").and_then(Value::as_i64),
        Some(1)
    );
}


#[test]
fn active_collab_agent_is_not_hidden_by_stale_terminated_contract() {
    let root = terminated_temp_root();
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
