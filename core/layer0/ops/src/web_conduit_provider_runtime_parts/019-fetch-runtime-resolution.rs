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
        "in_flight_registry_behavior": "returns_empty_provider_set"
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
