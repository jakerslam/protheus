fn fetch_runtime_diagnostic_code_contract() -> Value {
    json!([
        "WEB_FETCH_PROVIDER_INVALID_AUTODETECT",
        "WEB_FETCH_AUTODETECT_SELECTED",
        "WEB_FETCH_PROVIDER_KEY_UNRESOLVED_FALLBACK_USED",
        "WEB_FETCH_PROVIDER_KEY_UNRESOLVED_NO_FALLBACK"
    ])
}

fn fetch_web_provider_snapshot_cache_contract() -> Value {
    json!({
        "cache_key_builder": "buildWebProviderSnapshotCacheKey",
        "cache_owner_scope": "OpenClawConfig+NodeJS.ProcessEnv",
        "cache_enable_predicate": "activate!=true && cache!=true && shouldUsePluginSnapshotCache(env)",
        "snapshot_cache_ttl_resolver": "resolvePluginSnapshotCacheTtlMs",
        "runtime_registry_fast_path": "resolveRuntimePluginRegistry",
        "in_flight_registry_behavior": "returns_empty_provider_set",
        "in_flight_registry_load_guard": "does_not_force_fresh_snapshot_load",
        "active_registry_compatibility_fast_path": true,
        "active_registry_workspace_inheritance": true,
        "workspace_change_invalidation": true,
        "cache_key_dimensions": ["config", "env", "workspace_dir", "candidate_plugin_ids"]
    })
}

fn fetch_runtime_provider_type_contract() -> Value {
    json!({
        "provider_context_type": "WebFetchProviderContext",
        "runtime_metadata_context_type": "WebFetchRuntimeMetadataContext",
        "provider_plugin_type": "WebFetchProviderPlugin",
        "provider_entry_type": "PluginWebFetchProviderEntry",
        "tool_definition_type": "WebFetchProviderToolDefinition",
        "credential_resolution_sources": ["config", "secretRef", "env", "missing", "not_required"]
    })
}

fn fetch_credential_presence_contract() -> Value {
    json!({
        "resolver": "hasConfiguredWebFetchCredential",
        "provider_set_resolver": "resolvePluginWebFetchProviders",
        "configured_credential_probe": "provider.getConfiguredCredentialValue || provider.getCredentialValue(fetchConfig)",
        "fallback_env_probe": "provider.envVars",
        "truthy_semantics": "non_empty_string_or_non_null"
    })
}

fn fetch_provider_contract_suite_contract() -> Value {
    json!({
        "registry_source": "pluginRegistrationContractRegistry",
        "registry_plugin_filter": "entry.webFetchProviderIds.length > 0",
        "registry_entry_resolver": "resolveWebFetchProviderContractEntriesForPluginId",
        "provider_id_source": "entry.webFetchProviderIds",
        "provider_lookup_contract": "entry.provider.id == providerId",
        "base_provider_contract": {
            "provider_id_regex": "^[a-z0-9][a-z0-9-]*$",
            "required_non_empty_fields": ["label", "hint", "placeholder", "credentialPath"],
            "signup_url_scheme": "https",
            "docs_url_scheme_if_present": "http_or_https",
            "env_vars_unique_and_non_empty": true,
            "inactive_secret_paths_include_credential_path_when_present": true
        },
        "credential_roundtrip_contract": {
            "setter": "provider.setCredentialValue(fetchConfigTarget, credentialValue)",
            "getter": "provider.getCredentialValue(fetchConfigTarget)",
            "configured_roundtrip_optional": "provider.setConfiguredCredentialValue/getConfiguredCredentialValue",
            "apply_selection_config_enables_plugin_entry": "provider.applySelectionConfig(config).plugins.entries[pluginId].enabled == true"
        },
        "tool_definition_contract": {
            "factory": "provider.createTool({ config, fetchConfig })",
            "description_non_empty": true,
            "parameters_object_required": true,
            "execute_function_required": true
        }
    })
}

fn fetch_provider_discovery_runtime_contract() -> Value {
    json!({
        "runtime_module": "src/plugins/provider-discovery.runtime.ts",
        "discovered_plugin_id_resolver": "resolveDiscoveredProviderPluginIds",
        "manifest_registry_loader": "loadPluginManifestRegistry",
        "entry_fast_path_resolver": "resolveProviderDiscoveryEntryPlugins",
        "entry_fast_path_module_normalizer": "normalizeDiscoveryModule",
        "entry_fast_path_source_loader": "createPluginSourceLoader",
        "entry_fast_path_manifest_field": "providerDiscoverySource",
        "entry_fast_path_optional": true,
        "fallback_provider_runtime_resolver": "resolvePluginProviders",
        "fallback_bundled_allowlist_compat": true
    })
}

