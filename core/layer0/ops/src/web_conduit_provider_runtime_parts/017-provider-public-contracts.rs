const SEARCH_PUBLIC_ARTIFACT_CANDIDATES: &[&str] = &[
    "web-search-contract-api.js",
    "web-search-provider.js",
    "web-search.js",
];
const FETCH_PUBLIC_ARTIFACT_CANDIDATES: &[&str] = &[
    "web-fetch-contract-api.js",
    "web-fetch-provider.js",
    "web-fetch.js",
];

fn fetch_provider_request_contract() -> Value {
    json!({
        "extract_modes": ["text", "markdown"],
        "supports_summary_only": true,
        "supports_timeout_ms": true,
        "supports_cache_ttl_minutes": true
    })
}

fn public_artifact_candidates(family: WebProviderFamily) -> &'static [&'static str] {
    match family {
        WebProviderFamily::Search => SEARCH_PUBLIC_ARTIFACT_CANDIDATES,
        WebProviderFamily::Fetch => FETCH_PUBLIC_ARTIFACT_CANDIDATES,
    }
}

fn provider_ids_for_family(family: WebProviderFamily) -> Vec<String> {
    builtin_provider_descriptors(family)
        .iter()
        .map(|descriptor| descriptor.provider.to_string())
        .collect::<Vec<_>>()
}

fn unsupported_provider_examples(family: WebProviderFamily) -> &'static [&'static str] {
    match family {
        WebProviderFamily::Search => &[
            "brave",
            "exa",
            "firecrawl",
            "google",
            "moonshot",
            "perplexity",
            "tavily",
            "xai",
        ],
        WebProviderFamily::Fetch => &["firecrawl"],
    }
}

fn public_artifact_contract_for_family(family: WebProviderFamily) -> Value {
    json!({
        "family": provider_family_name(family),
        "runtime_mode": "built_in_only",
        "resolution_mode": "explicit_allowlist",
        "artifact_candidates": public_artifact_candidates(family),
        "allowlisted_provider_ids": provider_ids_for_family(family)
    })
}

fn runtime_resolution_contract(family: WebProviderFamily) -> Value {
    json!({
        "family": provider_family_name(family),
        "runtime_mode": "built_in_only",
        "supports_runtime_registry": false,
        "prefer_runtime_providers": false,
        "configured_provider_fallback": "auto-detect",
        "bundled_provider_precedence": true
    })
}

fn provider_contract_fields_snapshot(provider: &str, family: WebProviderFamily) -> Value {
    match family {
        WebProviderFamily::Search => {
            let credential_path = format!("/web_conduit/search_provider_config/{provider}/api_key");
            let env_path = format!("/web_conduit/search_provider_config/{provider}/api_key_env");
            let env_keys = provider_env_keys(provider, family);
            if env_keys.is_empty() {
                json!({
                    "inactive_secret_paths": [],
                    "credential_contract": {
                        "type": "none"
                    },
                    "configured_credential": Value::Null
                })
            } else {
                json!({
                    "inactive_secret_paths": [credential_path.clone()],
                    "credential_contract": {
                        "type": "top-level",
                        "env_keys": env_keys,
                        "inline_path": credential_path,
                        "env_path": env_path
                    },
                    "configured_credential": {
                        "provider_id": provider,
                        "field": "api_key",
                        "path": format!("/web_conduit/search_provider_config/{provider}/api_key"),
                        "env_path": format!("/web_conduit/search_provider_config/{provider}/api_key_env")
                    }
                })
            }
        }
        WebProviderFamily::Fetch => json!({
            "inactive_secret_paths": [],
            "credential_contract": {
                "type": "none"
            },
            "configured_credential": Value::Null
        }),
    }
}

fn provider_registration_contract(policy: &Value, family: WebProviderFamily) -> Value {
    let default_provider_chain = match family {
        WebProviderFamily::Search => resolved_search_provider_chain("", &json!({}), policy),
        WebProviderFamily::Fetch => fetch_provider_chain_from_request("", &json!({}), policy),
    };
    json!({
        "family": provider_family_name(family),
        "selection_policy_path": match family {
            WebProviderFamily::Search => "/web_conduit/search_provider_order",
            WebProviderFamily::Fetch => "/web_conduit/fetch_provider_order",
        },
        "default_provider_chain": default_provider_chain,
        "supported_provider_ids": provider_ids_for_family(family),
        "unsupported_provider_examples": unsupported_provider_examples(family),
        "credential_types_supported": match family {
            WebProviderFamily::Search => json!(["none", "top-level"]),
            WebProviderFamily::Fetch => json!(["none"]),
        },
        "runtime_resolution_contract": runtime_resolution_contract(family),
        "supports_configured_credential": family == WebProviderFamily::Search,
        "scoped_credentials_supported": false,
        "public_artifact_contract": public_artifact_contract_for_family(family)
    })
}

