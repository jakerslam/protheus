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
