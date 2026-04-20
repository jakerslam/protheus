
fn runtime_web_tools_exports_contract() -> Value {
    json!({
        "module_entrypoint": "src/agents/tools/web-tools.ts",
        "exports": ["createWebFetchTool", "extractReadableContent", "createWebSearchTool"],
        "web_fetch_factory": "createWebFetchTool",
        "readability_helper": "extractReadableContent",
        "web_search_factory": "createWebSearchTool"
    })
}

fn runtime_web_tools_default_enablement_contract() -> Value {
    json!({
        "web_fetch_enabled_by_default_non_sandbox": true,
        "web_fetch_explicit_disable_supported": true,
        "web_search_runtime_provider_override_supported": true,
        "web_search_runtime_only_provider_hydration": true,
        "runtime_web_search_metadata_fields": [
            "providerConfigured",
            "providerSource",
            "selectedProvider",
            "selectedProviderKeySource",
            "diagnostics",
            "toolSurfaceHealth"
        ],
        "runtime_metadata_provider_override_supported": true,
        "runtime_metadata_provider_override_field": "runtimeWebSearch.selectedProvider"
    })
}

fn runtime_web_fetch_unit_test_harness_contract() -> Value {
    json!({
        "headers_factory_entrypoint": "makeFetchHeaders",
        "headers_key_normalizer": "normalizeLowercaseStringOrEmpty",
        "headers_lookup_contract": "map[normalizeLowercaseStringOrEmpty(key)] ?? null",
        "base_test_config_entrypoint": "createBaseWebFetchToolConfig",
        "base_test_opts_supported": ["maxResponseBytes", "lookupFn"],
        "base_test_defaults": {
            "cache_ttl_minutes": 0,
            "firecrawl_enabled": false
        },
        "base_test_optional_overrides": {
            "max_response_bytes_config_path": "config.tools.web.fetch.maxResponseBytes",
            "lookup_fn_passthrough_field": "lookupFn",
            "max_response_bytes_added_only_when_truthy": true,
            "lookup_fn_added_only_when_present": true
        },
        "max_response_bytes_override_supported": true,
        "readability_test_mock_entrypoint": "web-fetch.test-mocks.ts",
        "readability_test_mock_behavior": "extractReadableContent returns deterministic title/text to avoid heavy dynamic imports"
    })
}

fn default_runtime_web_tools_metadata() -> Value {
    json!({
        "search": {
            "provider_configured": Value::Null,
            "provider_source": "none",
            "selected_provider": Value::Null,
            "selected_provider_key_source": Value::Null,
            "configured_surface_path": Value::Null,
            "config_surface": Value::Null,
            "manifest_contract_owner": Value::Null,
            "public_artifact_runtime": public_artifact_contract_for_family(WebProviderFamily::Search),
            "tool_surface_health": default_runtime_web_family_health(WebProviderFamily::Search),
            "diagnostics": []
        },
        "fetch": {
            "provider_configured": Value::Null,
            "provider_source": "none",
            "selected_provider": Value::Null,
            "selected_provider_key_source": Value::Null,
            "configured_surface_path": Value::Null,
            "config_surface": Value::Null,
            "manifest_contract_owner": Value::Null,
            "public_artifact_runtime": public_artifact_contract_for_family(WebProviderFamily::Fetch),
            "tool_surface_health": default_runtime_web_family_health(WebProviderFamily::Fetch),
            "diagnostics": []
        },
        "image_tool": default_image_tool_runtime_metadata(),
        "openclaw_web_tools_contract": {
            "exports": runtime_web_tools_exports_contract(),
            "default_enablement": runtime_web_tools_default_enablement_contract(),
            "fetch_unit_test_harness": runtime_web_fetch_unit_test_harness_contract()
        },
        "tool_surface_health": default_runtime_web_tools_health_summary(),
        "diagnostics": []
    })
}

