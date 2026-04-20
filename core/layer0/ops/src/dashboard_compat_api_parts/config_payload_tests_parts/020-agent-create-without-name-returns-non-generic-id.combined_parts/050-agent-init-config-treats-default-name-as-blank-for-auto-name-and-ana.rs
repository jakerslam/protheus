

#[test]
fn agent_init_config_treats_default_name_as_blank_for_auto_name_and_analyst_intro() {
    let root = agent_create_temp_root();
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"role":"analyst"}"#,
        &agent_create_ok_snapshot(),
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

    let configured = handle(
        root.path(),
        "PATCH",
        &format!("/api/agents/{agent_id}/config"),
        format!(
            "{{\"name\":\"{}\",\"system_prompt\":\"You are an analyst.\",\"archetype\":\"analyst\",\"profile\":\"analysis\",\"contract\":{{\"mission\":\"Analyze outcomes\",\"termination_condition\":\"task_or_timeout\",\"expiry_seconds\":3600}}}}",
            agent_id
        )
        .as_bytes(),
        &agent_create_ok_snapshot(),
    )
    .expect("config");
    assert_eq!(configured.status, 200);
    assert_eq!(
        configured
            .payload
            .pointer("/rename_notice/auto_generated")
            .and_then(Value::as_bool),
        Some(true),
        "default-like name should be treated as blank so post-init auto-name runs"
    );

    let agent_row = handle(
        root.path(),
        "GET",
        &format!("/api/agents/{agent_id}"),
        &[],
        &agent_create_ok_snapshot(),
    )
    .expect("agent row");
    let resolved_name = clean_text(
        agent_row.payload.get("name").and_then(Value::as_str).unwrap_or(""),
        120,
    );
    assert!(!resolved_name.is_empty());
    assert_ne!(
        resolved_name.to_ascii_lowercase(),
        agent_id.to_ascii_lowercase(),
        "post-init auto-name should replace the default id label"
    );

    let session = handle(
        root.path(),
        "GET",
        &format!("/api/agents/{agent_id}/session"),
        &[],
        &agent_create_ok_snapshot(),
    )
    .expect("session");
    let messages = session
        .payload
        .pointer("/session/sessions/0/messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!messages.is_empty(), "expected intro message after init");
    let intro_text = clean_text(
        messages[0].get("text").and_then(Value::as_str).unwrap_or(""),
        280,
    )
    .to_ascii_lowercase();
    assert!(
        intro_text.contains("what should we analyze first"),
        "analyst intro should ask about analysis, not research: {intro_text}"
    );
}

#[test]
fn agent_message_runtime_probe_uses_authoritative_runtime_summary() {
    let root = agent_create_temp_root();
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Runtime Probe","role":"analyst"}"#,
        &agent_create_ok_snapshot(),
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

    let message = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"Report runtime sync now. What changed in queue depth, cockpit blocks, conduit signals, and memory context?"}"#,
            &agent_create_ok_snapshot(),
        )
        .expect("agent runtime probe");
    assert_eq!(message.status, 200);
    assert_eq!(
        message.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    let response = message
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(response.contains("Current queue depth:"));
    assert!(!response.contains("Persistent memory is enabled for this agent across"));
    assert!(message
        .payload
        .get("runtime_sync")
        .and_then(Value::as_object)
        .is_some());
}
