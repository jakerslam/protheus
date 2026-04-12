#[cfg(test)]
mod openclaw_media_tool_shared_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn openclaw_media_tool_shared_local_roots_do_not_widen_from_media_sources() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        let workspace_dir = root.join("state").join("workspace-agent");
        let external_photo = "/Users/peter/Pictures/photo.png".to_string();
        let external_video = "file:///Users/peter/Movies/clip.mp4".to_string();

        let roots = resolve_media_tool_local_root_patterns(
            root,
            Some(workspace_dir.to_string_lossy().as_ref()),
            false,
            &[external_photo, external_video, "/top-level-file.png".to_string()],
        );

        assert!(roots.iter().any(|row| row == &workspace_dir.display().to_string()));
        assert!(roots.iter().any(|row| {
            row.ends_with("client/runtime/local/state/workspace")
        }));
        assert!(!roots.iter().any(|row| row == "/Users/peter/Pictures"));
        assert!(!roots.iter().any(|row| row == "/Users/peter/Movies"));
        assert!(!roots.iter().any(|row| row == "/"));
    }

    #[test]
    fn openclaw_media_tool_shared_normalizes_reference_inputs() {
        let request = json!({
            "images": [" @/tmp/a.png ", "/tmp/a.png", "/tmp/b.png"],
            "image": "@/tmp/c.png"
        });
        let rows =
            normalize_media_reference_inputs(&request, "image", "images", 4, "reference images")
                .expect("normalized");
        assert_eq!(
            rows,
            vec![
                "@/tmp/c.png".to_string(),
                "@/tmp/a.png".to_string(),
                "/tmp/b.png".to_string()
            ]
        );
    }

    #[test]
    fn openclaw_media_tool_shared_parses_boolean_strings() {
        let request = json!({
            "summaryOnly": "true",
            "host_read_capability": false
        });
        assert_eq!(media_tool_read_boolean_param(&request, "summary_only"), Some(true));
        assert_eq!(
            media_tool_read_boolean_param(&request, "host_read_capability"),
            Some(false)
        );
    }

    #[test]
    fn openclaw_media_generate_provider_list_action_result_reports_details() {
        let out = create_media_generate_provider_list_action_result(
            &[MediaGenerateProviderListRow {
                id: "openai".to_string(),
                default_model: Some("sora-mini".to_string()),
                models: vec!["sora-mini".to_string()],
                modes: vec!["generate".to_string()],
                auth_env_vars: vec!["OPENAI_API_KEY".to_string()],
                capabilities: json!({"generate": {"supportsAudio": true}}),
                capability_summary: "modes=generate, audio".to_string(),
            }],
            "No providers.",
        );
        assert_eq!(
            out.pointer("/details/providers/0/id").and_then(Value::as_str),
            Some("openai")
        );
        assert_eq!(
            out.pointer("/details/providers/0/authEnvVars/0")
                .and_then(Value::as_str),
            Some("OPENAI_API_KEY")
        );
        assert!(
            out.pointer("/content/0/text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("modes=generate, audio")
        );
    }

    #[test]
    fn openclaw_media_tool_contracts_surface_examples_in_status_and_providers() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("workspace root");
        let status = api_status(root);
        let providers = api_providers(root);
        assert_eq!(
            status
                .pointer("/media_request_contract/tool_shared_contract/boolean_param_contract/snake_case_reads_camel_case_alias")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            providers
                .pointer("/media_generation_action_contracts/video_generation/actions/2")
                .and_then(Value::as_str),
            Some("status")
        );
        assert!(
            providers
                .pointer("/media_generation_action_contracts/video_generation/duplicate_guard_example/content/0/text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("Do not call video_generate again")
        );
        assert_eq!(
            providers
                .pointer("/media_generation_action_contracts/music_generation/status_example/details/provider")
                .and_then(Value::as_str),
            Some("minimax")
        );
    }
}
