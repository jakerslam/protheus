
fn runtime_web_family_metadata(root: &Path, policy: &Value, family: WebProviderFamily) -> Value {
    let configured_path = match family {
        WebProviderFamily::Search => "/web_conduit/search_provider_order",
        WebProviderFamily::Fetch => "/web_conduit/fetch_provider_order",
    };
    let configured_provider_input = configured_provider_input_from_policy(policy, family);
    let configured_provider = configured_provider_input
        .as_ref()
        .and_then(|raw| normalize_provider_token_for_family(raw, family));
    let selected_provider = match family {
        WebProviderFamily::Search => resolved_search_provider_chain("", &json!({}), policy)
            .first()
            .cloned(),
        WebProviderFamily::Fetch => fetch_provider_chain_from_request("", &json!({}), policy)
            .first()
            .cloned(),
    };
    let mut diagnostics = Vec::<Value>::new();
    if let Some(raw) = configured_provider_input.as_ref() {
        if configured_provider.is_none() {
            diagnostics.push(runtime_diagnostic(
                invalid_provider_code(family),
                format!(
                    "{configured_path} contains unsupported provider token \"{raw}\"; falling back to auto-detect precedence."
                ),
                configured_path,
            ));
        }
    }
    for raw in raw_provider_tokens_from_policy(policy, family) {
        if normalize_provider_token_for_family(&raw, family).is_none()
            && configured_provider_input.as_deref() != Some(raw.as_str())
        {
            diagnostics.push(runtime_diagnostic(
                invalid_provider_code(family),
                format!(
                    "{configured_path} contains unsupported provider token \"{raw}\"; falling back to auto-detect precedence."
                ),
                configured_path,
            ));
        }
    }
    let provider_source = if let Some(configured) = configured_provider.as_ref() {
        if selected_provider.as_ref() == Some(configured) {
            "configured"
        } else if selected_provider.is_some() {
            let missing_credential =
                !provider_has_runtime_credential_with(configured, family, |key| {
                    std::env::var(key).ok()
                }) && provider_descriptor(configured, family)
                    .map(|descriptor| !descriptor.env_keys.is_empty())
                    .unwrap_or(false);
            if missing_credential {
                if let Some(selected) = selected_provider.as_ref() {
                    diagnostics.push(runtime_diagnostic(
                        fallback_used_code(family),
                        format!(
                            "{configured_path} prefers \"{configured}\", but its credential is unresolved; falling back to \"{selected}\"."
                        ),
                        &configured_scope_path(configured, family),
                    ));
                } else {
                    diagnostics.push(runtime_diagnostic(
                        no_fallback_code(family),
                        format!(
                            "{configured_path} prefers \"{configured}\", but no credential-backed or keyless fallback provider is available."
                        ),
                        &configured_scope_path(configured, family),
                    ));
                }
            }
            "auto-detect"
        } else {
            "none"
        }
    } else if let Some(selected) = selected_provider.as_ref() {
        diagnostics.push(runtime_diagnostic(
            auto_detect_code(family),
            format!(
                "{} auto-detected provider \"{selected}\".",
                provider_family_name(family)
            ),
            configured_path,
        ));
        "auto-detect"
    } else {
        "none"
    };
    let selection_fallback_reason = if configured_provider_input.is_some()
        && configured_provider.is_none()
        && selected_provider.is_some()
    {
        Some("invalid_configured_provider")
    } else if configured_provider.is_some()
        && selected_provider.is_some()
        && selected_provider != configured_provider
    {
        Some("credential_unresolved")
    } else {
        None
    };
    let owner_provider = selected_provider
        .as_deref()
        .or(configured_provider.as_deref());
    let selected_provider_key_source = selected_provider_key_source(policy, owner_provider, family);
    let tool_surface_health = runtime_web_family_health(
        family,
        selected_provider.as_deref(),
        &selected_provider_key_source,
        selection_fallback_reason,
        &diagnostics,
    );
    let allow_fallback_hint = provider_source != "configured";
    let execution_gate = runtime_web_execution_gate(
        tool_surface_health
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unavailable"),
        tool_surface_health
            .get("selected_provider_ready")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        allow_fallback_hint,
        tool_surface_health
            .get("blocking_reason")
            .and_then(Value::as_str)
            .unwrap_or("none"),
    );
    json!({
        "configured_provider_input": configured_provider_input,
        "provider_configured": configured_provider,
        "provider_source": provider_source,
        "selected_provider": selected_provider,
        "selected_provider_key_source": selected_provider_key_source,
        "selection_fallback_reason": selection_fallback_reason,
        "configured_surface_path": configured_provider
            .as_deref()
            .map(|provider| configured_scope_path(provider, family)),
        "config_surface": config_surface_snapshot(policy, owner_provider, family),
        "manifest_contract_owner": manifest_contract_owner(owner_provider, family),
        "public_artifact_runtime": public_artifact_contract_for_family(family),
        "tool_surface_health": tool_surface_health,
        "execution_gate": execution_gate,
        "resolution_contract": runtime_resolution_contract(family),
        "state_path": runtime_web_tools_state_path(root).display().to_string(),
        "diagnostics": diagnostics
    })
}

