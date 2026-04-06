#[test]
fn agent_create_without_name_returns_non_generic_identity_name() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"role":"analyst"}"#,
        &json!({"ok": true}),
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
    let name = clean_text(
        created
            .payload
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    assert!(!name.is_empty());
    assert_eq!(name, agent_id);
    let listed =
        handle(root.path(), "GET", "/api/agents", &[], &json!({"ok": true})).expect("list agents");
    let rows = listed.payload.as_array().cloned().unwrap_or_default();
    assert!(rows.iter().any(|row| {
        clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 120) == name
    }));
}

#[test]
fn large_param_models_preserve_default_name_during_post_init_seed() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let registry_path = root.path().join(
        "client/runtime/local/state/ui/infring_dashboard/provider_registry.json",
    );
    if let Some(parent) = registry_path.parent() {
        fs::create_dir_all(parent).expect("provider registry parent");
    }
    fs::write(
        &registry_path,
        serde_json::to_string_pretty(&json!({
            "type": "infring_dashboard_provider_registry",
            "providers": {
                "ollama": {
                    "id": "ollama",
                    "is_local": true,
                    "needs_key": false,
                    "auth_status": "ok",
                    "reachable": true,
                    "model_profiles": {
                        "selfname-120b": {
                            "power_rating": 5,
                            "cost_rating": 2,
                            "param_count_billion": 120,
                            "specialty": "general"
                        }
                    }
                }
            }
        }))
        .expect("provider registry json"),
    )
    .expect("write provider registry");

    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"role":"analyst","provider":"ollama","model":"selfname-120b"}"#,
        &json!({"ok": true}),
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
        br#"{"system_prompt":"seed intro message"}"#,
        &json!({"ok": true}),
    )
    .expect("config patch");
    assert_eq!(configured.status, 200);

    let resulting_name = clean_text(
        configured
            .payload
            .pointer("/agent/name")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    assert_eq!(
        resulting_name, agent_id,
        ">=80B models should keep default name so the model can self-name later"
    );
    assert!(
        configured.payload.get("rename_notice").is_none(),
        "post-init auto-rename should not fire for >=80B models"
    );
}

#[test]
fn agents_config_blank_name_and_partial_identity_are_auto_normalized() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Starter","role":"analyst"}"#,
        &json!({"ok": true}),
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
        br#"{"name":"","identity":{"vibe":"calm"}}"#,
        &json!({"ok": true}),
    )
    .expect("config");
    assert_eq!(configured.status, 200);
    assert_eq!(
        configured.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );

    let profiles_path = root
        .path()
        .join("client/runtime/local/state/ui/infring_dashboard/agent_profiles.json");
    let profiles_raw = fs::read_to_string(&profiles_path).expect("profiles state");
    let profiles = serde_json::from_str::<Value>(&profiles_raw).expect("profiles json");
    let profile = profiles
        .get("agents")
        .and_then(Value::as_object)
        .and_then(|agents| agents.get(&agent_id))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let stored_name = clean_text(
        profile.get("name").and_then(Value::as_str).unwrap_or(""),
        120,
    );
    let stored_emoji = clean_text(
        profile
            .pointer("/identity/emoji")
            .and_then(Value::as_str)
            .unwrap_or(""),
        24,
    );
    assert!(
        stored_name.eq("Starter"),
        "blank name patch should keep the existing configured name"
    );
    assert!(
        stored_emoji.eq("∞"),
        "partial identity patch should preserve the default Infring symbol"
    );
    assert_eq!(
        profile
            .pointer("/identity/vibe")
            .and_then(Value::as_str)
            .unwrap_or(""),
        "calm"
    );
}

