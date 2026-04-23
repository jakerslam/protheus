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

#[test]
fn agent_terminal_routes_through_command_router() {
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

    let terminal = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/terminal"),
        br#"{"command":"printf 'ok'"}"#,
        &terminated_ok_snapshot(),
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
        &terminated_ok_snapshot(),
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
        Some("infring-ops daemon ping")
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
        Some("translated_infring_cli_alias_to_infring_ops")
    );
}

#[test]
fn agent_terminal_blocks_policy_denied_command_with_structured_summary() {
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

    let blocked = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/terminal"),
        br#"{"command":"git reset --hard HEAD"}"#,
        &terminated_ok_snapshot(),
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