fn browser_materialization_array_field(
    source: &Value,
    field: &str,
    fallback: &[&str],
) -> Value {
    source
        .get(field)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| json!(clean_text(row, 160)))
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .map(Value::Array)
        .unwrap_or_else(|| Value::Array(fallback.iter().map(|row| json!(row)).collect()))
}

fn browser_materialization_profile_compilation_contract(
    config: &Value,
    enabled: bool,
    adapter_ready: bool,
) -> Value {
    let profile_contract = config
        .get("profile_contract")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let request_contract = config
        .get("request_contract")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let denied_fields = browser_materialization_array_field(
        &request_contract,
        "denied_fields",
        &[
            "browser_args",
            "launch_args",
            "args",
            "stealthArgs",
            "extra_args",
            "_strategy_args",
            "launchOptions",
            "contextOptions",
            "context_options",
            "headless",
            "viewport",
            "userAgent",
            "user_agent",
            "timezone",
            "timezoneId",
            "locale",
            "colorScheme",
            "humanize",
            "humanPreset",
            "humanConfig",
            "geoip",
            "cdp_command",
            "user_script",
            "proxy",
            "proxy_url",
            "proxy_credentials",
            "proxyUrl",
            "proxyCredentials",
            "session_id",
            "sessionId",
            "storage_state",
            "storageState",
            "local_file",
            "localFile",
            "userDataDir",
        ],
    );
    let denied_launch_args = browser_materialization_array_field(
        &profile_contract,
        "denied_launch_args",
        &[
            "--remote-debugging-port",
            "--disable-web-security",
            "--ignore-certificate-errors",
            "--allow-file-access-from-files",
            "--load-extension",
        ],
    );
    let telemetry_fields = browser_materialization_array_field(
        &profile_contract,
        "telemetry_fields",
        &[
            "profile_ref",
            "state_scope",
            "effective_profile_hash",
            "denied_option_count",
        ],
    );
    json!({
        "version": "browser_profile_compilation_v1",
        "source_pattern": "cloakbrowser_launch_profile_compiler",
        "compile_before_adapter_launch": true,
        "status": if !enabled {
            "prepared_capability_disabled"
        } else if adapter_ready {
            "ready_for_adapter"
        } else {
            "blocked_adapter_not_ready"
        },
        "profile_source": profile_contract
            .get("profile_source")
            .and_then(Value::as_str)
            .unwrap_or("tool_cd_policy"),
        "default_profile": profile_contract
            .get("default_profile")
            .and_then(Value::as_str)
            .unwrap_or("stateless_public_materialization"),
        "effective_profile_ref": profile_contract
            .get("default_profile")
            .and_then(Value::as_str)
            .unwrap_or("stateless_public_materialization"),
        "state_scope": profile_contract
            .get("state_scope")
            .and_then(Value::as_str)
            .unwrap_or("stateless"),
        "caller_override_allowed": profile_contract
            .get("caller_override_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "normalize_or_reject_context_conflicts": profile_contract
            .get("normalize_or_reject_context_conflicts")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "context_options_allowed_from_caller": profile_contract
            .get("context_options_allowed_from_caller")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "context_conflict_fields": profile_contract
            .get("context_conflict_fields")
            .cloned()
            .unwrap_or_else(|| json!([
                "locale",
                "timezone",
                "timezoneId",
                "viewport",
                "userAgent",
                "user_agent",
                "colorScheme"
            ])),
        "close_browser_on_context_creation_failure": profile_contract
            .get("close_browser_on_context_creation_failure")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "context_close_closes_browser": profile_contract
            .get("context_close_closes_browser")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "persistent_context_allowed": profile_contract
            .get("persistent_context_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "humanized_interaction_allowed": profile_contract
            .get("humanized_interaction_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "proxy_contract": profile_contract
            .get("proxy_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_proxy_capability_contract_v1",
                "source_pattern": "cloakbrowser_proxy_url_resolution",
                "default_admitted": false,
                "separate_capability_required": true,
                "direct_request_proxy_fields_allowed": false,
                "credentials_separated_from_server": true,
                "credentials_stored_by_gateway_secret_broker": true,
                "raw_proxy_url_chat_visible": false,
                "raw_proxy_credentials_chat_visible": false,
                "scheme_normalization_allowed_after_admission": true,
                "socks_proxy_requires_adapter_arg_lane": true,
                "credential_percent_encoding_internal": true,
                "credential_encoding_notice_chat_visible": false,
                "proxy_bypass_list_policy_owned": true,
                "malformed_proxy_config_rejected_before_adapter": true
            })),
        "argument_compiler": profile_contract
            .get("argument_compiler")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_argument_compiler_contract_v1",
                "source_pattern": "cloakbrowser_build_args",
                "dedupe_key": "chromium_flag_name_before_equals",
                "priority_order": [
                    "policy_default_args",
                    "policy_profile_args",
                    "admitted_profile_fields"
                ],
                "caller_supplied_args_allowed": false,
                "dedicated_profile_fields_override_default_args": true,
                "debug_override_logs_chat_visible": false
            })),
        "proxy_capability_required": profile_contract
            .get("proxy_capability_required")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "persistent_session_capability_required": profile_contract
            .get("persistent_session_capability_required")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "separate_admission_required_for": [
            "proxy",
            "persistent_session",
            "caller_controlled_launch_args",
            "raw_cdp_commands",
            "arbitrary_user_scripts"
        ],
        "denied_caller_fields": denied_fields,
        "denied_launch_args": denied_launch_args,
        "telemetry_fields": telemetry_fields,
        "raw_launch_args_chat_visible": false,
        "raw_browser_trace_chat_visible": false
    })
}

