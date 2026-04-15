#[test]
fn channels_live_probe_marks_connected_when_adapter_verifies() {
    let root = tempfile::tempdir().expect("tempdir");
    let configure = handle(
        root.path(),
        "POST",
        "/api/channels/webchat/configure",
        br#"{"fields":{"workspace":"default"}}"#,
        &json!({"ok": true}),
    )
    .expect("configure");
    assert_eq!(configure.status, 200);
    let test = handle(
        root.path(),
        "POST",
        "/api/channels/webchat/test",
        br#"{"force_live":true}"#,
        &json!({"ok": true}),
    )
    .expect("test");
    assert_eq!(
        test.payload.get("status").and_then(Value::as_str),
        Some("ok")
    );
    let channels = handle(
        root.path(),
        "GET",
        "/api/channels",
        &[],
        &json!({"ok": true}),
    )
    .expect("channels");
    let webchat = channels
        .payload
        .get("channels")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find(|row| {
                row.get("name")
                    .and_then(Value::as_str)
                    .map(|v| v == "webchat")
                    .unwrap_or(false)
            })
        })
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert_eq!(
        webchat
            .get("live_probe")
            .and_then(Value::as_object)
            .and_then(|probe| probe.get("status"))
            .and_then(Value::as_str),
        Some("ok")
    );
    assert_eq!(webchat.get("connected").and_then(Value::as_bool), Some(true));
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
        br#"{"force_live":true}"#,
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
