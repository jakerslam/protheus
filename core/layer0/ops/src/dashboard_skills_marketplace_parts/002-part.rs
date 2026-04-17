        )
        .expect("create");
        assert_eq!(created.status, 200);
        let core_registry_after_create = read_json(&root.path().join(CORE_SKILLS_REGISTRY_REL))
            .expect("core registry after create");
        assert!(core_registry_after_create
            .get("installed")
            .and_then(Value::as_object)
            .and_then(|rows| rows.get("my-demo-skill"))
            .is_some());
        let removed = handle(
            root.path(),
            "POST",
            "/api/skills/uninstall",
            &json!({}),
            br#"{"name":"my-demo-skill"}"#,
        )
        .expect("uninstall");
        assert_eq!(removed.status, 200);
        let core_registry_after_remove = read_json(&root.path().join(CORE_SKILLS_REGISTRY_REL))
            .expect("core registry after remove");
        assert!(core_registry_after_remove
            .get("installed")
            .and_then(Value::as_object)
            .and_then(|rows| rows.get("my-demo-skill"))
            .is_none());
    }

    #[test]
    fn mcp_payload_normalizes_from_array() {
        let root = tempfile::tempdir().expect("tempdir");
        let snapshot = json!({
            "skills": {
                "upstream": {
                    "mcp_servers": [
                        {"name":"figma","connected":true},
                        {"name":"linear","connected":false}
                    ]
                }
            }
        });
        let out = handle(root.path(), "GET", "/api/mcp/servers", &snapshot, &[]).expect("mcp");
        assert_eq!(
            out.payload.get("total_connected").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            out.payload.get("total_configured").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            out.payload
                .get("servers")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(2)
        );
    }

    #[test]
    fn prompt_context_emits_only_enabled_rows_with_context() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(DASHBOARD_SKILLS_STATE_REL),
            &json!({
                "installed": {
                    "alpha": {
                        "name": "alpha",
                        "enabled": true,
                        "has_prompt_context": true,
                        "prompt_context": "alpha context"
                    },
                    "beta": {
                        "name": "beta",
                        "enabled": false,
                        "has_prompt_context": true,
                        "prompt_context": "beta context"
                    }
                },
                "created": {}
            }),
        );
        let out = skills_prompt_context(root.path(), 8, 2000);
        assert!(out.contains("alpha context"));
        assert!(!out.contains("beta context"));
    }
}