fn runtime_browser_materialization_metadata(root: &Path, policy: &Value) -> Value {
    let config = policy
        .pointer("/web_conduit/browser_materialization")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let dependency_lifecycle =
        config
            .get("dependency_lifecycle")
            .cloned()
            .unwrap_or_else(|| {
                json!({
                    "version": "browser_dependency_lifecycle_contract_v1",
                    "source_pattern": "cloakbrowser_platform_version_cache_contract",
                    "platform_detection": "runtime_owned",
                    "platform_tag_chat_visible": false,
                    "binary_version_source": "provider_readiness_manifest",
                    "local_binary_override_allowed": false,
                    "surprise_download_allowed": false,
                    "ordinary_research_may_install_dependency": false,
                    "install_requires_explicit_operator_action": true,
                    "cache_root_policy_owned": true,
                    "cache_cleanup_tied_to_system_cleanup": true,
                    "version_marker_may_upgrade_only_if_binary_exists": true,
                    "download_install_contract": {
                        "source_pattern": "cloakbrowser_atomic_download_checksum_extract",
                        "temp_download_required": true,
                        "partial_download_cleanup_required": true,
                        "checksum_verification_required": true,
                        "checksum_skip_allowed_for_ordinary_research": false,
                        "primary_fallback_download_allowed_only_in_operator_install": true,
                        "archive_path_traversal_rejected": true,
                        "single_root_flatten_allowed": true,
                        "extract_sets_executable_permissions": true
                    },
                    "update_contract": {
                        "source_pattern": "cloakbrowser_rate_limited_background_update",
                        "background_update_during_ordinary_research_allowed": false,
                        "rate_limited_update_checks_required": true,
                        "update_marker_write_atomic_required": true,
                        "wrapper_update_notice_chat_visible": false
                    },
                    "raw_binary_path_chat_visible": false,
                    "download_url_chat_visible": false,
                    "unsupported_platform_status": "dependency_unavailable"
                })
            });
    let enabled = config
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let selected_provider = config
        .get("provider_order")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 80))
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| "local_browser".to_string());
    let adapter_ready = enabled
        && config
            .get("adapter_ready")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let status = if !enabled {
        "unavailable"
    } else if adapter_ready {
        "ready"
    } else {
        "degraded"
    };
    let blocking_reason = if !enabled {
        "capability_not_enabled"
    } else if adapter_ready {
        "none"
    } else {
        "adapter_not_ready"
    };
    let execution_gate = if !enabled {
        json!({
            "should_execute": false,
            "mode": "blocked",
            "reason": "capability_not_enabled",
            "retry_recommended": false,
            "retry_lane": "none"
        })
    } else {
        runtime_web_execution_gate(status, adapter_ready, false, blocking_reason)
    };
    let diagnostics = if enabled && !adapter_ready {
        vec![runtime_diagnostic(
            "WEB_BROWSER_MATERIALIZATION_ADAPTER_NOT_READY",
            "browser materialization is policy-enabled but no admitted browser adapter is ready; search/fetch providers remain the default path.".to_string(),
            "/web_conduit/browser_materialization",
        )]
    } else {
        Vec::new()
    };
    json!({
        "enabled": enabled,
        "selected_provider": selected_provider,
        "provider_source": "policy",
        "permission_class": config
            .get("permission_class")
            .and_then(Value::as_str)
            .unwrap_or("public_web_dynamic_read"),
        "requires_explicit_admission": config
            .get("requires_explicit_admission")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "tool_surface_health": {
            "status": status,
            "selected_provider_ready": adapter_ready,
            "selected_provider_requires_credential": false,
            "selected_provider_credential_state": "not_required",
            "blocking_reason": blocking_reason,
            "available_provider_count": 1,
            "diagnostic_count": diagnostics.len()
        },
        "execution_gate": execution_gate,
        "capability_contract": {
            "capability_id": "browser_materialize_page",
            "optional_capability": true,
            "chat_visibility": "hidden_until_synthesized",
            "security": config.get("security").cloned().unwrap_or_else(|| json!({})),
            "output_contract": config.get("output_contract").cloned().unwrap_or_else(|| json!({})),
            "request_contract": config.get("request_contract").cloned().unwrap_or_else(|| json!({})),
            "profile_contract": config.get("profile_contract").cloned().unwrap_or_else(|| json!({})),
            "evidence_handoff": config.get("evidence_handoff").cloned().unwrap_or_else(|| json!({})),
            "readiness_lifecycle": {
                "state": if !enabled {
                    "not_configured"
                } else if adapter_ready {
                    "ready"
                } else {
                    "not_installed"
                },
                "ordinary_research_may_install_dependency": false,
                "cleanup_tied_to_system_cleanup": true,
                "dependency_lifecycle": dependency_lifecycle
            },
            "state_path": runtime_web_tools_state_path(root).display().to_string()
        },
        "profile_compilation": browser_materialization_profile_compilation_contract(
            &config,
            enabled,
            adapter_ready,
        ),
        "diagnostics": diagnostics
    })
}