pub(crate) fn search_provider_request_contract(policy: &Value) -> Value {
    json!({
        "default_count": search_default_count(policy),
        "max_count": search_max_count(policy),
        "timeout_ms": search_default_timeout_ms(policy),
        "cache_ttl_minutes": search_default_cache_ttl_minutes(policy),
        "supports_filters": search_filter_support_matrix()
    })
}

pub(crate) fn search_provider_registration_contract(policy: &Value) -> Value {
    provider_registration_contract(policy, WebProviderFamily::Search)
}

pub(crate) fn fetch_provider_registration_contract(policy: &Value) -> Value {
    provider_registration_contract(policy, WebProviderFamily::Fetch)
}

pub(crate) fn web_provider_public_artifact_contracts() -> Value {
    json!({
        "search": public_artifact_contract_for_family(WebProviderFamily::Search),
        "fetch": public_artifact_contract_for_family(WebProviderFamily::Fetch)
    })
}

pub(crate) fn web_tool_catalog_snapshot(policy: &Value) -> Value {
    let search_chain = resolved_search_provider_chain("", &json!({}), policy);
    let fetch_chain = fetch_provider_chain_from_request("", &json!({}), policy);
    json!([
        {
            "tool": "web_search",
            "label": "Web Search",
            "family": "search",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "default_provider": search_chain.first().cloned().unwrap_or_else(|| "none".to_string()),
            "default_provider_chain": search_chain,
            "request_contract": search_provider_request_contract(policy),
            "registration_contract": search_provider_registration_contract(policy)
        },
        {
            "tool": "web_fetch",
            "label": "Web Fetch",
            "family": "fetch",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "default_provider": fetch_chain.first().cloned().unwrap_or_else(|| "none".to_string()),
            "default_provider_chain": fetch_chain,
            "request_contract": fetch_provider_request_contract(),
            "registration_contract": fetch_provider_registration_contract(policy)
        }
    ])
}

