
#[test]
fn agent_init_config_seeds_role_tailored_intro_message() {
    let root = agent_create_temp_root();
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"role":"engineer"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create agent");
    assert_eq!(created.status, 200);
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
        br#"{
            "name":"",
            "system_prompt":"You are a coding specialist.",
            "archetype":"coding",
            "profile":"builder",
            "contract":{"mission":"Build features","termination_condition":"task_or_timeout","expiry_seconds":3600}
        }"#,
        &agent_create_ok_snapshot(),
    )
    .expect("config");
    assert_eq!(configured.status, 200);
    assert_eq!(
        configured.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        configured
            .payload
            .pointer("/rename_notice/auto_generated")
            .and_then(Value::as_bool),
        Some(true)
    );
    let resolved_name = clean_text(
        configured
            .payload
            .pointer("/agent/name")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    assert!(!resolved_name.is_empty(), "post-init should resolve a concrete name");
    assert_ne!(
        resolved_name.to_ascii_lowercase(),
        agent_id.to_ascii_lowercase(),
        "post-init should replace default agent-id display name when user leaves name blank"
    );

    let session = handle(
        root.path(),
        "GET",
        &format!("/api/agents/{agent_id}/session"),
        &[],
        &agent_create_ok_snapshot(),
    )
    .expect("session");
    assert_eq!(session.status, 200);
    let messages = session
        .payload
        .pointer("/session/sessions/0/messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!messages.is_empty(), "expected intro message after init");
    let first = messages[0].clone();
    assert_eq!(first.get("role").and_then(Value::as_str), Some("assistant"));
    let intro_text = clean_text(first.get("text").and_then(Value::as_str).unwrap_or(""), 280)
        .to_ascii_lowercase();
    assert!(
        intro_text.contains("what are we coding today"),
        "intro should be tailored to coding role: {intro_text}"
    );
}

#[test]
fn agent_init_config_infers_role_from_template_when_role_omitted() {
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
        br#"{
            "system_prompt":"You are a coding specialist.",
            "archetype":"coder",
            "profile":"coding",
            "contract":{"mission":"Build features","termination_condition":"task_or_timeout","expiry_seconds":3600}
        }"#,
        &agent_create_ok_snapshot(),
    )
    .expect("config");
    assert_eq!(configured.status, 200);
    assert_eq!(
        configured.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );

    let agent_row = handle(
        root.path(),
        "GET",
        &format!("/api/agents/{agent_id}"),
        &[],
        &agent_create_ok_snapshot(),
    )
    .expect("agent row");
    assert_eq!(
        agent_row.payload.get("role").and_then(Value::as_str),
        Some("engineer"),
        "init payload should infer coding role when explicit role is omitted"
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
        intro_text.contains("what are we coding today"),
        "intro should follow inferred coding role: {intro_text}"
    );
}