#[test]
fn repeated_default_agent_creation_avoids_double_agent_prefix() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());

    let first = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"role":"analyst"}"#,
        &json!({"ok": true}),
    )
    .expect("create first agent");
    assert_eq!(first.status, 200);

    let second = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"role":"analyst"}"#,
        &json!({"ok": true}),
    )
    .expect("create second agent");
    assert_eq!(second.status, 200);

    let second_agent_id = clean_text(
        second
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let second_name = clean_text(
        second
            .payload
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    assert!(
        !second_agent_id.starts_with("agent-agent-"),
        "agent_id should not duplicate the agent prefix: {second_agent_id}"
    );
    assert!(
        !second_name.starts_with("agent-agent-"),
        "display name should not duplicate the agent prefix: {second_name}"
    );
    assert!(
        second_agent_id.starts_with("agent-"),
        "agent_id should keep the default agent prefix: {second_agent_id}"
    );
    assert_eq!(second_name, second_agent_id);
}

#[test]
fn permanent_lifespan_agents_do_not_auto_expire() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Permanent","role":"analyst","contract":{"lifespan":"permanent"}}"#,
        &json!({"ok": true}),
    )
    .expect("create permanent agent");
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

    let details = handle(
        root.path(),
        "GET",
        &format!("/api/agents/{agent_id}"),
        &[],
        &json!({"ok": true}),
    )
    .expect("agent details");
    assert_eq!(
        details
            .payload
            .pointer("/contract/termination_condition")
            .and_then(Value::as_str)
            .map(|value| value.to_ascii_lowercase()),
        Some("manual".to_string())
    );
    assert_eq!(
        details
            .payload
            .pointer("/contract/auto_terminate_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        details
            .payload
            .pointer("/contract/idle_terminate_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    let enforcement = crate::dashboard_agent_state::enforce_expired_contracts(root.path());
    let terminated = enforcement
        .get("terminated")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        !terminated.iter().any(|row| {
            clean_text(row.get("agent_id").and_then(Value::as_str).unwrap_or(""), 180) == agent_id
        }),
        "permanent agent should not be terminated by expiry enforcement"
    );
}

#[test]
fn identity_hydration_prompt_uses_agent_metadata() {
    let row = json!({
        "id": "agent-lucas",
        "name": "Lucas",
        "role": "engineer",
        "identity": {
            "archetype": "coder",
            "vibe": "friendly"
        },
        "system_prompt": "Stay practical and calm. Keep responses concise."
    });
    let prompt = agent_identity_hydration_prompt(&row);
    assert!(prompt.contains("name=Lucas"), "prompt should carry agent name");
    assert!(
        prompt.contains("role=engineer"),
        "prompt should carry agent role"
    );
    assert!(
        prompt.contains("archetype=coder"),
        "prompt should carry agent archetype"
    );
    assert!(prompt.contains("vibe=friendly"), "prompt should carry agent vibe");
    assert!(
        prompt.contains("Personality directive: Stay practical and calm."),
        "prompt should carry a brief personality hydration sentence"
    );
}

#[test]
fn agents_routes_create_message_config_and_git_tree_round_trip() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Jarvis","role":"director","provider":"ollama","model":"qwen:4b"}"#,
        &json!({"ok": true}),
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
        handle(root.path(), "GET", "/api/agents", &[], &json!({"ok": true})).expect("list agents");
    let rows = listed.payload.as_array().cloned().unwrap_or_default();
    assert!(rows.iter().any(|row| {
        clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 180) == agent_id
    }));

    let details = handle(
        root.path(),
        "GET",
        &format!("/api/agents/{agent_id}"),
        &[],
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"role":"engineer"}"#,
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"role":"analyst"}"#,
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
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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

#[test]
fn agent_init_config_treats_default_name_as_blank_for_auto_name_and_analyst_intro() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"role":"analyst"}"#,
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

    let configured = handle(
        root.path(),
        "PATCH",
        &format!("/api/agents/{agent_id}/config"),
        format!(
            "{{\"name\":\"{}\",\"system_prompt\":\"You are an analyst.\",\"archetype\":\"analyst\",\"profile\":\"analysis\",\"contract\":{{\"mission\":\"Analyze outcomes\",\"termination_condition\":\"task_or_timeout\",\"expiry_seconds\":3600}}}}",
            agent_id
        )
        .as_bytes(),
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
        &json!({"ok": true}),
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
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Runtime Probe","role":"analyst"}"#,
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

    let message = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"Report runtime sync now. What changed in queue depth, cockpit blocks, conduit signals, and memory context?"}"#,
            &json!({"ok": true}),
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

#[test]
fn memory_denial_variant_is_remediated_to_persistent_summary() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Memory Probe","role":"analyst"}"#,
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

    let seeded = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"Remember this exactly: favorite animal is octopus and codename aurora-7."}"#,
            &json!({"ok": true}),
        )
        .expect("seed memory");
    assert_eq!(seeded.status, 200);

    let second = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/sessions"),
        br#"{"label":"Session 2"}"#,
        &json!({"ok": true}),
    )
    .expect("create second session");
    let sid = clean_text(
        second
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
        &json!({"ok": true}),
    )
    .expect("switch second session");
    assert_eq!(switched.status, 200);

    let denial_variant = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"I still do not see any stored memory context from earlier in this session. I do not retain information between exchanges unless you explicitly use a memory conduit, and I can only work with what is in the current message."}"#,
            &json!({"ok": true}),
        )
        .expect("denial variant message");
    assert_eq!(denial_variant.status, 200);
    let response = denial_variant
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        response.contains("Here's what I remember from earlier:")
            || response.contains("octopus")
            || response.contains("aurora-7"),
        "memory denial variant should be remediated to user-facing memory recall"
    );
    assert!(
        !response
            .to_ascii_lowercase()
            .contains("do not retain information between exchanges"),
        "raw denial text should not leak back to caller"
    );
}

#[test]
fn internal_recalled_context_metadata_is_not_echoed_to_user() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Context Leak Guard","role":"analyst"}"#,
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

    let message = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Persistent memory is enabled for this agent across 1 session(s) with 12 stored messages. Recalled context: alpha | beta | gamma"}"#,
        &json!({"ok": true}),
    )
    .expect("metadata dump probe");
    assert_eq!(message.status, 200);
    let response = message
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(
        !response.contains("persistent memory is enabled for this agent across"),
        "internal metadata banner must never be returned as user-visible output"
    );
    assert!(
        !response.contains("recalled context:"),
        "recalled-context scaffolding must never be user-visible output"
    );
}
