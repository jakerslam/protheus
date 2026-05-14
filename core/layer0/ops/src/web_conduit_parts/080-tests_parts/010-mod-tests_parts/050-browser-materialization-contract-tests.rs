
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
            out.pointer("/pre_navigation_url_safety/status")
                .and_then(Value::as_str),
            Some("allowed")
        );
        assert_eq!(
            out.pointer("/final_url_safety/status")
                .and_then(Value::as_str),
            Some("not_observed")
        );
        assert_eq!(
            out.pointer("/navigation_contract/final_url_revalidation_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_strategy/strategy")
                .and_then(Value::as_str),
            Some("smart_dom_settle_default")
        );
        assert_eq!(
            out.pointer("/readiness_strategy/caller_raw_wait_script_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/context_contract/caller_context_options_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/context_contract/close_browser_on_context_creation_failure")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/context_contract/context_close_closes_browser")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/context_contract/persistent_context_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/cleanup_status/status").and_then(Value::as_str),
            Some("not_started")
        );
        assert_eq!(
            out.pointer("/retry_diagnostics/hidden_retry_executed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/status").and_then(Value::as_str),
            Some("prepared_capability_disabled")
        );
        assert_eq!(
            out.pointer("/profile_compilation/argument_compiler/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_build_args")
        );
        assert_eq!(
            out.pointer("/profile_compilation/argument_compiler/dedupe_key")
                .and_then(Value::as_str),
            Some("chromium_flag_name_before_equals")
        );
        assert_eq!(
            out.pointer("/profile_compilation/argument_compiler/caller_supplied_args_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/proxy_contract/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_proxy_url_resolution")
        );
        assert_eq!(
            out.pointer("/profile_compilation/proxy_contract/separate_capability_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/proxy_contract/raw_proxy_credentials_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/geo_consistency_contract/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_geoip_exit_ip_consistency")
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/geo_consistency_contract/direct_request_geo_fields_allowed",
            )
            .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/geo_consistency_contract/external_geo_db_download_allowed_during_research",
            )
            .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/geo_consistency_contract/raw_exit_ip_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/adapter_parity_contract/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_puppeteer_playwright_parity")
        );
        assert_eq!(
            out.pointer("/profile_compilation/adapter_parity_contract/direct_backend_selection_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/adapter_parity_contract/same_proxy_contract_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_platform_version_cache_contract")
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/surprise_download_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/raw_binary_path_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/download_install_contract/checksum_verification_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/download_install_contract/archive_path_traversal_rejected")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/update_contract/background_update_during_ordinary_research_allowed")
                .and_then(Value::as_bool),
            Some(false)
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
                .any(|row| row.as_str() == Some("final_url_safety")))
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
            out.pointer("/retry_diagnostics/retry_recommendation")
                .and_then(Value::as_str),
            Some("do_not_retry_without_request_change")
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
    fn browser_materialization_rejects_strategy_and_extra_args_from_callers() {
        let tmp = tempfile::tempdir().expect("tempdir");
        for field in ["extra_args", "_strategy_args"] {
            let out = api_browser_materialize_page(
                tmp.path(),
                &json!({
                    "url": "https://example.com/research",
                    "admission_ref": "test-browser-capability",
                    field: ["--ignore-certificate-errors"]
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
            assert!(
                out.get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .contains(field)
            );
        }
    }

    #[test]
    fn browser_materialization_rejects_direct_playwright_profile_overrides() {
        let tmp = tempfile::tempdir().expect("tempdir");
        for (field, value) in [
            ("launchOptions", json!({"slowMo": 100})),
            ("contextOptions", json!({"locale": "en-US"})),
            ("args", json!(["--disable-web-security"])),
            ("stealthArgs", json!(false)),
            ("backend", json!("puppeteer")),
            ("browserBackend", json!("playwright")),
            ("adapter_kind", json!("puppeteer")),
            ("viewport", json!({"width": 1024, "height": 768})),
            ("userAgent", json!("custom-agent")),
            ("timezoneId", json!("America/New_York")),
            ("locale", json!("en-US")),
            ("humanize", json!(true)),
            ("geoip", json!(true)),
            ("userDataDir", json!("/tmp/persistent-profile")),
            ("storageState", json!({"cookies": []})),
        ] {
            let mut request = json!({
                "url": "https://example.com/research",
                "admission_ref": "test-browser-capability"
            });
            request[field] = value;
            let out = api_browser_materialize_page(tmp.path(), &request);

            assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
            assert_eq!(
                out.get("error").and_then(Value::as_str),
                Some("unsafe_caller_control_rejected")
            );
            assert!(
                out.get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .contains(field)
            );
            assert_eq!(
                out.get("browser_launch_attempted").and_then(Value::as_bool),
                Some(false)
            );
        }
    }

    #[test]
    fn browser_materialization_rejects_non_http_schemes_before_execution() {
        let tmp = tempfile::tempdir().expect("tempdir");
        for url in [
            "file:///etc/passwd",
            "data:text/html,<h1>nope</h1>",
            "javascript:alert(1)",
            "chrome://settings",
            "about:blank",
            "ftp://example.com/file",
            "http://",
        ] {
            let out = api_browser_materialize_page(
                tmp.path(),
                &json!({
                    "url": url,
                    "admission_ref": "test-browser-capability"
                }),
            );

            assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
            assert_eq!(
                out.get("error").and_then(Value::as_str),
                Some("url_safety_blocked")
            );
            assert_eq!(
                out.get("tool_execution_attempted").and_then(Value::as_bool),
                Some(false)
            );
            assert_eq!(
                out.pointer("/url_safety/status").and_then(Value::as_str),
                Some("invalid_url")
            );
        }
    }

    #[test]
    fn browser_materialization_blocks_private_and_internal_targets() {
        let tmp = tempfile::tempdir().expect("tempdir");
        for url in [
            "http://169.254.169.254/latest/meta-data/",
            "http://127.0.0.1",
            "http://localhost",
            "http://10.0.0.1",
            "http://172.16.0.1",
            "http://192.168.1.1",
            "http://0.0.0.0",
            "http://[::1]",
            "http://[::ffff:127.0.0.1]",
            "http://100.64.0.1",
        ] {
            let out = api_browser_materialize_page(
                tmp.path(),
                &json!({
                    "url": url,
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
                Some("private_network_blocked")
            );
            assert_eq!(
                out.get("tool_execution_attempted").and_then(Value::as_bool),
                Some(false)
            );
        }
    }

    #[test]
    fn browser_materialization_accepts_case_insensitive_http_scheme() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_browser_materialize_page(
            tmp.path(),
            &json!({
                "url": "HTTP://93.184.216.34/research",
                "admission_ref": "test-browser-capability"
            }),
        );

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("capability_not_enabled")
        );
        assert_eq!(
            out.pointer("/url_safety/status").and_then(Value::as_str),
            Some("allowed")
        );
        assert_eq!(
            out.pointer("/url_safety/host").and_then(Value::as_str),
            Some("93.184.216.34")
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
        assert_eq!(
            out.pointer("/retry_diagnostics/retry_recommendation")
                .and_then(Value::as_str),
            Some("do_not_retry_without_request_change")
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
        assert_eq!(
            out.pointer("/retry_diagnostics/retry_recommendation")
                .and_then(Value::as_str),
            Some("satisfy_adapter_readiness_before_retry")
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
            out.pointer("/retry_diagnostics/retry_recommendation")
                .and_then(Value::as_str),
            Some("implement_or_admit_browser_adapter_before_retry")
        );
        assert_eq!(
            out.pointer("/evidence_handoff_contract/browser_success_is_not_source_truth_without_packaging")
                .and_then(Value::as_bool),
            Some(true)
        );
    }
