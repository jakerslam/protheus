
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browse_and_search_are_paginated() {
        let root = tempfile::tempdir().expect("tempdir");
        let browse = handle(
            root.path(),
            "GET",
            "/api/clawhub/browse?sort=downloads&limit=5",
            &json!({}),
            &[],
        )
        .expect("browse");
        let rows = browse
            .payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(rows.len(), 5);
        let search = handle(
            root.path(),
            "GET",
            "/api/clawhub/search?q=router&limit=10",
            &json!({}),
            &[],
        )
        .expect("search");
        let search_rows = search
            .payload
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(search_rows.iter().any(|row| {
            row.get("slug")
                .and_then(Value::as_str)
                .map(|v| v.contains("router"))
                .unwrap_or(false)
        }));
    }

    #[test]
    fn install_create_uninstall_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let installed = handle(
            root.path(),
            "POST",
            "/api/clawhub/install",
            &json!({}),
            br#"{"slug":"model-router-pro"}"#,
        )
        .expect("install");
        assert_eq!(installed.status, 200);
        let listed = handle(root.path(), "GET", "/api/skills", &json!({}), &[]).expect("skills");
        let rows = listed
            .payload
            .get("skills")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.iter().any(|row| {
            row.get("name")
                .and_then(Value::as_str)
                .map(|v| v == "model-router-pro")
                .unwrap_or(false)
        }));
        let core_registry = read_json(&root.path().join(CORE_SKILLS_REGISTRY_REL))
            .expect("core registry after install");
        let core_prompt = core_registry
            .get("installed")
            .and_then(Value::as_object)
            .and_then(|rows| rows.get("model-router-pro"))
            .and_then(|row| row.get("prompt_context"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            !core_prompt.is_empty(),
            "core registry should persist prompt context"
        );

        let created = handle(
            root.path(),
            "POST",
            "/api/skills/create",
            &json!({}),
            br#"{"name":"my-demo-skill","description":"demo","runtime":"prompt_only","prompt_context":"ctx"}"#,
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
