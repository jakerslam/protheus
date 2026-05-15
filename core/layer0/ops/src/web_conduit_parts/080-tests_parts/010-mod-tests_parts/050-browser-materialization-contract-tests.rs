
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
            out.pointer("/context_contract/default_viewport_policy_owned")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/context_contract/locale_timezone_cdp_emulation_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/context_contract/generic_context_kwargs_allowed_from_workflow")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/context_contract/storage_state_requires_session_capability")
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
            out.pointer(
                "/profile_compilation/argument_compiler/single_effective_flag_per_key_required",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/argument_compiler/dedicated_locale_timezone_fields_override_compiled_args",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/argument_compiler/raw_fingerprint_args_allowed_from_workflow")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/argument_compiler/webrtc_auto_resolution_requires_admitted_proxy_exit_ip",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/argument_compiler/raw_webrtc_ip_chat_visible")
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
            out.pointer("/profile_compilation/proxy_contract/credentials_removed_from_server_url")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/proxy_contract/socks5h_scheme_supported_after_admission")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/proxy_contract/proxy_encoding_notice_redacts_credentials")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/proxy_contract/nonstandard_socks_path_query_rejected_before_adapter",
            )
            .and_then(Value::as_bool),
            Some(true)
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
            out.pointer(
                "/profile_compilation/geo_consistency_contract/geo_db_first_use_download_allowed_during_research",
            )
            .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/geo_consistency_contract/geo_db_atomic_temp_rename_required",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/geo_consistency_contract/exit_ip_echo_provider_order_policy_owned",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/geo_consistency_contract/country_locale_map_bcp47_required",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/geo_consistency_contract/fill_only_missing_locale_or_timezone",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/geo_consistency_contract/geo_timeout_preserves_existing_profile_fields",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/geo_consistency_contract/private_exit_or_proxy_ip_not_claim_evidence",
            )
            .and_then(Value::as_bool),
            Some(true)
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
            out.pointer("/profile_compilation/service_pool_contract/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_cdp_multiplexer_pool")
        );
        assert_eq!(
            out.pointer("/profile_compilation/service_pool_contract/workflow_raw_cdp_authority_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/service_pool_contract/cleanup_confined_to_service_data_dir")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/service_pool_contract/generic_fingerprint_query_params_allowed_from_workflow")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/service_pool_contract/remote_debugging_cli_flags_stripped_from_passthrough")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/service_pool_contract/service_data_dir_policy_owned")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/service_pool_contract/passthrough_browser_args_allowed_from_workflow")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/service_pool_contract/external_adapter_handoff_contract/source_pattern",
            )
            .and_then(Value::as_str),
            Some("cloakbrowser_cdp_integration_examples")
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/service_pool_contract/external_adapter_handoff_contract/capability_endpoint_ref_required",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/service_pool_contract/external_adapter_handoff_contract/raw_cdp_http_url_from_workflow_allowed",
            )
            .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/service_pool_contract/external_adapter_handoff_contract/adapter_output_must_reenter_evidence_pack",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/service_pool_contract/external_adapter_handoff_contract/raw_cdp_version_response_chat_visible",
            )
            .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/launch_execution_contract/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_basic_launch_contract_tests")
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/launch_execution_contract/page_navigation_success_is_not_evidence_without_packaging",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/launch_execution_contract/fingerprint_probe_results_telemetry_only",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/launch_execution_contract/raw_browser_handle_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/stealth_unit_test_contract/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_mock_fast_stealth_unit_tests")
        );
        assert_eq!(
            out.pointer("/profile_compilation/stealth_unit_test_contract/mock_fast_tests_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/stealth_unit_test_contract/selector_expression_escaping_tests_required",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/stealth_unit_test_contract/slow_external_site_tests_not_release_gate_by_default",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/stealth_unit_test_contract/raw_detection_hooks_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/humanize_unit_test_contract/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_humanize_unit_tests")
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/humanize_unit_test_contract/frame_locator_element_handle_patch_coverage_required",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/humanize_unit_test_contract/per_call_config_override_containment_required",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/humanize_unit_test_contract/slow_behavioral_detection_tests_quarantined",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/humanize_unit_test_contract/raw_behavior_detection_results_chat_visible",
            )
            .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/wrapper_lifecycle_contract/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_python_wrapper_lifecycle")
        );
        assert_eq!(
            out.pointer("/profile_compilation/wrapper_lifecycle_contract/async_cancellation_closes_browser")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/wrapper_lifecycle_contract/direct_persistent_user_data_dir_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/persistent_session_contract/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_persistent_context_tests")
        );
        assert_eq!(
            out.pointer("/profile_compilation/persistent_session_contract/separate_capability_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/persistent_session_contract/direct_user_data_dir_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/persistent_session_contract/session_profile_ref_broker_owned")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/persistent_session_contract/caller_supplied_args_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/persistent_session_contract/locale_timezone_binary_profile_fields_only")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/persistent_session_contract/locale_timezone_context_kwargs_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/persistent_session_contract/sync_async_close_stops_driver_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/persistent_session_contract/raw_persistent_profile_path_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_human_config_presets")
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/default_admitted")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/numeric_action_budget_schema_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/config_merge_must_not_mutate_base")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/page_frame_element_handle_patching_requires_capability")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/trusted_key_dispatch_requires_human_capability")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/raw_mouse_interface_exposed_to_workflow")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/bezier_mouse_path_policy_owned")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/click_targeting_requires_element_box")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/raw_mouse_coordinates_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/raw_keyboard_interface_exposed_to_workflow")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/typing_cadence_policy_owned")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/shift_symbol_dispatch_requires_cdp_session")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/raw_keyboard_text_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/raw_scroll_interface_exposed_to_workflow")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/wheel_delta_randomization_budget_bounded")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/scroll_to_selector_requires_interaction_capability")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/raw_scroll_coordinates_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/human_interaction_contract/raw_behavior_parameters_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/read_only_dom_probe_contract/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_isolated_world_dom_reads")
        );
        assert_eq!(
            out.pointer("/profile_compilation/read_only_dom_probe_contract/isolated_world_context_recreated_after_navigation")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/read_only_dom_probe_contract/caller_supplied_probe_scripts_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/read_only_dom_probe_contract/page_settling_probe_may_detect_scroll_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/read_only_dom_probe_contract/selector_scroll_requires_interaction_capability")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/read_only_dom_probe_contract/raw_probe_expression_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/default_config_contract/source_pattern")
                .and_then(Value::as_str),
            Some("cloakbrowser_python_config_defaults")
        );
        assert_eq!(
            out.pointer("/profile_compilation/default_config_contract/caller_ignore_default_args_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/profile_compilation/default_config_contract/random_fingerprint_seed_not_ordinary_research")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/default_config_contract/platform_binary_path_template_policy_owned",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/default_config_contract/cache_dir_env_override_allowed_only_operator_readiness",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/default_config_contract/unsupported_platform_fails_closed",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/profile_compilation/default_config_contract/gpu_fingerprint_flags_not_policy_surface",
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/profile_compilation/default_config_contract/raw_cache_dir_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
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
            out.pointer("/readiness_lifecycle/dependency_lifecycle/download_install_contract/checksum_manifest_standard_and_binary_mode_supported")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/download_install_contract/checksum_mismatch_blocks_install")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/download_install_contract/tar_gz_archive_supported")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/download_install_contract/zip_archive_supported")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/download_install_contract/custom_download_url_disables_public_fallback")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/download_install_contract/archive_path_traversal_rejected")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/download_install_contract/absolute_symlink_targets_skipped")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/download_install_contract/macos_app_bundle_flattening_denied")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/download_install_contract/multiple_top_level_entries_not_flattened")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/update_contract/background_update_during_ordinary_research_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/update_contract/update_checks_disabled_by_auto_update_env")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/update_contract/semantic_version_tuple_variable_length_allowed")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/update_contract/draft_releases_ignored")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/update_contract/platform_release_asset_match_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/update_contract/no_platform_release_asset_is_nonfatal_unavailable")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/binary_info_contract/install_result_revalidated_before_ready")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/readiness_lifecycle/dependency_lifecycle/binary_info_contract/raw_download_url_chat_visible")
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
            ("ignoreDefaultArgs", json!(["--enable-automation"])),
            ("download_url", json!("https://example.com/browser.tar.gz")),
            ("fingerprintSeed", json!(12345)),
            ("backend", json!("puppeteer")),
            ("browserBackend", json!("playwright")),
            ("adapter_kind", json!("puppeteer")),
            ("viewport", json!({"width": 1024, "height": 768})),
            ("userAgent", json!("custom-agent")),
            ("timezoneId", json!("America/New_York")),
            ("timezone_id", json!("America/New_York")),
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
        assert_eq!(
            out.pointer("/blocker_classification/blocker_class")
                .and_then(Value::as_str),
            Some("adapter_not_ready")
        );
        assert_eq!(
            out.pointer("/blocker_classification/retryable")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/retry_diagnostics/retry_budget/attempts_consumed")
                .and_then(Value::as_u64),
            Some(0)
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

    #[test]
    fn browser_materialization_fake_provider_returns_materialized_page_without_launch() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut policy = default_policy();
        let browser_config = policy
            .pointer_mut("/web_conduit/browser_materialization")
            .expect("browser materialization policy");
        browser_config["enabled"] = json!(true);
        browser_config["adapter_ready"] = json!(true);
        browser_config["provider_order"] = json!(["fake_materialization"]);
        write_json_atomic(&policy_path(tmp.path()), &policy).expect("write policy");

        let out = api_browser_materialize_page(
            tmp.path(),
            &json!({
                "url": "https://example.com/research",
                "admission_ref": "test-browser-capability"
            }),
        );

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("provider").and_then(Value::as_str), Some("fake_materialization"));
        assert_eq!(
            out.get("tool_execution_attempted").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.get("browser_launch_attempted").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/materialized_page/provider").and_then(Value::as_str),
            Some("fake_materialization")
        );
        assert_eq!(
            out.pointer("/materialized_page/title").and_then(Value::as_str),
            Some("Deterministic browser materialization fixture")
        );
        assert_eq!(
            out.pointer("/materialized_page/blocker_classification/blocker_class")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            out.pointer("/blocker_classification/blocker_class")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            out.pointer("/web_tooling_gates/summary/passed")
                .and_then(Value::as_u64),
            Some(6)
        );
        assert_eq!(
            out.pointer("/web_tooling_gates/summary/not_evaluated")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            out.pointer("/materialized_page/extraction_confidence")
                .and_then(Value::as_str),
            Some("usable")
        );
        assert_eq!(
            out.pointer("/final_url_safety/status").and_then(Value::as_str),
            Some("allowed")
        );
        assert_eq!(
            out.pointer("/cleanup_status/status").and_then(Value::as_str),
            Some("completed_noop")
        );
        assert_eq!(
            out.pointer("/artifact_quarantine/state").and_then(Value::as_str),
            Some("created_ref_only")
        );
        assert_eq!(
            out.pointer("/artifact_quarantine/raw_artifact_bytes_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/artifact_quarantine/projection_contains_raw_artifacts")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/artifact_quarantine/evidence_receives_extracted_text_only")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            out.pointer("/artifact_quarantine/raw_html_ref")
                .and_then(Value::as_str)
                .unwrap_or("")
                .ends_with("/raw-html")
        );
        assert!(
            out.pointer("/artifact_quarantine/browser_trace_ref")
                .and_then(Value::as_str)
                .unwrap_or("")
                .ends_with("/browser-trace")
        );
        assert!(
            out.pointer("/artifact_quarantine/screenshot_ref")
                .map(Value::is_null)
                .unwrap_or(false)
        );
        assert_eq!(
            out.pointer("/artifact_quarantine/console_log_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/artifact_quarantine/network_log_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/artifact_manifest/projection_contains_raw_artifacts")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/artifact_manifest/evidence_receives_extracted_text_only")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/artifact_manifest/artifacts/0/kind")
                .and_then(Value::as_str),
            Some("raw_html")
        );
        assert_eq!(
            out.pointer("/artifact_manifest/artifacts/0/raw_bytes_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/artifact_manifest/artifacts/0/workflow_trace_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/artifact_manifest/screenshot/captured")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/evidence_handoff_contract/evidence_candidate_state")
                .and_then(Value::as_str),
            Some("evidence_pack_candidate_created")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/state").and_then(Value::as_str),
            Some("evidence_pack_candidate_created")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/pack_version")
                .and_then(Value::as_str),
            Some("evidence_pack_v1")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/source_kind")
                .and_then(Value::as_str),
            Some("browser_materialized_page")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/source_class")
                .and_then(Value::as_str),
            Some("web_page")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/source_domain")
                .and_then(Value::as_str),
            Some("example.com")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/permissions")
                .and_then(Value::as_str),
            Some("public_web")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/promotion/decision")
                .and_then(Value::as_str),
            Some("candidate_ready_for_packaging")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/promotion/safety/raw_payload_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/evidence_candidate/evidence_artifacts/raw_html_ref")
                .and_then(Value::as_str)
                .map(|raw| raw.ends_with("/raw-html")),
            Some(true)
        );
        assert!(
            out.pointer("/evidence_candidate/claim_hints/0")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("deterministic rendered page fixture")
        );
        assert!(
            out.pointer("/evidence_candidate/term_hints")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| row.as_str() == Some("materialization")))
                .unwrap_or(false)
        );
        assert_eq!(
            out.pointer("/evidence_candidate/promoted_to_evidence_pack")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/evidence_pack_candidates/0/pack_version")
                .and_then(Value::as_str),
            Some("evidence_pack_v1")
        );
        assert_eq!(
            out.pointer("/evidence_refs/0/source_kind")
                .and_then(Value::as_str),
            Some("browser_materialized_page")
        );
        assert_eq!(
            out.pointer("/evidence_refs/0/locator").and_then(Value::as_str),
            Some("https://example.com/research")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/artifact_manifest_ref").is_some(),
            true
        );
        assert_eq!(
            out.pointer("/materialized_page/raw_payload_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        let encoded = serde_json::to_string(&out).expect("encode output");
        assert!(!encoded.contains("<html"));
        assert!(!encoded.contains("</html"));
        assert!(!encoded.contains("console.log"));
        assert!(!encoded.contains("networkEvents"));
        assert!(!encoded.contains("data:image/png;base64"));
    }

    #[test]
    fn browser_materialization_rejects_caller_raw_artifact_payloads() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut policy = default_policy();
        let browser_config = policy
            .pointer_mut("/web_conduit/browser_materialization")
            .expect("browser materialization policy");
        browser_config["enabled"] = json!(true);
        browser_config["adapter_ready"] = json!(true);
        browser_config["provider_order"] = json!(["fake_materialization"]);
        write_json_atomic(&policy_path(tmp.path()), &policy).expect("write policy");

        for field in [
            "raw_html",
            "rawPayload",
            "screenshot_bytes",
            "browserTrace",
            "consoleLogs",
            "networkLogs",
        ] {
            let mut request = json!({
                "url": "https://example.com/research",
                "admission_ref": "test-browser-capability"
            });
            request[field] = json!("caller supplied raw material");
            let out = api_browser_materialize_page(tmp.path(), &request);
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
                out.pointer("/artifact_quarantine/state").and_then(Value::as_str),
                Some("not_created")
            );
        }
    }

    #[test]
    fn browser_materialization_local_static_fixture_extracts_policy_owned_page() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture_dir = tmp.path().join("fixtures");
        std::fs::create_dir_all(&fixture_dir).expect("fixture dir");
        std::fs::write(
            fixture_dir.join("static.html"),
            r#"<!doctype html>
            <html>
              <head><title>Local Static Fixture Research</title></head>
              <body>
                <main>
                  <h1>Research fixture heading</h1>
                  <p>April 2026 scientific breakthrough fixture evidence with enough extracted text for synthesis packaging.</p>
                  <a href="/source">Source page</a>
                  <a href="https://example.org/more">More context</a>
                </main>
              </body>
            </html>"#,
        )
        .expect("fixture write");

        let mut policy = default_policy();
        let browser_config = policy
            .pointer_mut("/web_conduit/browser_materialization")
            .expect("browser materialization policy");
        browser_config["enabled"] = json!(true);
        browser_config["adapter_ready"] = json!(true);
        browser_config["provider_order"] = json!(["local_static_fixture"]);
        browser_config["local_static_fixture"] = json!({
            "fixture_url": "https://example.com/research",
            "fixture_rel_path": "fixtures/static.html",
            "content_type": "text/html; charset=utf-8"
        });
        write_json_atomic(&policy_path(tmp.path()), &policy).expect("write policy");

        let out = api_browser_materialize_page(
            tmp.path(),
            &json!({
                "url": "https://example.com/research",
                "admission_ref": "test-browser-capability"
            }),
        );

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("provider").and_then(Value::as_str),
            Some("local_static_fixture")
        );
        assert_eq!(
            out.get("tool_execution_attempted").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.get("browser_launch_attempted").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/materialized_page/title").and_then(Value::as_str),
            Some("Local Static Fixture Research")
        );
        assert!(
            out.pointer("/materialized_page/main_text_or_markdown")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("April 2026 scientific breakthrough fixture evidence")
        );
        assert_eq!(
            out.pointer("/materialized_page/extractor")
                .and_then(Value::as_str),
            Some("readability")
        );
        assert_eq!(
            out.pointer("/materialized_page/extraction_confidence")
                .and_then(Value::as_str),
            Some("usable")
        );
        assert_eq!(
            out.pointer("/materialized_page/content_truncated")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/materialized_page/links_summary/0/href")
                .and_then(Value::as_str),
            Some("https://example.com/source")
        );
        assert_eq!(
            out.pointer("/final_url_safety/status").and_then(Value::as_str),
            Some("allowed")
        );
        assert_eq!(
            out.pointer("/cleanup_status/status").and_then(Value::as_str),
            Some("completed")
        );
        assert_eq!(
            out.pointer("/cleanup_status/cleanup_attempted")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/local_static_fixture/fixture_path_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/local_static_fixture/raw_fixture_payload_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/evidence_candidate/source_kind")
                .and_then(Value::as_str),
            Some("browser_materialized_page")
        );
        assert!(
            out.pointer("/evidence_candidate/snippet")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("April 2026 scientific breakthrough fixture evidence")
        );
        assert_eq!(
            out.pointer("/artifact_quarantine/projection_contains_raw_artifacts")
                .and_then(Value::as_bool),
            Some(false)
        );
        let encoded = serde_json::to_string(&out).expect("encode output");
        assert!(!encoded.contains("<html"));
        assert!(!encoded.contains("<body"));
        assert!(!encoded.contains("ws://"));
        assert!(!encoded.contains("devtools"));
        assert!(!encoded.contains("static.html"));
    }

    #[test]
    fn browser_materialization_local_static_fixture_classifies_thin_content() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture_dir = tmp.path().join("fixtures");
        std::fs::create_dir_all(&fixture_dir).expect("fixture dir");
        std::fs::write(
            fixture_dir.join("thin.html"),
            r#"<!doctype html><html><head><title>Thin Fixture</title></head><body><main>Short note.</main></body></html>"#,
        )
        .expect("fixture write");

        let mut policy = default_policy();
        let browser_config = policy
            .pointer_mut("/web_conduit/browser_materialization")
            .expect("browser materialization policy");
        browser_config["enabled"] = json!(true);
        browser_config["adapter_ready"] = json!(true);
        browser_config["provider_order"] = json!(["local_static_fixture"]);
        browser_config["local_static_fixture"] = json!({
            "fixture_url": "https://example.com/thin",
            "fixture_rel_path": "fixtures/thin.html",
            "content_type": "text/html; charset=utf-8"
        });
        write_json_atomic(&policy_path(tmp.path()), &policy).expect("write policy");

        let out = api_browser_materialize_page(
            tmp.path(),
            &json!({
                "url": "https://example.com/thin",
                "admission_ref": "test-browser-capability"
            }),
        );

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/materialized_page/blocker_classification/blocker_class")
                .and_then(Value::as_str),
            Some("content_too_thin")
        );
        assert_eq!(
            out.pointer("/blocker_classification/evidence_impact")
                .and_then(Value::as_str),
            Some("low_confidence_raw")
        );
        assert_eq!(
            out.pointer("/materialized_page/extraction_confidence")
                .and_then(Value::as_str),
            Some("low_confidence_raw")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/confidence")
                .and_then(Value::as_str),
            Some("low_confidence_raw")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/promotion/decision")
                .and_then(Value::as_str),
            Some("candidate_retained_low_confidence_content_too_thin")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/promotion/components/substantive_main_text")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/web_tooling_gates/summary/soft_failed")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            out.pointer("/web_tooling_gates/summary/hard_failed")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert!(out
            .pointer("/evidence_candidate/quality_flags")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("content_too_thin")))
            .unwrap_or(false));
    }

    #[test]
    fn browser_materialization_local_static_fixture_rejects_blocker_shell_as_evidence() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture_dir = tmp.path().join("fixtures");
        std::fs::create_dir_all(&fixture_dir).expect("fixture dir");
        std::fs::write(
            fixture_dir.join("challenge.html"),
            r#"<!doctype html>
            <html>
              <head><title>Checking your browser</title></head>
              <body>
                <main>
                  <h1>Checking your browser before accessing the site</h1>
                  <p>Please complete the following challenge to verify you are human. Cloudflare bot protection detected unusual traffic and access denied until the browser challenge completes.</p>
                  <p>This page contains plenty of words but it is an anti bot or access challenge, not usable source evidence for the user's research task.</p>
                </main>
              </body>
            </html>"#,
        )
        .expect("fixture write");

        let mut policy = default_policy();
        let browser_config = policy
            .pointer_mut("/web_conduit/browser_materialization")
            .expect("browser materialization policy");
        browser_config["enabled"] = json!(true);
        browser_config["adapter_ready"] = json!(true);
        browser_config["provider_order"] = json!(["local_static_fixture"]);
        browser_config["local_static_fixture"] = json!({
            "fixture_url": "https://example.com/challenge",
            "fixture_rel_path": "fixtures/challenge.html",
            "content_type": "text/html; charset=utf-8"
        });
        write_json_atomic(&policy_path(tmp.path()), &policy).expect("write policy");

        let out = api_browser_materialize_page(
            tmp.path(),
            &json!({
                "url": "https://example.com/challenge",
                "admission_ref": "test-browser-capability"
            }),
        );

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/materialized_page/blocker_classification/blocker_class")
                .and_then(Value::as_str),
            Some("anti_bot_or_access_challenge")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/confidence")
                .and_then(Value::as_str),
            Some("rejected")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/promotion/decision")
                .and_then(Value::as_str),
            Some("rejected_blocker_shell")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/promotion/components/not_blocker_shell")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/web_tooling_gates/summary/soft_failed")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert!(out
            .pointer("/evidence_candidate/quality_flags")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("not_promotable")))
            .unwrap_or(false));
    }

    #[test]
    fn browser_materialization_local_static_fixture_cleans_up_on_failure() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut policy = default_policy();
        let browser_config = policy
            .pointer_mut("/web_conduit/browser_materialization")
            .expect("browser materialization policy");
        browser_config["enabled"] = json!(true);
        browser_config["adapter_ready"] = json!(true);
        browser_config["provider_order"] = json!(["local_static_fixture"]);
        browser_config["local_static_fixture"] = json!({
            "fixture_url": "https://example.com/research",
            "fixture_rel_path": "fixtures/missing.html",
            "content_type": "text/html; charset=utf-8"
        });
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
            out.get("provider").and_then(Value::as_str),
            Some("local_static_fixture")
        );
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("local_static_fixture_unavailable")
        );
        assert_eq!(
            out.get("tool_execution_attempted").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/cleanup_status/status").and_then(Value::as_str),
            Some("completed_after_failure")
        );
        assert_eq!(
            out.pointer("/cleanup_status/cleanup_attempted")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/retry_diagnostics/retry_recommendation")
                .and_then(Value::as_str),
            Some("fix_policy_owned_fixture_before_retry")
        );
        assert!(out.get("materialized_page").map(Value::is_null).unwrap_or(false));
        assert_eq!(
            out.pointer("/artifact_quarantine/state").and_then(Value::as_str),
            Some("not_created")
        );
        let encoded = serde_json::to_string(&out).expect("encode output");
        assert!(!encoded.contains("missing.html"));
        assert!(!encoded.contains("ws://"));
        assert!(!encoded.contains("devtools"));
    }

    #[test]
    fn browser_materialization_local_static_fixture_cannot_bypass_url_safety() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut policy = default_policy();
        let browser_config = policy
            .pointer_mut("/web_conduit/browser_materialization")
            .expect("browser materialization policy");
        browser_config["enabled"] = json!(true);
        browser_config["adapter_ready"] = json!(true);
        browser_config["provider_order"] = json!(["local_static_fixture"]);
        browser_config["local_static_fixture"] = json!({
            "fixture_url": "http://127.0.0.1/research",
            "fixture_rel_path": "fixtures/static.html",
            "content_type": "text/html; charset=utf-8"
        });
        write_json_atomic(&policy_path(tmp.path()), &policy).expect("write policy");

        let out = api_browser_materialize_page(
            tmp.path(),
            &json!({
                "url": "http://127.0.0.1/research",
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
            Some("private_network_blocked")
        );
        assert!(out.get("provider").is_none());
    }

    #[test]
    fn browser_materialization_local_js_fixture_proves_rendered_content_gap() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture_dir = tmp.path().join("fixtures");
        std::fs::create_dir_all(&fixture_dir).expect("fixture dir");
        std::fs::write(
            fixture_dir.join("js-shell.html"),
            r#"<!doctype html>
            <html>
              <head><title>Local JS Fixture Research</title></head>
              <body>
                <main>
                  <h1>JS shell</h1>
                  <div id="app">Loading...</div>
                  <script>
                    document.getElementById('app').textContent = 'Rendered JS fixture content: study result from hydrated page.';
                  </script>
                </main>
              </body>
            </html>"#,
        )
        .expect("fixture write");
        std::fs::write(
            fixture_dir.join("js-rendered.txt"),
            "Rendered JS fixture content: study result from hydrated page with enough detail for evidence packaging.",
        )
        .expect("rendered fixture write");

        let mut policy = default_policy();
        let browser_config = policy
            .pointer_mut("/web_conduit/browser_materialization")
            .expect("browser materialization policy");
        browser_config["enabled"] = json!(true);
        browser_config["adapter_ready"] = json!(true);
        browser_config["provider_order"] = json!(["local_js_rendered_fixture"]);
        browser_config["local_js_rendered_fixture"] = json!({
            "fixture_url": "https://example.com/js-research",
            "fixture_rel_path": "fixtures/js-shell.html",
            "rendered_text_rel_path": "fixtures/js-rendered.txt",
            "rendered_marker": "Rendered JS fixture content",
            "content_type": "text/html; charset=utf-8"
        });
        write_json_atomic(&policy_path(tmp.path()), &policy).expect("write policy");

        let out = api_browser_materialize_page(
            tmp.path(),
            &json!({
                "url": "https://example.com/js-research",
                "admission_ref": "test-browser-capability"
            }),
        );

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("provider").and_then(Value::as_str),
            Some("local_js_rendered_fixture")
        );
        assert_eq!(
            out.pointer("/js_render_proof/direct_fetch_contains_rendered_marker")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/js_render_proof/materialized_contains_rendered_marker")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/js_render_proof/readiness_strategy_policy_owned")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/js_render_proof/caller_supplied_script_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert!(
            out.pointer("/materialized_page/main_text_or_markdown")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("Rendered JS fixture content")
        );
        assert_eq!(
            out.pointer("/materialized_page/extractor")
                .and_then(Value::as_str),
            Some("policy_owned_js_render_fixture")
        );
        assert_eq!(
            out.pointer("/materialized_page/local_js_rendered_fixture/caller_supplied_script_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/cleanup_status/status").and_then(Value::as_str),
            Some("completed")
        );
        assert_eq!(
            out.pointer("/evidence_candidate/snippet")
                .and_then(Value::as_str)
                .map(|snippet| snippet.contains("Rendered JS fixture content")),
            Some(true)
        );
        let encoded = serde_json::to_string(&out).expect("encode output");
        assert!(!encoded.contains("document.getElementById"));
        assert!(!encoded.contains("js-shell.html"));
        assert!(!encoded.contains("js-rendered.txt"));
        assert!(!encoded.contains("ws://"));
        assert!(!encoded.contains("devtools"));
    }

    #[test]
    fn browser_materialization_rejects_caller_supplied_render_scripts() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut policy = default_policy();
        let browser_config = policy
            .pointer_mut("/web_conduit/browser_materialization")
            .expect("browser materialization policy");
        browser_config["enabled"] = json!(true);
        browser_config["adapter_ready"] = json!(true);
        browser_config["provider_order"] = json!(["local_js_rendered_fixture"]);
        browser_config["local_js_rendered_fixture"] = json!({
            "fixture_url": "https://example.com/js-research",
            "fixture_rel_path": "fixtures/js-shell.html",
            "rendered_text_rel_path": "fixtures/js-rendered.txt",
            "rendered_marker": "Rendered JS fixture content"
        });
        write_json_atomic(&policy_path(tmp.path()), &policy).expect("write policy");

        for field in ["script", "javascript", "evaluate_script", "raw_wait_script"] {
            let mut request = json!({
                "url": "https://example.com/js-research",
                "admission_ref": "test-browser-capability"
            });
            request[field] = json!("document.body.innerText = 'caller controlled'");
            let out = api_browser_materialize_page(tmp.path(), &request);
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
                out.pointer("/artifact_quarantine/state").and_then(Value::as_str),
                Some("not_created")
            );
        }
    }

    #[test]
    fn browser_materialization_blocks_unsafe_final_urls_before_artifacts() {
        for (final_url, expected_status) in [
            ("http://127.0.0.1/private", "private_network_blocked"),
            (
                "https://user:p4ssw0rd-leak@example.com/private",
                "blocked_url_credentials",
            ),
            ("file:///etc/passwd", "invalid_url"),
        ] {
            let tmp = tempfile::tempdir().expect("tempdir");
            let fixture_dir = tmp.path().join("fixtures");
            std::fs::create_dir_all(&fixture_dir).expect("fixture dir");
            std::fs::write(
                fixture_dir.join("static.html"),
                "<html><head><title>Unsafe final URL fixture</title></head><body><main>Should not be read into evidence.</main></body></html>",
            )
            .expect("fixture write");

            let mut policy = default_policy();
            let browser_config = policy
                .pointer_mut("/web_conduit/browser_materialization")
                .expect("browser materialization policy");
            browser_config["enabled"] = json!(true);
            browser_config["adapter_ready"] = json!(true);
            browser_config["provider_order"] = json!(["local_static_fixture"]);
            browser_config["local_static_fixture"] = json!({
                "fixture_url": "https://example.com/redirect-start",
                "final_url": final_url,
                "fixture_rel_path": "fixtures/static.html",
                "content_type": "text/html; charset=utf-8"
            });
            write_json_atomic(&policy_path(tmp.path()), &policy).expect("write policy");

            let out = api_browser_materialize_page(
                tmp.path(),
                &json!({
                    "url": "https://example.com/redirect-start",
                    "admission_ref": "test-browser-capability"
                }),
            );

            assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
            assert_eq!(
                out.get("error").and_then(Value::as_str),
                Some("url_safety_blocked")
            );
            assert_eq!(
                out.pointer("/final_url_safety/status")
                    .and_then(Value::as_str),
                Some(expected_status)
            );
            assert_eq!(
                out.pointer("/final_url_safety/revalidate_before_artifact_creation")
                    .and_then(Value::as_bool),
                Some(true)
            );
            assert_eq!(
                out.pointer("/blocker_classification/blocker_class")
                    .and_then(Value::as_str),
                Some("unsafe_url")
            );
            assert_eq!(
                out.pointer("/web_tooling_gates/summary/hard_failed")
                    .and_then(Value::as_u64),
                Some(3)
            );
            assert_eq!(
                out.pointer("/artifact_quarantine/state").and_then(Value::as_str),
                Some("not_created")
            );
            assert!(out.get("materialized_page").map(Value::is_null).unwrap_or(false));
            assert!(out.get("evidence_candidate").map(Value::is_null).unwrap_or(false));
            assert_eq!(
                out.pointer("/cleanup_status/status").and_then(Value::as_str),
                Some("completed_after_failure")
            );
            let encoded = serde_json::to_string(&out).expect("encode output");
            assert!(!encoded.contains("Should not be read into evidence"));
            assert!(!encoded.contains("p4ssw0rd-leak"));
            assert!(!encoded.contains("user:p4ssw0rd-leak"));
            assert!(!encoded.contains("<html"));
        }
    }
