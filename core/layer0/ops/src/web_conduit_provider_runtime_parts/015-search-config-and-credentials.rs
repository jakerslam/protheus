const DEFAULT_SEARCH_COUNT: u64 = 8;
const MAX_SEARCH_COUNT: u64 = 12;
const DEFAULT_SEARCH_CACHE_TTL_MINUTES: u64 = 8;
const MAX_SEARCH_CACHE_TTL_MINUTES: u64 = 240;
const DEFAULT_SEARCH_TIMEOUT_MS: u64 = 9_000;
const SEARCH_FILTER_DOCS_URL: &str = "https://docs.openclaw.ai/tools/web";

fn parse_value_u64(raw: Option<&Value>) -> Option<u64> {
    match raw {
        Some(Value::Number(number)) => number.as_u64().or_else(|| {
            number
                .as_i64()
                .and_then(|value| if value >= 0 { Some(value as u64) } else { None })
        }),
        Some(Value::String(text)) => text.trim().parse::<u64>().ok(),
        _ => None,
    }
}

fn parse_request_u64_aliases(
    request: &Value,
    names: &[&str],
    fallback: u64,
    min: u64,
    max: u64,
) -> u64 {
    names.iter()
        .find_map(|name| parse_value_u64(request.get(*name)))
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn policy_u64(policy: &Value, paths: &[&str], fallback: u64, min: u64, max: u64) -> u64 {
    paths.iter()
        .find_map(|path| policy.pointer(path))
        .and_then(|raw| parse_value_u64(Some(raw)))
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn request_string_alias(request: &Value, names: &[&str], max_len: usize) -> Option<String> {
    names.iter().find_map(|name| {
        request
            .get(*name)
            .and_then(Value::as_str)
            .map(|raw| clean_text(raw, max_len))
            .filter(|value| !value.is_empty())
    })
}

fn search_filter_value(request: &Value, name: &str) -> Option<String> {
    request_string_alias(request, &[name], 80).map(|value| value.to_ascii_lowercase())
}

fn search_filter_support_matrix() -> Value {
    json!({
        "allowed_domains": true,
        "exclude_subdomains": true,
        "count": true,
        "timeout_ms": true,
        "cache_ttl_minutes": true,
        "country": false,
        "language": false,
        "freshness": false,
        "date_after": false,
        "date_before": false
    })
}

pub(crate) fn search_default_count(policy: &Value) -> usize {
    policy_u64(
        policy,
        &["/web_conduit/search_default_count", "/search_default_count"],
        DEFAULT_SEARCH_COUNT,
        1,
        MAX_SEARCH_COUNT,
    ) as usize
}

pub(crate) fn search_max_count(policy: &Value) -> usize {
    let fallback = policy_u64(
        policy,
        &["/web_conduit/search_default_count", "/search_default_count"],
        MAX_SEARCH_COUNT,
        1,
        MAX_SEARCH_COUNT,
    );
    policy_u64(
        policy,
        &["/web_conduit/search_max_count", "/search_max_count"],
        fallback.max(DEFAULT_SEARCH_COUNT),
        1,
        MAX_SEARCH_COUNT,
    ) as usize
}

pub(crate) fn search_default_timeout_ms(policy: &Value) -> u64 {
    policy_u64(
        policy,
        &["/web_conduit/search_timeout_ms", "/web_conduit/timeout_ms", "/timeout_ms"],
        DEFAULT_SEARCH_TIMEOUT_MS,
        1_000,
        120_000,
    )
}

pub(crate) fn search_default_cache_ttl_minutes(policy: &Value) -> u64 {
    policy_u64(
        policy,
        &[
            "/web_conduit/search_cache_ttl_minutes",
            "/search_cache_ttl_minutes",
        ],
        DEFAULT_SEARCH_CACHE_TTL_MINUTES,
        0,
        MAX_SEARCH_CACHE_TTL_MINUTES,
    )
}

pub(crate) fn resolve_search_count(request: &Value, policy: &Value) -> usize {
    let default_count = search_default_count(policy) as u64;
    let max_count = search_max_count(policy) as u64;
    parse_request_u64_aliases(
        request,
        &["count", "top_k", "max_results", "num"],
        default_count,
        1,
        max_count,
    ) as usize
}

pub(crate) fn resolve_search_timeout_ms(request: &Value, policy: &Value) -> u64 {
    if let Some(seconds) =
        parse_value_u64(request.get("timeout_seconds").or_else(|| request.get("timeoutSeconds")))
    {
        return (seconds.saturating_mul(1_000)).clamp(1_000, 120_000);
    }
    parse_request_u64_aliases(
        request,
        &["timeout_ms", "timeoutMs"],
        search_default_timeout_ms(policy),
        1_000,
        120_000,
    )
}

pub(crate) fn resolve_search_cache_ttl_seconds(request: &Value, policy: &Value, status: &str) -> i64 {
    if let Some(minutes) = parse_value_u64(
        request
            .get("cache_ttl_minutes")
            .or_else(|| request.get("cacheTtlMinutes")),
    ) {
        return (minutes.min(MAX_SEARCH_CACHE_TTL_MINUTES) as i64).saturating_mul(60);
    }
    if request
        .get("cache")
        .and_then(Value::as_bool)
        .map(|enabled| !enabled)
        .unwrap_or(false)
    {
        return 0;
    }
    let configured = search_default_cache_ttl_minutes(policy);
    if configured == 0 {
        return 0;
    }
    let fallback = if status == "ok" || status == "partial" {
        SEARCH_CACHE_TTL_SUCCESS_SECS
    } else {
        SEARCH_CACHE_TTL_NO_RESULTS_SECS
    };
    ((configured as i64) * 60).max(fallback.min((configured as i64) * 60))
}

pub(crate) fn normalized_search_filters(request: &Value) -> Value {
    json!({
        "country": search_filter_value(request, "country"),
        "language": search_filter_value(request, "language"),
        "freshness": search_filter_value(request, "freshness"),
        "date_after": search_filter_value(request, "date_after").or_else(|| search_filter_value(request, "dateAfter")),
        "date_before": search_filter_value(request, "date_before").or_else(|| search_filter_value(request, "dateBefore"))
    })
}

pub(crate) fn unsupported_search_filter_response(request: &Value) -> Option<Value> {
    let filters = [
        ("country", "country filtering"),
        ("language", "language filtering"),
        ("freshness", "freshness filtering"),
        ("date_after", "date_after/date_before filtering"),
        ("date_before", "date_after/date_before filtering"),
        ("dateAfter", "date_after/date_before filtering"),
        ("dateBefore", "date_after/date_before filtering"),
    ];
    for (key, label) in filters {
        let value = request_string_alias(request, &[key], 80);
        if value.is_some() {
            let normalized = if key == "dateAfter" {
                "date_after"
            } else if key == "dateBefore" {
                "date_before"
            } else {
                key
            };
            return Some(json!({
                "ok": false,
                "error": if normalized.starts_with("date_") {
                    Value::String("unsupported_date_filter".to_string())
                } else {
                    Value::String(format!("unsupported_{normalized}"))
                },
                "unsupported_filter": normalized,
                "message": format!("{label} is not supported by the current built-in search provider set."),
                "docs": SEARCH_FILTER_DOCS_URL,
                "supported_filters": search_filter_support_matrix()
            }));
        }
    }
    None
}

fn search_provider_config_section<'a>(policy: &'a Value, provider: &str) -> Option<&'a Value> {
    policy.pointer(&format!("/web_conduit/search_provider_config/{provider}"))
}