fn fetch_provider_discovery_contract_suite_contract() -> Value {
    json!({
        "contract_test_file": "src/plugins/contracts/provider-discovery.contract.test.ts",
        "helper_module": "test/helpers/plugins/provider-discovery-contract.ts",
        "contract_targets": [
            "cloudflare-ai-gateway",
            "github-copilot",
            "minimax",
            "modelstudio",
            "ollama",
            "sglang",
            "vllm"
        ],
        "contract_invocation_pattern": "describe<Provider>ProviderDiscoveryContract()",
        "catalog_entrypoint": "runProviderCatalog"
    })
}

fn fetch_web_provider_helper_contract() -> Value {
    json!({
        "helper_module": "test/helpers/plugins/web-fetch-provider-contract.ts",
        "registry_source": "pluginRegistrationContractRegistry",
        "provider_entry_resolver": "resolveWebFetchProviderContractEntriesForPluginId",
        "provider_id_source": "entry.webFetchProviderIds",
        "contract_suite_installer": "installWebFetchProviderContractSuite",
        "missing_provider_entry_failure_mode": "web fetch provider contract entry missing"
    })
}

fn fetch_runtime_web_channel_plugin_contract() -> Value {
    json!({
        "runtime_module": "src/plugins/runtime/runtime-web-channel-plugin.ts",
        "runtime_record_resolver": "resolvePluginRuntimeRecordByEntryBaseNames",
        "runtime_module_path_resolver": "resolvePluginRuntimeModulePath",
        "plugin_boundary_loader": "loadPluginBoundaryModuleWithJiti",
        "entry_base_names": ["light-runtime-api", "runtime-api"],
        "light_runtime_cache_behavior": "cache_by_module_path",
        "heavy_runtime_cache_behavior": "cache_by_module_path",
        "missing_export_behavior": "throws_web_channel_plugin_runtime_missing_export_error"
    })
}

fn fetch_provider_runtime_core_contract() -> Value {
    json!({
        "core_runtime_contract_targets": [
            "src/plugins/contracts/tts.provider-runtime.contract.test.ts",
            "src/plugins/capability-provider-runtime.test.ts",
            "src/plugins/capability-provider-runtime.ts",
            "src/plugins/memory-embedding-provider-runtime.test.ts",
            "src/plugins/memory-embedding-provider-runtime.ts"
        ],
        "capability_runtime_entrypoints": [
            "resolveCapabilityProviderRuntime",
            "resolveMemoryEmbeddingProviderRuntime"
        ],
        "runtime_invariants": [
            "provider_runtime_resolution_contract",
            "credential_presence_contract",
            "runtime_model_catalog_contract"
        ]
    })
}

fn fetch_visibility_sanitization_contract() -> Value {
    json!({
        "sanitizer_entrypoint": "sanitizeHtml",
        "strip_invisible_unicode_entrypoint": "stripInvisibleUnicode",
        "always_remove_tags": ["meta", "template", "svg", "canvas", "iframe", "object", "embed"],
        "hidden_class_name_contract": ["sr-only", "visually-hidden", "d-none", "hidden", "invisible", "screen-reader-only", "offscreen"],
        "hidden_class_token_boundary_match_required": true,
        "hidden_style_pattern_contract": [
            "display:none",
            "visibility:hidden",
            "opacity:0",
            "font-size:0",
            "text-indent:-1000px+",
            "transparent_color",
            "clip_path_inset_percent",
            "transform_scale_0_or_far_negative_translate",
            "width0_height0_overflow_hidden"
        ],
        "hidden_attr_contract": ["aria-hidden=true", "hidden_attribute", "input[type=hidden]"],
        "comment_stripping": true,
        "invisible_unicode_codepoint_contract": [
            "U+200B",
            "U+200C",
            "U+200D",
            "U+200E",
            "U+200F",
            "U+202A-U+202E",
            "U+2060",
            "U+FEFF"
        ]
    })
}

fn fetch_shared_runtime_contract() -> Value {
    json!({
        "timeout_default_seconds": 30,
        "cache_ttl_default_minutes": 15,
        "cache_default_max_entries": 100,
        "timeout_resolver": "resolveTimeoutSeconds",
        "cache_ttl_resolver": "resolveCacheTtlMs",
        "cache_key_normalizer": "normalizeCacheKey",
        "cache_read_helper": "readCache",
        "cache_write_helper": "writeCache",
        "response_reader": "readResponseText",
        "timeout_signal_wrapper": "withTimeout"
    })
}

