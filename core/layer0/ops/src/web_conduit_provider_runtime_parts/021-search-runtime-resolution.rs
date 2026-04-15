fn search_provider_hint_is_explicit(provider_hint: &str) -> bool {
    let normalized = clean_text(provider_hint, 60).to_ascii_lowercase();
    !normalized.is_empty()
        && normalized != "auto"
        && normalize_provider_token_for_family(&normalized, WebProviderFamily::Search).is_some()
}

fn reorder_search_providers_by_credential_availability(rows: Vec<String>) -> Vec<String> {
    let mut credential_ready = Vec::<String>::new();
    let mut missing_credential = Vec::<String>::new();
    for provider in rows {
        if provider_has_runtime_credential_with(&provider, WebProviderFamily::Search, |key| {
            std::env::var(key).ok()
        }) {
            credential_ready.push(provider);
        } else {
            missing_credential.push(provider);
        }
    }
    credential_ready.extend(missing_credential);
    credential_ready
}

fn search_runtime_diagnostic_code_contract() -> Value {
    json!([
        "WEB_SEARCH_PROVIDER_INVALID_AUTODETECT",
        "WEB_SEARCH_AUTODETECT_SELECTED",
        "WEB_SEARCH_KEY_UNRESOLVED_FALLBACK_USED",
        "WEB_SEARCH_KEY_UNRESOLVED_NO_FALLBACK"
    ])
}

