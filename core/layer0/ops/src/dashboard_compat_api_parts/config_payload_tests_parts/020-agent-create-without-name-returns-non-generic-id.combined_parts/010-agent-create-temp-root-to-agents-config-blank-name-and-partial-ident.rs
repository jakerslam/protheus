fn agent_create_temp_root() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn agent_create_ok_snapshot() -> Value {
    json!({"ok": true})
}

#[test]
fn agent_create_without_name_returns_non_generic_identity_name() {
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
        handle(root.path(), "GET", "/api/agents", &[], &agent_create_ok_snapshot()).expect("list agents");
    let rows = listed.payload.as_array().cloned().unwrap_or_default();
    assert!(rows.iter().any(|row| {
        clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 120) == name
    }));
}

#[test]
fn large_param_models_preserve_default_name_during_post_init_seed() {
    let root = agent_create_temp_root();
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
        br#"{"system_prompt":"seed intro message"}"#,
        &agent_create_ok_snapshot(),
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
    let root = agent_create_temp_root();
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Starter","role":"analyst"}"#,
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
        br#"{"name":"","identity":{"vibe":"calm"}}"#,
        &agent_create_ok_snapshot(),
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
