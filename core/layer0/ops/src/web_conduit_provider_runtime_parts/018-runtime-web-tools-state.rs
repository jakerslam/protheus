const RUNTIME_WEB_TOOLS_METADATA_REL: &str =
    "client/runtime/local/state/web_conduit/runtime_web_tools_metadata.json";

fn runtime_web_tools_metadata_path(root: &Path) -> PathBuf {
    runtime_state_path(root, RUNTIME_WEB_TOOLS_METADATA_REL)
}

pub(crate) fn runtime_web_tools_state_path(root: &Path) -> PathBuf {
    runtime_web_tools_metadata_path(root)
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
            "diagnostics": []
        },
        "image_tool": default_image_tool_runtime_metadata(),
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

fn invalid_provider_code(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "WEB_SEARCH_PROVIDER_INVALID_AUTODETECT",
        WebProviderFamily::Fetch => "WEB_FETCH_PROVIDER_INVALID_AUTODETECT",
    }
}

fn auto_detect_code(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "WEB_SEARCH_AUTODETECT_SELECTED",
        WebProviderFamily::Fetch => "WEB_FETCH_AUTODETECT_SELECTED",
    }
}

fn fallback_used_code(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "WEB_SEARCH_KEY_UNRESOLVED_FALLBACK_USED",
        WebProviderFamily::Fetch => "WEB_FETCH_PROVIDER_KEY_UNRESOLVED_FALLBACK_USED",
    }
}

fn no_fallback_code(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "WEB_SEARCH_KEY_UNRESOLVED_NO_FALLBACK",
        WebProviderFamily::Fetch => "WEB_FETCH_PROVIDER_KEY_UNRESOLVED_NO_FALLBACK",
    }
}

fn configured_scope_path(provider: &str, family: WebProviderFamily) -> String {
    match family {
        WebProviderFamily::Search => format!("/web_conduit/search_provider_config/{provider}"),
        WebProviderFamily::Fetch => format!("/web_conduit/fetch_provider_config/{provider}"),
    }
}

fn config_surface_snapshot(
    policy: &Value,
    provider: Option<&str>,
    family: WebProviderFamily,
) -> Value {
    let Some(provider_id) = provider else {
        return Value::Null;
    };
    match family {
        WebProviderFamily::Search => {
            let section = search_provider_config_section(policy, provider_id);
            let inline_present = section
                .and_then(|row| row.get("api_key"))
                .and_then(Value::as_str)
                .map(|raw| !clean_text(raw, 600).is_empty())
                .unwrap_or(false);
            let env_name = section
                .and_then(|row| row.get("api_key_env"))
                .and_then(Value::as_str)
                .map(|raw| clean_text(raw, 160))
                .filter(|value| !value.is_empty());
            json!({
                "path": configured_scope_path(provider_id, family),
                "configured": section.is_some(),
                "has_inline_api_key": inline_present,
                "has_api_key_env": env_name.is_some(),
                "api_key_env": env_name
            })
        }
        WebProviderFamily::Fetch => json!({
            "path": configured_scope_path(provider_id, family),
            "configured": false
        }),
    }
}

fn manifest_contract_owner(provider: Option<&str>, family: WebProviderFamily) -> Value {
    provider
        .map(|provider_id| {
            json!({
                "kind": "built_in",
                "provider": provider_id,
                "family": provider_family_name(family)
            })
        })
        .unwrap_or(Value::Null)
}

fn selected_provider_key_source(
    policy: &Value,
    provider: Option<&str>,
    family: WebProviderFamily,
) -> Value {
    let Some(provider_id) = provider else {
        return Value::Null;
    };
    let raw = match family {
        WebProviderFamily::Search => {
            resolve_provider_credential_source_with_env(policy, provider_id, family, |key| {
                std::env::var(key).ok()
            })
        }
        WebProviderFamily::Fetch => "not_required".to_string(),
    };
    let normalized = match raw.as_str() {
        "policy_inline" => "config",
        "policy_env" | "env" => "env",
        "not_required" => "not_required",
        _ => "missing",
    };
    Value::String(normalized.to_string())
}

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
    json!({
        "configured_provider_input": configured_provider_input,
        "provider_configured": configured_provider,
        "provider_source": provider_source,
        "selected_provider": selected_provider,
        "selected_provider_key_source": selected_provider_key_source(policy, owner_provider, family),
        "selection_fallback_reason": selection_fallback_reason,
        "configured_surface_path": configured_provider
            .as_deref()
            .map(|provider| configured_scope_path(provider, family)),
        "config_surface": config_surface_snapshot(policy, owner_provider, family),
        "manifest_contract_owner": manifest_contract_owner(owner_provider, family),
        "public_artifact_runtime": public_artifact_contract_for_family(family),
        "resolution_contract": runtime_resolution_contract(family),
        "state_path": runtime_web_tools_state_path(root).display().to_string(),
        "diagnostics": diagnostics
    })
}

