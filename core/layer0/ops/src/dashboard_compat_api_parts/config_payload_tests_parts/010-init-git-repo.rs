use super::*;

fn init_git_repo(root: &Path) {
    let status = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(root)
        .status()
        .expect("git init");
    assert!(status.success());
    let status = Command::new("git")
        .args(["config", "user.email", "codex@example.com"])
        .current_dir(root)
        .status()
        .expect("git config email");
    assert!(status.success());
    let status = Command::new("git")
        .args(["config", "user.name", "Codex"])
        .current_dir(root)
        .status()
        .expect("git config name");
    assert!(status.success());
    let _ = fs::write(root.join("README.md"), "dashboard test repo\n");
    let status = Command::new("git")
        .args(["add", "README.md"])
        .current_dir(root)
        .status()
        .expect("git add");
    assert!(status.success());
    let status = Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(root)
        .status()
        .expect("git commit");
    assert!(status.success());
}

#[test]
fn providers_endpoint_uses_registry_rows() {
    let root = tempfile::tempdir().expect("tempdir");
    write_json(
        &state_path(root.path(), PROVIDER_REGISTRY_REL),
        &json!({
            "type": "infring_dashboard_provider_registry",
            "providers": {
                "ollama": {"id": "ollama", "display_name": "Ollama", "is_local": true, "needs_key": false},
                "openai": {"id": "openai", "display_name": "OpenAI", "is_local": false, "needs_key": true}
            }
        }),
    );
    let out = handle(
        root.path(),
        "GET",
        "/api/providers",
        &[],
        &json!({"ok": true}),
    )
    .expect("providers");
    let rows = out
        .payload
        .get("providers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(rows.len() >= 2);
    assert!(rows
        .iter()
        .any(|row| { row.get("id").and_then(Value::as_str) == Some("openai") }));
    assert!(rows
        .iter()
        .any(|row| { row.get("id").and_then(Value::as_str) == Some("ollama") }));
}

#[test]
fn status_and_auth_endpoints_are_rust_authoritative() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let status = handle_with_headers(
        root.path(),
        "GET",
        "/api/status",
        &[],
        &[("Host", "127.0.0.1:4173")],
        &json!({
            "ok": true,
            "runtime_sync": {
                "summary": {
                    "queue_depth": 2,
                    "conduit_signals": 5,
                    "backpressure_level": "normal"
                }
            }
        }),
    )
    .expect("status");
    assert_eq!(status.status, 200);
    assert_eq!(
        status.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        status.payload.get("api_listen").and_then(Value::as_str),
        Some("127.0.0.1:4173")
    );
    assert_eq!(
        status
            .payload
            .pointer("/runtime_sync/queue_depth")
            .and_then(Value::as_i64),
        Some(2)
    );
    let auth = handle(
        root.path(),
        "GET",
        "/api/auth/check",
        &[],
        &json!({"ok": true}),
    )
    .expect("auth");
    assert_eq!(auth.status, 200);
    assert_eq!(
        auth.payload.get("authenticated").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn channels_endpoint_returns_catalog_defaults() {
    let root = tempfile::tempdir().expect("tempdir");
    let out = handle(
        root.path(),
        "GET",
        "/api/channels",
        &[],
        &json!({"ok": true}),
    )
    .expect("channels");
    let rows = out
        .payload
        .get("channels")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(rows.len() >= 40);
    assert!(rows.iter().any(|row| {
        row.get("name")
            .and_then(Value::as_str)
            .map(|v| v == "whatsapp")
            .unwrap_or(false)
    }));
    assert!(rows.iter().any(|row| {
        row.get("name")
            .and_then(Value::as_str)
            .map(|v| v == "gohighlevel")
            .unwrap_or(false)
    }));
}

#[test]
fn channels_configure_and_test_round_trip() {
    let root = tempfile::tempdir().expect("tempdir");
    let configure = handle(
        root.path(),
        "POST",
        "/api/channels/discord/configure",
        br#"{"fields":{"bot_token":"abc","channel_id":"123"}}"#,
        &json!({"ok": true}),
    )
    .expect("configure");
    assert_eq!(configure.status, 200);
    let test = handle(
        root.path(),
        "POST",
        "/api/channels/discord/test",
        &[],
        &json!({"ok": true}),
    )
    .expect("test");
    assert_eq!(
        test.payload.get("status").and_then(Value::as_str),
        Some("ok")
    );
}

#[test]
fn gohighlevel_channel_test_requires_location_id() {
    let root = tempfile::tempdir().expect("tempdir");
    let configure = handle(
        root.path(),
        "POST",
        "/api/channels/gohighlevel/configure",
        br#"{"fields":{"private_integration_token":"pit-test-token"}}"#,
        &json!({"ok": true}),
    )
    .expect("configure gohighlevel");
    assert_eq!(configure.status, 200);
    let test = handle(
        root.path(),
        "POST",
        "/api/channels/gohighlevel/test",
        &[],
        &json!({"ok": true}),
    )
    .expect("test gohighlevel");
    assert_eq!(
        test.payload.get("status").and_then(Value::as_str),
        Some("error")
    );
    assert!(test
        .payload
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("location_id"));
}

#[test]
fn route_decision_endpoint_prefers_local_when_offline() {
    let root = tempfile::tempdir().expect("tempdir");
    write_json(
        &state_path(root.path(), PROVIDER_REGISTRY_REL),
        &json!({
            "type": "infring_dashboard_provider_registry",
            "providers": {
                "ollama": {
                    "id": "ollama",
                    "is_local": true,
                    "needs_key": false,
                    "auth_status": "ok",
                    "model_profiles": {
                        "smallthinker:4b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 4, "specialty":"general"}
                    }
                },
                "openai": {
                    "id": "openai",
                    "is_local": false,
                    "needs_key": true,
                    "auth_status": "set",
                    "model_profiles": {
                        "gpt-5": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 70, "specialty":"general"}
                    }
                }
            }
        }),
    );
    let out = handle(
        root.path(),
        "POST",
        "/api/route/decision",
        br#"{"offline_required":true,"task_type":"general"}"#,
        &json!({"ok": true}),
    )
    .expect("route decision");
    assert_eq!(
        out.payload
            .get("selected")
            .and_then(|v| v.get("provider"))
            .and_then(Value::as_str),
        Some("ollama")
    );
}

#[test]
fn providers_routing_endpoint_updates_signed_policy() {
    let root = tempfile::tempdir().expect("tempdir");
    let update = handle(
        root.path(),
        "POST",
        "/api/providers/routing",
        br#"{
          "signature":"sig:test-routing-config-v1",
          "retry":{"max_attempts_per_route":3,"max_total_attempts":6},
          "fallback_chain":[
            {"provider":"moonshot","model":"kimi-k2.5"},
            {"provider":"openrouter","model":"deepseek/deepseek-chat-v3-0324:free"}
          ]
        }"#,
        &json!({"ok": true}),
    )
    .expect("routing update");
    assert_eq!(update.status, 200);
    assert_eq!(
        update.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    let fetch = handle(
        root.path(),
        "GET",
        "/api/providers/routing",
        &[],
        &json!({"ok": true}),
    )
    .expect("routing fetch");
    assert_eq!(
        fetch
            .payload
            .pointer("/policy/retry/max_total_attempts")
            .and_then(Value::as_i64),
        Some(6)
    );
    assert!(fetch
        .payload
        .pointer("/policy/policy_hash")
        .and_then(Value::as_str)
        .map(|value| !value.is_empty())
        .unwrap_or(false));
}

#[test]
fn virtual_key_budget_exhaustion_blocks_second_chat_turn() {
    let root = tempfile::tempdir().expect("tempdir");
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Budget Bot","role":"general"}"#,
        &json!({"ok": true}),
    )
    .expect("agent create");
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());
    let key = handle(
        root.path(),
        "POST",
        "/api/virtual-keys",
        br#"{
          "key_id":"team-alpha",
          "provider":"openai",
          "model":"gpt-5",
          "team_id":"alpha",
          "budget_limit_usd":0.000001,
          "rate_limit_rpm":100
        }"#,
        &json!({"ok": true}),
    )
    .expect("virtual key create");
    assert_eq!(key.status, 200);
    let first = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"hello","virtual_key_id":"team-alpha"}"#,
        &json!({"ok": true}),
    )
    .expect("first message");
    assert_eq!(first.status, 200);
    let second = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"hello again","virtual_key_id":"team-alpha"}"#,
        &json!({"ok": true}),
    )
    .expect("second message");
    assert_eq!(second.status, 402);
    assert_eq!(
        second.payload.get("error").and_then(Value::as_str),
        Some("virtual_key_budget_exceeded")
    );
}

#[test]
fn whatsapp_qr_start_exposes_data_url() {
    let root = tempfile::tempdir().expect("tempdir");
    let out = handle(
        root.path(),
        "POST",
        "/api/channels/whatsapp/qr/start",
        &[],
        &json!({"ok": true}),
    )
    .expect("qr");
    let url = out
        .payload
        .get("qr_data_url")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(url.starts_with("data:image/svg+xml;base64,"));
}