pub(crate) fn provider_health_snapshot(root: &Path, providers: &[String]) -> Value {
    let state = load_provider_health(root);
    let now_ts = Utc::now().timestamp();
    let rows = providers
        .iter()
        .map(|provider| {
            let provider_id = normalize_provider_token(provider).unwrap_or_else(|| provider.clone());
            let entry = state
                .pointer(&format!("/providers/{provider_id}"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let circuit_open_until = entry
                .get("circuit_open_until")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            json!({
                "provider": provider_id,
                "consecutive_failures": entry.get("consecutive_failures").and_then(Value::as_u64).unwrap_or(0),
                "circuit_open_until": circuit_open_until,
                "circuit_open": circuit_open_until > now_ts,
                "last_success_at": entry.get("last_success_at").cloned().unwrap_or(Value::Null),
                "last_failure_at": entry.get("last_failure_at").cloned().unwrap_or(Value::Null),
                "last_error": clean_text(entry.get("last_error").and_then(Value::as_str).unwrap_or(""), 220)
            })
        })
        .collect::<Vec<_>>();
    json!(rows)
}

fn provider_catalog_snapshot_with_env_family<F>(
    root: &Path,
    policy: &Value,
    family: WebProviderFamily,
    resolve_env: F,
) -> Value
where
    F: Fn(&str) -> Option<String> + Copy,
{
    let state = if family == WebProviderFamily::Search {
        load_provider_health(root)
    } else {
        default_provider_health_state()
    };
    let now_ts = Utc::now().timestamp();
    let chain = match family {
        WebProviderFamily::Search => {
            provider_chain_from_request_with_env("", &json!({}), policy, resolve_env)
        }
        WebProviderFamily::Fetch => {
            fetch_provider_chain_from_request_with_env("", &json!({}), policy, resolve_env)
        }
    };
    let request_contract = if family == WebProviderFamily::Search {
        search_provider_request_contract(policy)
    } else {
        fetch_provider_request_contract()
    };
    let public_artifact_contract = public_artifact_contract_for_family(family);
    let rows = chain
        .iter()
        .enumerate()
        .map(|(index, provider)| {
            let entry = state
                .pointer(&format!("/providers/{provider}"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let descriptor = provider_descriptor(provider, family);
            let requires_credential = descriptor
                .map(|current| !current.env_keys.is_empty())
                .unwrap_or(false);
            let credential_present =
                provider_has_runtime_credential_with(provider, family, resolve_env);
            let circuit_open_until = entry
                .get("circuit_open_until")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            json!({
                "family": provider_family_name(descriptor.map(|current| current.family).unwrap_or(family)),
                "provider": provider,
                "aliases": provider_aliases(provider, family),
                "source": provider_source_kind(provider, family),
                "requires_credential": requires_credential,
                "credential_present": credential_present,
                "credential_env_keys": provider_env_keys(provider, family),
                "credential_source": resolve_provider_credential_source_with_env(policy, provider, family, resolve_env),
                "available": !requires_credential || credential_present,
                "selected_by_default": index == 0,
                "auto_detect_rank": index + 1,
                "consecutive_failures": if family == WebProviderFamily::Search {
                    entry.get("consecutive_failures").and_then(Value::as_u64).unwrap_or(0)
                } else {
                    0
                },
                "circuit_open_until": if family == WebProviderFamily::Search && circuit_open_until > now_ts {
                    circuit_open_until
                } else {
                    0
                },
                "last_error": if family == WebProviderFamily::Search {
                    clean_text(entry.get("last_error").and_then(Value::as_str).unwrap_or(""), 220)
                } else {
                    String::new()
                },
                "request_contract": request_contract.clone(),
                "contract_fields": provider_contract_fields_snapshot(provider, family),
                "public_artifact_contract": public_artifact_contract.clone()
            })
        })
        .collect::<Vec<_>>();
    json!(rows)
}

pub(crate) fn provider_catalog_snapshot(root: &Path, policy: &Value) -> Value {
    provider_catalog_snapshot_with_env_family(root, policy, WebProviderFamily::Search, |key| {
        std::env::var(key).ok()
    })
}

pub(crate) fn fetch_provider_catalog_snapshot(root: &Path, policy: &Value) -> Value {
    provider_catalog_snapshot_with_env_family(root, policy, WebProviderFamily::Fetch, |key| {
        std::env::var(key).ok()
    })
}

#[cfg(test)]
mod openclaw_provider_contract_tests {
    use super::*;

    #[test]
    fn provider_catalog_snapshot_reports_public_contract_fields_for_serper() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["serperdev", "duckduckgo"],
                "search_default_count": 7,
                "search_cache_ttl_minutes": 13
            }
        });
        let catalog = provider_catalog_snapshot_with_env_family(
            tmp.path(),
            &policy,
            WebProviderFamily::Search,
            |key| {
                if key == "SERPER_API_KEY" {
                    Some("test-key".to_string())
                } else {
                    None
                }
            },
        );
        let rows = catalog.as_array().expect("catalog rows");
        let serper = rows
            .iter()
            .find(|row| row.get("provider").and_then(Value::as_str) == Some("serperdev"))
            .expect("serper row");
        assert_eq!(
            serper.get("credential_source").and_then(Value::as_str),
            Some("env")
        );
        assert_eq!(
            serper
                .pointer("/request_contract/default_count")
                .and_then(Value::as_u64),
            Some(7)
        );
        assert_eq!(
            serper
                .pointer("/contract_fields/credential_contract/type")
                .and_then(Value::as_str),
            Some("top-level")
        );
        assert_eq!(
            serper
                .pointer("/contract_fields/configured_credential/path")
                .and_then(Value::as_str),
            Some("/web_conduit/search_provider_config/serperdev/api_key")
        );
        assert_eq!(
            serper
                .pointer("/public_artifact_contract/artifact_candidates/0")
                .and_then(Value::as_str),
            Some("web-search-contract-api.js")
        );
    }

    #[test]
    fn fetch_provider_catalog_snapshot_reports_public_artifact_contract() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "fetch_provider_order": ["direct_http"]
            }
        });
        let catalog = fetch_provider_catalog_snapshot(tmp.path(), &policy);
        let rows = catalog.as_array().expect("catalog rows");
        let default = rows.first().expect("default fetch provider");
        assert_eq!(
            default
                .pointer("/contract_fields/credential_contract/type")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            default
                .pointer("/public_artifact_contract/artifact_candidates/0")
                .and_then(Value::as_str),
            Some("web-fetch-contract-api.js")
        );
    }

    #[test]
    fn web_tool_catalog_snapshot_reports_registration_contracts() {
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo", "bing_rss"],
                "fetch_provider_order": ["direct_http"],
                "search_default_count": 4
            }
        });
        let catalog = web_tool_catalog_snapshot(&policy);
        let rows = catalog.as_array().expect("tool rows");
        let search = rows
            .iter()
            .find(|row| row.get("tool").and_then(Value::as_str) == Some("web_search"))
            .expect("web_search tool");
        let fetch = rows
            .iter()
            .find(|row| row.get("tool").and_then(Value::as_str) == Some("web_fetch"))
            .expect("web_fetch tool");
        assert_eq!(
            search
                .pointer("/request_contract/default_count")
                .and_then(Value::as_u64),
            Some(4)
        );
        assert_eq!(
            search
                .pointer("/registration_contract/public_artifact_contract/artifact_candidates/0")
                .and_then(Value::as_str),
            Some("web-search-contract-api.js")
        );
        assert_eq!(
            fetch
                .pointer("/registration_contract/public_artifact_contract/runtime_mode")
                .and_then(Value::as_str),
            Some("built_in_only")
        );
        assert_eq!(
            fetch
                .pointer("/registration_contract/runtime_resolution_contract/configured_provider_fallback")
                .and_then(Value::as_str),
            Some("auto-detect")
        );
    }
}
