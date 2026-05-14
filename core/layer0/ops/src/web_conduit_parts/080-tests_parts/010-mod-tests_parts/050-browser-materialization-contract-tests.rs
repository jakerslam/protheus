
    #[test]
    fn browser_materialization_default_off_fails_closed_without_launch() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_browser_materialize_page(
            tmp.path(),
            &json!({
                "url": "https://example.com/research",
                "admission_ref": "test-browser-capability"
            }),
        );

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("web_conduit_browser_materialization")
        );
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("capability_not_enabled")
        );
        assert_eq!(
            out.get("browser_launch_attempted").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.get("tool_execution_attempted").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/url_safety/status").and_then(Value::as_str),
            Some("allowed")
        );
        assert_eq!(
            out.pointer("/profile_compilation/status").and_then(Value::as_str),
            Some("prepared_capability_disabled")
        );
        assert_eq!(
            out.get("raw_payload_chat_visible").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/materialized_page_contract/schema_ref")
                .and_then(Value::as_str),
            Some("web_research.browser_materialized_page.v1")
        );
        assert!(out
            .pointer("/materialized_page_contract/fields")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("main_text_or_markdown")))
            .unwrap_or(false));
        assert_eq!(
            out.pointer("/evidence_handoff_contract/target_lane")
                .and_then(Value::as_str),
            Some("candidate_enrichment")
        );
        assert_eq!(
            out.pointer("/evidence_handoff_contract/evidence_candidate_state")
                .and_then(Value::as_str),
            Some("not_created")
        );
        assert_eq!(
            out.pointer("/artifact_quarantine/raw_payload_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn browser_materialization_rejects_caller_supplied_browser_controls() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_browser_materialize_page(
            tmp.path(),
            &json!({
                "url": "https://example.com/research",
                "admission_ref": "test-browser-capability",
                "browser_args": ["--disable-web-security"]
            }),
        );

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("unsafe_caller_control_rejected")
        );
        assert_eq!(
            out.get("browser_launch_attempted").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/url_safety/status").and_then(Value::as_str),
            Some("not_evaluated")
        );
        assert!(
            out.get("reason")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .contains("browser_args")
        );
    }

    #[test]
    fn browser_materialization_blocks_credentialed_urls_before_adapter_execution() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_browser_materialize_page(
            tmp.path(),
            &json!({
                "url": "https://user:secret@example.com/research",
                "admission_ref": "test-browser-capability"
            }),
        );

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("url_safety_blocked")
        );
        assert_eq!(
            out.pointer("/url_safety/status").and_then(Value::as_str),
            Some("blocked_url_credentials")
        );
        assert_eq!(
            out.get("tool_execution_attempted").and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn browser_materialization_enabled_without_adapter_is_adapter_not_ready() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut policy = default_policy();
        let browser_config = policy
            .pointer_mut("/web_conduit/browser_materialization")
            .expect("browser materialization policy");
        browser_config["enabled"] = json!(true);
        browser_config["adapter_ready"] = json!(false);
        write_json_atomic(&policy_path(tmp.path()), &policy).expect("write policy");

        let out = api_browser_materialize_page(
            tmp.path(),
            &json!({
                "url": "https://example.com/research",
                "admission_ref": "test-browser-capability"
            }),
        );

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("adapter_not_ready")
        );
        assert_eq!(
            out.get("browser_launch_attempted").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/status").and_then(Value::as_str),
            Some("blocked_adapter_not_ready")
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/state").and_then(Value::as_str),
            Some("not_installed")
        );
    }

    #[test]
    fn browser_materialization_ready_adapter_still_uses_stub_until_adapter_exists() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut policy = default_policy();
        let browser_config = policy
            .pointer_mut("/web_conduit/browser_materialization")
            .expect("browser materialization policy");
        browser_config["enabled"] = json!(true);
        browser_config["adapter_ready"] = json!(true);
        write_json_atomic(&policy_path(tmp.path()), &policy).expect("write policy");

        let out = api_browser_materialize_page(
            tmp.path(),
            &json!({
                "url": "https://example.com/research",
                "admission_ref": "test-browser-capability"
            }),
        );

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("browser_adapter_stub_only")
        );
        assert_eq!(
            out.get("browser_launch_attempted").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/status").and_then(Value::as_str),
            Some("ready_for_adapter")
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/state").and_then(Value::as_str),
            Some("ready")
        );
        assert!(out.get("materialized_page").map(Value::is_null).unwrap_or(false));
        assert_eq!(
            out.pointer("/evidence_handoff_contract/browser_success_is_not_source_truth_without_packaging")
                .and_then(Value::as_bool),
            Some(true)
        );
    }
