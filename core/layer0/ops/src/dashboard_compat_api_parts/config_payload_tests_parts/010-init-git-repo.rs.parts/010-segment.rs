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
fn channels_endpoint_backfills_null_runtime_metadata() {
    let root = tempfile::tempdir().expect("tempdir");
    write_json(
        &state_path(
            root.path(),
            "client/runtime/local/state/ui/infring_dashboard/channel_registry.json",
        ),
        &json!({
            "type": "infring_dashboard_channel_registry",
            "channels": {
                "discord": {
                    "name": "discord",
                    "display_name": "Discord",
                    "runtime_adapter": Value::Null,
                    "probe_method": Value::Null,
                    "requires_token": Value::Null,
                    "runtime_supported": Value::Null,
                    "configured": false,
                    "has_token": false,
                    "fields": Value::Null
                }
            }
        }),
    );
    let out = handle(
        root.path(),
        "GET",
        "/api/channels",
        &[],
        &json!({"ok": true}),
    )
    .expect("channels");
    let discord = out
        .payload
        .get("channels")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find(|row| {
                row.get("name")
                    .and_then(Value::as_str)
                    .map(|v| v == "discord")
                    .unwrap_or(false)
            })
        })
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert_eq!(
        discord.get("runtime_adapter").and_then(Value::as_str),
        Some("discord_bot")
    );
    assert_eq!(
        discord.get("probe_method").and_then(Value::as_str),
        Some("get")
    );
    assert_eq!(
        discord.get("requires_token").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        discord.get("runtime_supported").and_then(Value::as_bool),
        Some(true)
    );
    assert!(discord.get("fields").and_then(Value::as_array).is_some());
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
    let channels_before = handle(
        root.path(),
        "GET",
        "/api/channels",
        &[],
        &json!({"ok": true}),
    )
    .expect("channels before");
    let discord_before = channels_before
        .payload
        .get("channels")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find(|row| {
                row.get("name")
                    .and_then(Value::as_str)
                    .map(|v| v == "discord")
                    .unwrap_or(false)
            })
        })
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert_eq!(
        discord_before.get("connected").and_then(Value::as_bool),
        Some(false)
    );
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
    assert!(test
        .payload
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("Run live test"));
}

#[test]
fn channels_force_live_requires_endpoint_for_generic_adapter() {
    let root = tempfile::tempdir().expect("tempdir");
    let configure = handle(
        root.path(),
        "POST",
        "/api/channels/matrix/configure",
        br#"{"fields":{"token":"abc"}}"#,
        &json!({"ok": true}),
    )
    .expect("configure");
    assert_eq!(configure.status, 200);
    let test = handle(
        root.path(),
        "POST",
        "/api/channels/matrix/test",
        br#"{"force_live":true}"#,
        &json!({"ok": true}),
    )
    .expect("test");
    assert_eq!(
        test.payload.get("status").and_then(Value::as_str),
        Some("error")
    );
    assert!(test
        .payload
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("endpoint"));

    let channels = handle(
        root.path(),
        "GET",
        "/api/channels",
        &[],
        &json!({"ok": true}),
    )
    .expect("channels");
    let matrix = channels
        .payload
        .get("channels")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find(|row| {
                row.get("name")
                    .and_then(Value::as_str)
                    .map(|v| v == "matrix")
                    .unwrap_or(false)
            })
        })
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert_eq!(
        matrix
            .get("live_probe")
            .and_then(Value::as_object)
            .and_then(|probe| probe.get("status"))
            .and_then(Value::as_str),
        Some("error")
    );
    assert_eq!(matrix.get("connected").and_then(Value::as_bool), Some(false));
}