fn fetch_content_extraction_contract() -> Value {
    json!({
        "extract_modes": ["markdown", "text"],
        "html_to_markdown_entrypoint": "htmlToMarkdown",
        "markdown_to_text_entrypoint": "markdownToText",
        "readable_extraction_entrypoint": "extractReadableContent",
        "basic_extraction_entrypoint": "extractBasicHtmlContent",
        "truncate_entrypoint": "truncateText",
        "max_chars_enforced_after_wrapping": true,
        "extract_readable_title_required": true,
        "extract_readable_mode_parity": ["text", "markdown"],
        "readability_html_char_guard": 1_000_000,
        "readability_depth_guard": 3_000,
        "visibility_sanitization_required": true,
        "invisible_unicode_stripping_required": true
    })
}

fn fetch_provider_fallback_contract() -> Value {
    json!({
        "fallback_trigger_contract": [
            "direct_fetch_network_failure",
            "direct_fetch_http_failure",
            "readability_no_content"
        ],
        "provider_fallback_payload_rewrap_required": true,
        "provider_fallback_payload_truncation_required": true,
        "fallback_error_surface_contract": "provider_fallback_error_propagated",
        "safe_final_url_contract": "unsafe_provider_final_url_replaced_with_requested_url"
    })
}

fn fetch_ssrf_guard_contract() -> Value {
    json!({
        "blocked_before_fetch_contract": [
            "localhost_hostname",
            "private_ip_literal",
            "dns_resolves_private_ip"
        ],
        "redirect_target_revalidation_required": true,
        "rfc2544_benchmark_range_default": "deny",
        "rfc2544_benchmark_range_opt_in_flag": "ssrfPolicy.allowRfc2544BenchmarkRange",
        "public_host_allow_path": true,
        "proxy_dns_pinning_required": true
    })
}

fn fetch_response_and_wrapping_contract() -> Value {
    json!({
        "external_content_wrapper_required": true,
        "external_content_wrapper_marker_regex": "<<<EXTERNAL_UNTRUSTED_CONTENT id=\\\"[a-f0-9]{16}\\\">>>",
        "external_content_source_label": "web_fetch",
        "content_type_not_wrapped": true,
        "response_bytes_cap_enforced": true,
        "response_stream_truncation_warning_contains": "Response body truncated",
        "html_error_stripping_required": true,
        "html_error_message_max_chars": 5000
    })
}

fn fetch_cf_markdown_contract() -> Value {
    json!({
        "accept_header_preference": "text/markdown, text/html;q=0.9, */*;q=0.1",
        "markdown_content_type_contract": "text/markdown",
        "markdown_extractor_id": "cf-markdown",
        "markdown_text_mode_conversion_required": true,
        "html_fallback_extractor": "readability",
        "runtime_firecrawl_inactive_bypasses_provider_network_call": true,
        "markdown_tokens_header": "x-markdown-tokens",
        "markdown_tokens_logging_requires_url_redaction": true
    })
}

fn fetch_guarded_endpoint_contract() -> Value {
    json!({
        "strict_endpoint_wrapper": "withStrictWebToolsEndpoint",
        "trusted_endpoint_wrapper": "withTrustedWebToolsEndpoint",
        "strict_mode": "strict",
        "trusted_mode": "trusted_env_proxy",
        "trusted_policy": {
            "dangerously_allow_private_network": true,
            "allow_rfc2544_benchmark_range": true
        },
        "strict_policy_override": "none"
    })
}

fn fetch_runtime_provider_sort_contract() -> Value {
    json!({
        "alphabetical_sorter": "sortPluginProviders",
        "auto_detect_sorter": "sortPluginProvidersForAutoDetect",
        "registry_mapper": "mapRegistryProviders",
        "shared_sort_entrypoint": "sortWebFetchProviders",
        "shared_autodetect_sort_entrypoint": "sortWebFetchProvidersForAutoDetect"
    })
}

fn fetch_runtime_candidate_plugin_contract() -> Value {
    json!({
        "candidate_plugin_id_resolver": "resolveManifestDeclaredWebProviderCandidatePluginIds",
        "contract": "webFetchProviders",
        "config_key": "webFetch",
        "public_artifact_explicit_resolver": "resolveBundledExplicitWebFetchProvidersFromPublicArtifacts",
        "manifest_declared_provider_fallback": "pluginManifestDeclaresProviderConfig"
    })
}

fn fetch_public_artifact_resolution_contract() -> Value {
    json!({
        "bundled_resolution_config_resolver": "resolveBundledWebFetchResolutionConfig",
        "bundled_candidate_plugin_id_resolver": "resolveBundledCandidatePluginIds",
        "explicit_fast_path_resolver": "resolveBundledExplicitWebFetchProvidersFromPublicArtifacts",
        "manifest_records_fallback_resolver": "resolveBundledManifestRecordsByPluginId",
        "root_dir_loader": "loadBundledWebFetchProviderEntriesFromDir(path.basename(record.rootDir))",
        "fast_path_skips_manifest_scans_when_only_plugin_ids": true,
        "requires_public_artifact_for_each_bundled_manifest_contract_provider": true
    })
}

