
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

fn runtime_browser_materialization_metadata(root: &Path, policy: &Value) -> Value {
    let config = policy
        .pointer("/web_conduit/browser_materialization")
        .cloned()
        .unwrap_or_else(|| json!({}));
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
                "cleanup_tied_to_system_cleanup": true
            },
            "state_path": runtime_web_tools_state_path(root).display().to_string()
        },
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
