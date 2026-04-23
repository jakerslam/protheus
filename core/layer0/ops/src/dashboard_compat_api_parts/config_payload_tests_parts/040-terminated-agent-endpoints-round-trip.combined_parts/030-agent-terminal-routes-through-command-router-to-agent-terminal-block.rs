
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