fn fetch_runtime_resolution_contract() -> Value {
    json!({
        "origin": "openclaw_runtime_web_tools_contract",
        "fallback_runtime_resolver": "resolvePluginWebFetchProviders",
        "runtime_registry_resolver": "resolveRuntimeWebFetchProviders",
        "loader_mode_contract": ["runtime", "setup"],
        "loader_activation_flags": ["activate", "cache"],
        "public_artifact_runtime_resolver": "resolveBundledWebFetchProvidersFromPublicArtifacts",
        "manifest_contract_owner_resolver": "resolveManifestContractOwnerPluginId",
        "diagnostic_code_contract": fetch_runtime_diagnostic_code_contract(),
        "provider_type_contract": fetch_runtime_provider_type_contract(),
        "credential_presence_contract": fetch_credential_presence_contract(),
        "provider_contract_suite_contract": fetch_provider_contract_suite_contract(),
        "provider_discovery_runtime_contract": fetch_provider_discovery_runtime_contract(),
        "provider_discovery_contract_suite_contract": fetch_provider_discovery_contract_suite_contract(),
        "provider_helper_contract": fetch_web_provider_helper_contract(),
        "runtime_web_channel_plugin_contract": fetch_runtime_web_channel_plugin_contract(),
        "provider_runtime_core_contract": fetch_provider_runtime_core_contract(),
        "visibility_sanitization_contract": fetch_visibility_sanitization_contract(),
        "shared_runtime_contract": fetch_shared_runtime_contract(),
        "content_extraction_contract": fetch_content_extraction_contract(),
        "provider_fallback_contract": fetch_provider_fallback_contract(),
        "ssrf_guard_contract": fetch_ssrf_guard_contract(),
        "response_and_wrapping_contract": fetch_response_and_wrapping_contract(),
        "cf_markdown_contract": fetch_cf_markdown_contract(),
        "guarded_endpoint_contract": fetch_guarded_endpoint_contract(),
        "provider_sort_contract": fetch_runtime_provider_sort_contract(),
        "candidate_plugin_contract": fetch_runtime_candidate_plugin_contract(),
        "public_artifact_resolution_contract": fetch_public_artifact_resolution_contract(),
        "snapshot_cache_contract": fetch_web_provider_snapshot_cache_contract()
    })
}

pub(crate) fn fetch_provider_resolution_snapshot(
    root: &Path,
    policy: &Value,
    request: &Value,
    provider_hint: &str,
) -> Value {
    let mut runtime = runtime_web_family_metadata(root, policy, WebProviderFamily::Fetch);
    let requested_provider_hint = clean_text(provider_hint, 60).to_ascii_lowercase();
    let request_provider_chain = request_provider_chain_for_family(request, WebProviderFamily::Fetch);
    let runtime_selected_provider =
        runtime_selected_provider_from_request(request, WebProviderFamily::Fetch);
    let prefer_runtime_provider =
        request_prefers_runtime_provider(request) || runtime_selected_provider.is_some();
    let provider_chain = fetch_provider_chain_from_request(provider_hint, request, policy);
    let selected_provider = provider_chain
        .first()
        .cloned()
        .unwrap_or_else(|| "direct_http".to_string());
    let selection_scope = if requested_provider_hint != "auto"
        && normalize_provider_token_for_family(&requested_provider_hint, WebProviderFamily::Fetch)
            .is_some()
    {
        "request_provider_hint"
    } else if runtime_selected_provider
        .as_deref()
        .map(|provider| provider == selected_provider.as_str())
        .unwrap_or(false)
    {
        "runtime_metadata"
    } else if !request_provider_chain.is_empty() {
        "request_provider_chain"
    } else if runtime
        .get("provider_source")
        .and_then(Value::as_str)
        .unwrap_or("none")
        == "configured"
    {
        "policy_configured"
    } else if provider_chain.is_empty() {
        "none"
    } else {
        "auto-detect"
    };
    if let Some(obj) = runtime.as_object_mut() {
        obj.insert("requested_provider_hint".to_string(), json!(requested_provider_hint));
        obj.insert("request_provider_chain".to_string(), json!(request_provider_chain));
        obj.insert("provider_chain".to_string(), json!(provider_chain));
        obj.insert("selected_provider".to_string(), json!(selected_provider));
        obj.insert(
            "runtime_selected_provider".to_string(),
            runtime_selected_provider.map(Value::String).unwrap_or(Value::Null),
        );
        obj.insert(
            "runtime_provider_preferred".to_string(),
            json!(prefer_runtime_provider),
        );
        obj.insert("selection_scope".to_string(), json!(selection_scope));
        obj.insert(
            "openclaw_runtime_contract".to_string(),
            fetch_runtime_resolution_contract(),
        );
    }
    runtime
}