pub(crate) fn load_active_runtime_web_tools_metadata(root: &Path) -> Value {
    read_json_or(
        &runtime_web_tools_metadata_path(root),
        default_runtime_web_tools_metadata(),
    )
}

fn store_active_runtime_web_tools_metadata(root: &Path, metadata: &Value) {
    let _ = write_json_atomic(&runtime_web_tools_metadata_path(root), metadata);
}

pub(crate) fn clear_active_runtime_web_tools_metadata(root: &Path) {
    let _ = std::fs::remove_file(runtime_web_tools_metadata_path(root));
}

fn raw_provider_tokens_from_value(raw: &Value) -> Vec<String> {
    let rows = if let Some(array) = raw.as_array() {
        array
            .iter()
            .filter_map(|row| row.as_str())
            .flat_map(|row| row.split(|ch: char| ch == ',' || ch.is_ascii_whitespace()))
            .map(str::trim)
            .filter(|row| !row.is_empty())
            .map(|row| clean_text(row, 60).to_ascii_lowercase())
            .collect::<Vec<_>>()
    } else if let Some(single) = raw.as_str() {
        single
            .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
            .map(str::trim)
            .filter(|row| !row.is_empty())
            .map(|row| clean_text(row, 60).to_ascii_lowercase())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    dedupe_preserve(rows)
}

fn first_raw_provider_token_from_value(raw: &Value) -> Option<String> {
    raw_provider_tokens_from_value(raw).into_iter().next()
}

fn raw_provider_tokens_from_policy(policy: &Value, family: WebProviderFamily) -> Vec<String> {
    match family {
        WebProviderFamily::Search => policy
            .pointer("/web_conduit/search_provider_order")
            .or_else(|| policy.get("search_provider_order"))
            .map(raw_provider_tokens_from_value)
            .unwrap_or_default(),
        WebProviderFamily::Fetch => policy
            .pointer("/web_conduit/fetch_provider_order")
            .or_else(|| policy.get("fetch_provider_order"))
            .map(raw_provider_tokens_from_value)
            .unwrap_or_default(),
    }
}

fn configured_provider_input_from_policy(
    policy: &Value,
    family: WebProviderFamily,
) -> Option<String> {
    let explicit = match family {
        WebProviderFamily::Search => policy
            .pointer("/web_conduit/search_provider")
            .or_else(|| policy.get("search_provider")),
        WebProviderFamily::Fetch => policy
            .pointer("/web_conduit/fetch_provider")
            .or_else(|| policy.get("fetch_provider")),
    }
    .and_then(Value::as_str)
    .map(|raw| clean_text(raw, 60).to_ascii_lowercase())
    .filter(|value| !value.is_empty());
    explicit.or_else(|| match family {
        WebProviderFamily::Search => policy
            .pointer("/web_conduit/search_provider_order")
            .or_else(|| policy.get("search_provider_order"))
            .and_then(first_raw_provider_token_from_value),
        WebProviderFamily::Fetch => policy
            .pointer("/web_conduit/fetch_provider_order")
            .or_else(|| policy.get("fetch_provider_order"))
            .and_then(first_raw_provider_token_from_value),
    })
}

fn runtime_diagnostic(code: &str, message: String, path: &str) -> Value {
    json!({
        "code": code,
        "message": clean_text(&message, 260),
        "path": path
    })
}

fn default_runtime_web_family_health(family: WebProviderFamily) -> Value {
    json!({
        "status": "unavailable",
        "selected_provider_ready": false,
        "selected_provider_requires_credential": false,
        "selected_provider_credential_state": "unknown",
        "blocking_reason": "no_selected_provider",
        "available_provider_count": builtin_provider_descriptors(family).len(),
        "diagnostic_count": 0
    })
}

fn default_runtime_web_tools_health_summary() -> Value {
    json!({
        "status": "unavailable",
        "search_status": "unavailable",
        "fetch_status": "unavailable",
        "search_ready": false,
        "fetch_ready": false
    })
}