fn resolve_provider_credential_source_with_env<F>(
    policy: &Value,
    provider: &str,
    family: WebProviderFamily,
    resolve_env: F,
) -> String
where
    F: Fn(&str) -> Option<String> + Copy,
{
    let keys = provider_env_keys(provider, family);
    if keys.is_empty() {
        return "not_required".to_string();
    }
    if family == WebProviderFamily::Search {
        if let Some(section) = search_provider_config_section(policy, provider) {
            if let Some(api_key) = section.get("api_key").and_then(Value::as_str) {
                if !clean_text(api_key, 600).is_empty() {
                    return "policy_inline".to_string();
                }
            }
            if let Some(env_key) = section.get("api_key_env").and_then(Value::as_str) {
                let env_name = clean_text(env_key, 160);
                if !env_name.is_empty() {
                    if resolve_env(&env_name)
                        .map(|raw| !clean_text(&raw, 600).is_empty())
                        .unwrap_or(false)
                    {
                        return "policy_env".to_string();
                    }
                    return "policy_env_missing".to_string();
                }
            }
        }
    }
    if keys.iter().any(|key| {
        resolve_env(key)
            .map(|raw| !clean_text(&raw, 600).is_empty())
            .unwrap_or(false)
    }) {
        "env".to_string()
    } else {
        "missing".to_string()
    }
}

