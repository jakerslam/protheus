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

fn search_runtime_resolution_contract() -> Value {
    json!({
        "origin": "openclaw_runtime_web_tools_contract",
        "fallback_runtime_resolver": "resolvePluginWebSearchProviders",
        "public_artifact_runtime_resolver": "resolveBundledWebSearchProvidersFromPublicArtifacts",
        "manifest_contract_owner_resolver": "resolveManifestContractOwnerPluginId",
        "diagnostic_code_contract": search_runtime_diagnostic_code_contract()
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