pub(crate) fn runtime_web_tools_snapshot(root: &Path, policy: &Value) -> Value {
    let search = runtime_web_family_metadata(root, policy, WebProviderFamily::Search);
    let fetch = runtime_web_family_metadata(root, policy, WebProviderFamily::Fetch);
    let browser_materialization = runtime_browser_materialization_metadata(root, policy);
    let image_tool = image_tool_runtime_resolution_snapshot(root, policy, &json!({}));
    let search_status = search
        .pointer("/tool_surface_health/status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let fetch_status = fetch
        .pointer("/tool_surface_health/status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let search_ready = search
        .pointer("/tool_surface_health/selected_provider_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let fetch_ready = fetch
        .pointer("/tool_surface_health/selected_provider_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let browser_materialization_status = browser_materialization
        .pointer("/tool_surface_health/status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let browser_materialization_ready = browser_materialization
        .pointer("/tool_surface_health/selected_provider_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let browser_materialization_enabled = browser_materialization
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let search_execution_gate = search
        .get("execution_gate")
        .cloned()
        .unwrap_or_else(default_runtime_web_execution_gate);
    let fetch_execution_gate = fetch
        .get("execution_gate")
        .cloned()
        .unwrap_or_else(default_runtime_web_execution_gate);
    let browser_materialization_execution_gate = browser_materialization
        .get("execution_gate")
        .cloned()
        .unwrap_or_else(default_runtime_web_execution_gate);
    let overall_should_execute = search_execution_gate
        .get("should_execute")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || fetch_execution_gate
            .get("should_execute")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || browser_materialization_execution_gate
            .get("should_execute")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let overall_status = if search_status == "unavailable" || fetch_status == "unavailable" {
        "unavailable"
    } else if browser_materialization_enabled && browser_materialization_status == "unavailable" {
        "unavailable"
    } else if search_status == "degraded" || fetch_status == "degraded" {
        "degraded"
    } else if browser_materialization_enabled && browser_materialization_status == "degraded" {
        "degraded"
    } else {
        "ready"
    };
    let diagnostics = search
        .get("diagnostics")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .cloned()
        .chain(
            fetch
                .get("diagnostics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .cloned(),
        )
        .chain(
            browser_materialization
                .get("diagnostics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .cloned(),
        )
        .chain(
            image_tool
                .get("diagnostics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .cloned(),
        )
        .collect::<Vec<_>>();
    let metadata = json!({
        "search": search,
        "fetch": fetch,
        "browser_materialization": browser_materialization,
        "image_tool": image_tool,
        "openclaw_web_tools_contract": {
            "exports": runtime_web_tools_exports_contract(),
            "default_enablement": runtime_web_tools_default_enablement_contract(),
            "fetch_unit_test_harness": runtime_web_fetch_unit_test_harness_contract()
        },
        "tool_surface_health": {
            "status": overall_status,
            "search_status": search_status,
            "fetch_status": fetch_status,
            "browser_materialization_status": browser_materialization_status,
            "search_ready": search_ready,
            "fetch_ready": fetch_ready,
            "browser_materialization_ready": browser_materialization_ready
        },
        "tool_execution_gate": {
            "search": search_execution_gate,
            "fetch": fetch_execution_gate,
            "browser_materialization": browser_materialization_execution_gate,
            "overall_should_execute": overall_should_execute,
            "overall_mode": if overall_should_execute { "allow_any" } else { "blocked_all" }
        },
        "diagnostics": diagnostics
    });
    store_active_runtime_web_tools_metadata(root, &metadata);
    metadata
}