fn search_web_provider_snapshot_cache_contract() -> Value {
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

fn search_runtime_provider_type_contract() -> Value {
    json!({
        "provider_context_type": "WebSearchProviderContext",
        "runtime_metadata_context_type": "WebSearchRuntimeMetadataContext",
        "provider_plugin_type": "WebSearchProviderPlugin",
        "provider_entry_type": "PluginWebSearchProviderEntry",
        "tool_definition_type": "WebSearchProviderToolDefinition",
        "credential_resolution_sources": ["config", "secretRef", "env", "missing"]
    })
}

fn search_credential_presence_contract() -> Value {
    json!({
        "resolver": "hasConfiguredWebSearchCredential",
        "provider_set_resolver": "resolvePluginWebSearchProviders",
        "configured_credential_probe": "provider.getConfiguredCredentialValue || provider.getCredentialValue(searchConfig)",
        "fallback_env_probe": "provider.envVars",
        "origin_filter_supported": true,
        "truthy_semantics": "non_empty_string_or_non_null"
    })
}

fn search_provider_config_contract() -> Value {
    json!({
        "forced_provider_wrapper": "withForcedProvider",
        "search_config_resolver": "resolveSearchConfig",
        "provider_plugin_config_resolver": "resolveProviderWebSearchPluginConfig",
        "provider_plugin_config_mutator": "setProviderWebSearchPluginConfigValue",
        "scoped_config_merger": "mergeScopedSearchConfig",
        "top_level_credential_accessors": ["getTopLevelCredentialValue", "setTopLevelCredentialValue"],
        "scoped_credential_accessors": ["getScopedCredentialValue", "setScopedCredentialValue"],
        "search_enabled_resolver": "resolveSearchEnabled",
        "mirror_api_key_to_top_level_supported": true
    })
}

fn search_provider_credential_resolution_contract() -> Value {
    json!({
        "resolver": "resolveWebSearchProviderCredential",
        "resolution_order": [
            "config_inline_value",
            "config_secret_ref_env_value",
            "fallback_env_vars"
        ],
        "secret_input_normalizers": ["normalizeSecretInputString", "normalizeSecretInput"],
        "secret_ref_resolver": "resolveSecretInputRef",
        "empty_or_whitespace_credentials_rejected": true
    })
}

fn search_provider_common_runtime_contract() -> Value {
    json!({
        "default_search_count": 5,
        "max_search_count": 10,
        "timeout_default_seconds": 30,
        "cache_ttl_default_minutes": 15,
        "cache_default_max_entries": 100,
        "timeout_resolver": "resolveSearchTimeoutSeconds",
        "cache_ttl_resolver": "resolveSearchCacheTtlMs",
        "shared_timeout_resolver": "resolveTimeoutSeconds",
        "shared_cache_ttl_resolver": "resolveCacheTtlMs",
        "cache_key_normalizer": "normalizeCacheKey",
        "cache_read_helper": "readCache",
        "cache_write_helper": "writeCache",
        "response_reader": "readResponseText",
        "timeout_signal_wrapper": "withTimeout",
        "count_clamper": "resolveSearchCount",
        "date_range_parser": "parseIsoDateRange",
        "trusted_endpoint_wrapper": "withTrustedWebSearchEndpoint",
        "trusted_json_post_wrapper": "postTrustedWebToolsJson",
        "site_name_resolver": "resolveSiteName",
        "module_local_cache_required": true,
        "global_symbol_cache_forbidden": "openclaw.web-search.cache",
        "freshness_normalization_supported": true,
        "freshness_cross_provider_mapping_supported": {
            "brave_shortcuts": ["pd", "pw"],
            "perplexity_values": ["day", "week"]
        },
        "date_range_contract": {
            "iso_date_normalizer": "normalizeToIsoDate",
            "perplexity_date_converter": "isoToPerplexityDate",
            "brave_date_range_validator": "YYYY-MM-DDtoYYYY-MM-DD"
        },
        "unsupported_filter_contract": {
            "builder": "buildUnsupportedSearchFilterResponse",
            "date_filter_error_code": "unsupported_date_filter",
            "non_date_filter_error_prefix": "unsupported_"
        }
    })
}

fn search_citation_redirect_contract() -> Value {
    json!({
        "resolver_entrypoint": "resolveCitationRedirectUrl",
        "transport_wrapper": "withStrictWebToolsEndpoint",
        "request_method": "HEAD",
        "timeout_ms": 5000,
        "failure_fallback": "returns_original_url"
    })
}

fn search_redirect_hardening_contract() -> Value {
    json!({
        "guarded_head_resolution_required": true,
        "guarded_endpoint_entrypoint": "withStrictWebToolsEndpoint",
        "final_url_resolution_field": "finalUrl",
        "resolved_url_fallback": "original_url",
        "failure_mode_contract": "never_throws_returns_original_url",
        "ssrf_guard_applies_to_redirect_resolution": true
    })
}

fn search_runtime_provider_sort_contract() -> Value {
    json!({
        "alphabetical_sorter": "sortPluginProviders",
        "auto_detect_sorter": "sortPluginProvidersForAutoDetect",
        "registry_mapper": "mapRegistryProviders",
        "shared_sort_entrypoint": "sortWebSearchProviders",
        "shared_autodetect_sort_entrypoint": "sortWebSearchProvidersForAutoDetect"
    })
}

fn search_runtime_candidate_plugin_contract() -> Value {
    json!({
        "candidate_plugin_id_resolver": "resolveManifestDeclaredWebProviderCandidatePluginIds",
        "contract": "webSearchProviders",
        "config_key": "webSearch",
        "public_artifact_explicit_resolver": "resolveBundledExplicitWebSearchProvidersFromPublicArtifacts",
        "manifest_declared_provider_fallback": "pluginManifestDeclaresProviderConfig"
    })
}

fn search_public_artifact_resolution_contract() -> Value {
    json!({
        "bundled_resolution_config_resolver": "resolveBundledWebSearchResolutionConfig",
        "bundled_candidate_plugin_id_resolver": "resolveBundledCandidatePluginIds",
        "explicit_fast_path_resolver": "resolveBundledExplicitWebSearchProvidersFromPublicArtifacts",
        "manifest_records_fallback_resolver": "resolveBundledManifestRecordsByPluginId",
        "root_dir_loader": "loadBundledWebSearchProviderEntriesFromDir(path.basename(record.rootDir))",
        "fast_path_skips_manifest_scans_when_only_plugin_ids": true,
        "requires_public_artifact_for_each_bundled_manifest_contract_provider": true
    })
}

fn search_runtime_resolution_contract() -> Value {
    json!({
        "origin": "openclaw_runtime_web_tools_contract",
        "fallback_runtime_resolver": "resolvePluginWebSearchProviders",
        "runtime_registry_resolver": "resolveRuntimeWebSearchProviders",
        "loader_mode_contract": ["runtime", "setup"],
        "loader_activation_flags": ["activate", "cache"],
        "public_artifact_runtime_resolver": "resolveBundledWebSearchProvidersFromPublicArtifacts",
        "manifest_contract_owner_resolver": "resolveManifestContractOwnerPluginId",
        "diagnostic_code_contract": search_runtime_diagnostic_code_contract(),
        "provider_type_contract": search_runtime_provider_type_contract(),
        "credential_presence_contract": search_credential_presence_contract(),
        "provider_config_contract": search_provider_config_contract(),
        "provider_credential_resolution_contract": search_provider_credential_resolution_contract(),
        "provider_common_runtime_contract": search_provider_common_runtime_contract(),
        "citation_redirect_contract": search_citation_redirect_contract(),
        "redirect_hardening_contract": search_redirect_hardening_contract(),
        "provider_sort_contract": search_runtime_provider_sort_contract(),
        "candidate_plugin_contract": search_runtime_candidate_plugin_contract(),
        "public_artifact_resolution_contract": search_public_artifact_resolution_contract(),
        "snapshot_cache_contract": search_web_provider_snapshot_cache_contract()
    })
}

pub(crate) fn resolved_search_provider_chain(
    provider_hint: &str,
    request: &Value,
    policy: &Value,
) -> Vec<String> {
    let request_chain = request_provider_chain_for_family(request, WebProviderFamily::Search);
    let runtime_selected_provider =
        runtime_selected_provider_from_request(request, WebProviderFamily::Search);
    let prefer_runtime_provider =
        request_prefers_runtime_provider(request) || runtime_selected_provider.is_some();
    let base = provider_chain_from_request(provider_hint, request, policy);
    if base.is_empty()
        || search_provider_hint_is_explicit(provider_hint)
        || (!request_chain.is_empty() && !prefer_runtime_provider)
    {
        return base;
    }
    let mut preferred_providers = Vec::<String>::new();
    if let Some(runtime_provider) = runtime_selected_provider {
        preferred_providers.push(runtime_provider);
    }
    if !preferred_providers.is_empty() {
        let mut merged = preferred_providers;
        merged.extend(base);
        return reorder_search_providers_by_credential_availability(dedupe_preserve(merged));
    }
    let configured_provider =
        configured_provider_input_from_policy(policy, WebProviderFamily::Search)
            .as_ref()
            .and_then(|raw| normalize_provider_token_for_family(raw, WebProviderFamily::Search));
    let Some(configured_provider) = configured_provider else {
        return base;
    };
    let mut merged = vec![configured_provider];
    merged.extend(base);
    reorder_search_providers_by_credential_availability(dedupe_preserve(merged))
}

pub(crate) fn search_provider_resolution_snapshot(
    root: &Path,
    policy: &Value,
    request: &Value,
    provider_hint: &str,
) -> Value {
    let mut runtime = runtime_web_family_metadata(root, policy, WebProviderFamily::Search);
    let requested_provider_hint = clean_text(provider_hint, 60).to_ascii_lowercase();
    let request_provider_chain = request_provider_chain_for_family(request, WebProviderFamily::Search);
    let runtime_selected_provider =
        runtime_selected_provider_from_request(request, WebProviderFamily::Search);
    let prefer_runtime_provider =
        request_prefers_runtime_provider(request) || runtime_selected_provider.is_some();
    let provider_chain = resolved_search_provider_chain(provider_hint, request, policy);
    let selected_provider = provider_chain
        .first()
        .cloned()
        .unwrap_or_else(|| "none".to_string());
    let selection_scope = if search_provider_hint_is_explicit(&requested_provider_hint) {
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
    let allow_fallback = !matches!(
        selection_scope,
        "request_provider_hint" | "policy_configured" | "runtime_metadata"
    );
    if let Some(obj) = runtime.as_object_mut() {
        obj.insert(
            "requested_provider_hint".to_string(),
            json!(requested_provider_hint),
        );
        obj.insert(
            "request_provider_chain".to_string(),
            json!(request_provider_chain),
        );
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
        obj.insert("allow_fallback".to_string(), json!(allow_fallback));
        obj.insert(
            "openclaw_runtime_contract".to_string(),
            search_runtime_resolution_contract(),
        );
    }
    runtime
}
