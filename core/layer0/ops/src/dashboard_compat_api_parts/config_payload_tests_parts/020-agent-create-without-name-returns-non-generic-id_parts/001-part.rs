fn agents_routes_create_message_config_and_git_tree_round_trip() {
    let root = agent_create_temp_root();
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Jarvis","role":"director","provider":"ollama","model":"qwen:4b"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create agent");
    assert_eq!(created.status, 200);
    assert_eq!(
        created.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());

    let listed =
        handle(root.path(), "GET", "/api/agents", &[], &agent_create_ok_snapshot()).expect("list agents");
    let rows = listed.payload.as_array().cloned().unwrap_or_default();
    assert!(rows.iter().any(|row| {
        clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 180) == agent_id
    }));

    let details = handle(
        root.path(),
        "GET",
        &format!("/api/agents/{agent_id}"),
        &[],
        &agent_create_ok_snapshot(),
    )
    .expect("agent details");
    assert_eq!(details.status, 200);
    assert_eq!(
        details.payload.get("name").and_then(Value::as_str),
        Some("Jarvis")
    );

    let message = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"hello there"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("agent message");
    assert_eq!(message.status, 200);
    assert_eq!(
        message.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    let first_response = message
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        first_response.contains("hello there") || first_response.contains("Current queue depth:"),
        "agent response should return user-facing content or runtime remediation"
    );

    let new_session = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/sessions"),
        br#"{"label":"Ops"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("create session");
    let sid = clean_text(
        new_session
            .payload
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!sid.is_empty());
    let switched = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/sessions/{sid}/switch"),
        &[],
        &agent_create_ok_snapshot(),
    )
    .expect("switch session");
    assert_eq!(
        switched.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        switched
            .payload
            .get("active_session_id")
            .and_then(Value::as_str),
        Some(sid.as_str())
    );
    let cross_session = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"What did I say earlier?"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("cross session message");
    assert_eq!(cross_session.status, 200);
    assert!(
        cross_session
            .payload
            .pointer("/context_pool/pool_messages")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            >= 2
    );
    assert_eq!(
        cross_session
            .payload
            .pointer("/context_pool/cross_session_memory_enabled")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        cross_session
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("Here's what I remember from earlier:"),
        "cross-session recall should be remediated to user-facing recall text"
    );
    assert!(
        !cross_session
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("Persistent memory is enabled for this agent across"),
        "internal memory metadata should never leak into user-facing responses"
    );

    let configured = handle(
        root.path(),
        "PATCH",
        &format!("/api/agents/{agent_id}/config"),
        br#"{
              "mode":"focus",
              "git_branch":"feature/jarvis",
              "identity":{"emoji":"robot","color":"00ff00","archetype":"director","vibe":"direct"}
            }"#,
        &agent_create_ok_snapshot(),
    )
    .expect("config");
    assert_eq!(
        configured.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );

    let model = handle(
        root.path(),
        "PUT",
        &format!("/api/agents/{agent_id}/model"),
        br#"{"model":"openai/gpt-5"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("set model");
    assert_eq!(
        model.payload.get("provider").and_then(Value::as_str),
        Some("openai")
    );
    assert_eq!(
        model.payload.get("model").and_then(Value::as_str),
        Some("gpt-5")
    );

    let after_model = handle(
        root.path(),
        "GET",
        &format!("/api/agents/{agent_id}"),
        &[],
        &agent_create_ok_snapshot(),
    )
    .expect("agent after model");
    assert_eq!(
        after_model
            .payload
            .get("model_provider")
            .and_then(Value::as_str),
        Some("openai")
    );
    assert_eq!(
        after_model
            .payload
            .get("model_name")
            .and_then(Value::as_str),
        Some("gpt-5")
    );
    assert_eq!(
        after_model
            .payload
            .pointer("/identity/vibe")
            .and_then(Value::as_str),
        Some("direct")
    );

    let trees = handle(
        root.path(),
        "GET",
        &format!("/api/agents/{agent_id}/git-trees"),
        &[],
        &agent_create_ok_snapshot(),
    )
    .expect("git trees");
    let options = trees
        .payload
        .get("options")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(options.iter().any(|row| {
        row.get("branch")
            .and_then(Value::as_str)
            .map(|v| v == "main")
            .unwrap_or(false)
    }));
    let switched_tree = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/git-tree/switch"),
        br#"{"branch":"feature/jarvis"}"#,
        &agent_create_ok_snapshot(),
    )
    .expect("git tree switch");
    assert_eq!(
        switched_tree
            .payload
            .pointer("/current/git_branch")
            .and_then(Value::as_str),
        Some("feature/jarvis")
    );
}

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