pub(crate) fn resolve_search_provider_credential(policy: &Value, provider: &str) -> Option<String> {
    if let Some(section) = search_provider_config_section(policy, provider) {
        if let Some(api_key) = section.get("api_key").and_then(Value::as_str) {
            let cleaned = clean_text(api_key, 600);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
        if let Some(env_key) = section.get("api_key_env").and_then(Value::as_str) {
            let env_name = clean_text(env_key, 160);
            if !env_name.is_empty() {
                if let Ok(value) = std::env::var(&env_name) {
                    let cleaned = clean_text(&value, 600);
                    if !cleaned.is_empty() {
                        return Some(cleaned);
                    }
                }
            }
        }
    }
    provider_env_keys(provider, WebProviderFamily::Search)
        .iter()
        .find_map(|key| {
            std::env::var(key)
                .ok()
                .map(|raw| clean_text(&raw, 600))
                .filter(|value| !value.is_empty())
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

pub(crate) fn web_tool_catalog_snapshot(policy: &Value) -> Value {
    let search_chain = provider_chain_from_request("", &json!({}), policy);
    let fetch_chain = fetch_provider_chain_from_request("", &json!({}), policy);
    json!([
        {
            "tool": "web_search",
            "label": "Web Search",
            "family": "search",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "default_provider": search_chain.first().cloned().unwrap_or_else(|| "none".to_string()),
            "default_provider_chain": search_chain,
            "request_contract": search_provider_request_contract(policy)
        },
        {
            "tool": "web_fetch",
            "label": "Web Fetch",
            "family": "fetch",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "default_provider": fetch_chain.first().cloned().unwrap_or_else(|| "none".to_string()),
            "default_provider_chain": fetch_chain,
            "request_contract": {
                "extract_modes": ["text", "markdown"],
                "supports_summary_only": true,
                "supports_timeout_ms": true,
                "supports_cache_ttl_minutes": true
            }
        }
    ])
}

pub(crate) fn provider_health_snapshot(root: &Path, providers: &[String]) -> Value {
    let state = load_provider_health(root);
    let rows = providers
        .iter()
        .map(|provider| {
            let provider_id = normalize_provider_token(provider).unwrap_or_else(|| provider.clone());
            let entry = state
                .pointer(&format!("/providers/{provider_id}"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            json!({
                "provider": provider_id,
                "consecutive_failures": entry.get("consecutive_failures").and_then(Value::as_u64).unwrap_or(0),
                "circuit_open_until": entry.get("circuit_open_until").and_then(Value::as_i64).unwrap_or(0),
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
                "request_contract": if family == WebProviderFamily::Search {
                    search_provider_request_contract(policy)
                } else {
                    json!({
                        "extract_modes": ["text", "markdown"],
                        "supports_summary_only": true,
                        "supports_timeout_ms": true,
                        "supports_cache_ttl_minutes": true
                    })
                },
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
mod openclaw_search_helper_tests {
    use super::*;

    #[test]
    fn search_request_contract_reflects_policy_defaults() {
        let policy = json!({
            "web_conduit": {
                "search_default_count": 6,
                "search_max_count": 9,
                "search_cache_ttl_minutes": 11,
                "search_timeout_ms": 12000
            }
        });
        let contract = search_provider_request_contract(&policy);
        assert_eq!(contract.get("default_count").and_then(Value::as_u64), Some(6));
        assert_eq!(contract.get("max_count").and_then(Value::as_u64), Some(9));
        assert_eq!(
            contract.get("cache_ttl_minutes").and_then(Value::as_u64),
            Some(11)
        );
        assert_eq!(contract.get("timeout_ms").and_then(Value::as_u64), Some(12_000));
        assert_eq!(
            contract.pointer("/supports_filters/country").and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn unsupported_search_filter_response_flags_country() {
        let out = unsupported_search_filter_response(&json!({
            "country": "us"
        }))
        .expect("unsupported response");
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("unsupported_country")
        );
        assert_eq!(
            out.get("unsupported_filter").and_then(Value::as_str),
            Some("country")
        );
    }

    #[test]
    fn provider_catalog_snapshot_reports_credential_source_and_request_contract() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["serperdev", "duckduckgo"],
                "search_default_count": 7,
                "search_cache_ttl_minutes": 13
            }
        });
        let catalog =
            provider_catalog_snapshot_with_env_family(tmp.path(), &policy, WebProviderFamily::Search, |key| {
                if key == "SERPER_API_KEY" {
                    Some("test-key".to_string())
                } else {
                    None
                }
            });
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
            serper.pointer("/request_contract/default_count")
                .and_then(Value::as_u64),
            Some(7)
        );
        assert!(serper
            .get("credential_env_keys")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.as_str() == Some("SERPER_API_KEY")))
            .unwrap_or(false));
    }

    #[test]
    fn web_tool_catalog_snapshot_reports_web_search_and_web_fetch() {
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
            search.pointer("/request_contract/default_count")
                .and_then(Value::as_u64),
            Some(4)
        );
        assert_eq!(
            fetch.pointer("/request_contract/supports_summary_only")
                .and_then(Value::as_bool),
            Some(true)
        );
    }
}