pub(crate) fn runtime_web_tools_snapshot(root: &Path, policy: &Value) -> Value {
    let search = runtime_web_family_metadata(root, policy, WebProviderFamily::Search);
    let fetch = runtime_web_family_metadata(root, policy, WebProviderFamily::Fetch);
    let image_tool = image_tool_runtime_resolution_snapshot(root, policy, &json!({}));
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
        "image_tool": image_tool,
        "diagnostics": diagnostics
    });
    store_active_runtime_web_tools_metadata(root, &metadata);
    metadata
}

#[cfg(test)]
mod openclaw_runtime_web_tools_tests {
    use super::*;

    #[test]
    fn runtime_web_tools_snapshot_persists_active_state() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo", "bing_rss"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert_eq!(
            metadata
                .pointer("/search/selected_provider")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        let loaded = load_active_runtime_web_tools_metadata(tmp.path());
        assert_eq!(
            loaded
                .pointer("/fetch/selected_provider")
                .and_then(Value::as_str),
            Some("direct_http")
        );
    }

    #[test]
    fn runtime_web_tools_snapshot_load_is_defensive_clone() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo", "bing_rss"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let _snapshot = runtime_web_tools_snapshot(tmp.path(), &policy);
        let mut loaded = load_active_runtime_web_tools_metadata(tmp.path());
        if let Some(search) = loaded.pointer_mut("/search").and_then(Value::as_object_mut) {
            search.insert("selected_provider".to_string(), json!("brave"));
            search.insert("provider_configured".to_string(), json!("brave"));
        }
        let reloaded = load_active_runtime_web_tools_metadata(tmp.path());
        assert_eq!(
            reloaded
                .pointer("/search/selected_provider")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert_eq!(
            reloaded
                .pointer("/search/provider_configured")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
    }

    #[test]
    fn runtime_web_tools_snapshot_flags_invalid_search_provider_tokens() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["perplexity", "duckduckgo"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert_eq!(
            metadata
                .pointer("/search/provider_source")
                .and_then(Value::as_str),
            Some("auto-detect")
        );
        assert!(metadata
            .pointer("/search/diagnostics")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.get("code").and_then(Value::as_str)
                    == Some("WEB_SEARCH_PROVIDER_INVALID_AUTODETECT")))
            .unwrap_or(false));
    }

    #[test]
    fn clear_active_runtime_web_tools_metadata_removes_persisted_snapshot() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let _metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert!(runtime_web_tools_state_path(tmp.path()).exists());
        clear_active_runtime_web_tools_metadata(tmp.path());
        assert!(!runtime_web_tools_state_path(tmp.path()).exists());
        let loaded = load_active_runtime_web_tools_metadata(tmp.path());
        assert_eq!(
            loaded
                .pointer("/search/provider_source")
                .and_then(Value::as_str),
            Some("none")
        );
    }

    #[test]
    fn runtime_web_tools_snapshot_reports_missing_key_fallback() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["serperdev", "duckduckgo"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert_eq!(
            metadata
                .pointer("/search/provider_configured")
                .and_then(Value::as_str),
            Some("serperdev")
        );
        assert_eq!(
            metadata
                .pointer("/search/selected_provider")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert!(metadata
            .pointer("/search/diagnostics")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.get("code").and_then(Value::as_str)
                    == Some("WEB_SEARCH_KEY_UNRESOLVED_FALLBACK_USED")))
            .unwrap_or(false));
    }
}
