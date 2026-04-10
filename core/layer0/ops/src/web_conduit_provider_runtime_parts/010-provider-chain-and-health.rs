// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
// Web search provider runtime: chain selection + provider health + local search cache.

use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const PROVIDER_HEALTH_REL: &str = "client/runtime/local/state/web_conduit/provider_health.json";
const SEARCH_CACHE_REL: &str = "client/runtime/local/state/web_conduit/search_cache.json";
const SEARCH_CACHE_MAX_ENTRIES: usize = 256;
const SEARCH_CACHE_TTL_SUCCESS_SECS: i64 = 8 * 60;
const SEARCH_CACHE_TTL_NO_RESULTS_SECS: i64 = 90;

const DEFAULT_PROVIDER_CHAIN: &[&str] = &["serperdev", "duckduckgo", "duckduckgo_lite", "bing_rss"];

#[derive(Debug, Clone, Copy)]
pub(crate) struct CircuitPolicy {
    pub enabled: bool,
    pub failure_threshold: u64,
    pub open_for_secs: i64,
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len.max(1))
        .collect::<String>()
}

fn runtime_state_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

fn default_provider_chain_vec() -> Vec<String> {
    DEFAULT_PROVIDER_CHAIN
        .iter()
        .map(|row| row.to_string())
        .collect::<Vec<_>>()
}

fn default_provider_health_state() -> Value {
    json!({"version": 1, "providers": {}})
}

fn default_search_cache_state() -> Value {
    json!({"version": 1, "entries": {}})
}

fn read_json_or(path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("web_conduit_runtime_create_parent_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let encoded = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("web_conduit_runtime_encode_failed:{err}"))?;
    fs::write(&tmp, encoded)
        .map_err(|err| format!("web_conduit_runtime_tmp_write_failed:{err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("web_conduit_runtime_rename_failed:{err}"))?;
    Ok(())
}

fn normalize_provider_token(raw: &str) -> Option<String> {
    let lowered = clean_text(raw, 60).to_ascii_lowercase();
    let canonical = match lowered.as_str() {
        "serper" | "serperdev" => "serperdev",
        "duckduckgo" | "ddg" => "duckduckgo",
        "duckduckgo_lite" | "ddg_lite" | "lite" => "duckduckgo_lite",
        "bing" | "bing_rss" => "bing_rss",
        _ => "",
    };
    if canonical.is_empty() {
        None
    } else {
        Some(canonical.to_string())
    }
}

fn provider_env_keys(provider: &str) -> &'static [&'static str] {
    match provider {
        "serperdev" => &[
            "INFRING_SERPERDEV_API_KEY",
            "SERPERDEV_API_KEY",
            "INFRING_SERPER_API_KEY",
            "SERPER_API_KEY",
        ],
        _ => &[],
    }
}

fn provider_has_runtime_credential_with<F>(provider: &str, resolve_env: F) -> bool
where
    F: Fn(&str) -> Option<String>,
{
    let keys = provider_env_keys(provider);
    if keys.is_empty() {
        return true;
    }
    keys.iter().any(|key| {
        resolve_env(key)
            .map(|raw| !clean_text(&raw, 600).is_empty())
            .unwrap_or(false)
    })
}

fn parse_provider_list(raw: &Value) -> Vec<String> {
    let rows = if let Some(array) = raw.as_array() {
        array
            .iter()
            .filter_map(|row| row.as_str().map(ToString::to_string))
            .collect::<Vec<_>>()
    } else if let Some(single) = raw.as_str() {
        single
            .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
            .map(str::trim)
            .filter(|row| !row.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    rows.into_iter()
        .filter_map(|row| normalize_provider_token(&row))
        .collect::<Vec<_>>()
}

fn dedupe_preserve(rows: Vec<String>) -> Vec<String> {
    rows.into_iter().fold(Vec::<String>::new(), |mut acc, row| {
        if !acc.iter().any(|existing| existing == &row) {
            acc.push(row);
        }
        acc
    })
}

fn provider_chain_from_request_with_env<F>(
    provider_hint: &str,
    request: &Value,
    policy: &Value,
    resolve_env: F,
) -> Vec<String>
where
    F: Fn(&str) -> Option<String> + Copy,
{
    let hint = clean_text(provider_hint, 60).to_ascii_lowercase();
    let request_chain = request
        .get("provider_chain")
        .map(parse_provider_list)
        .unwrap_or_default();
    let request_chain_explicit = !request_chain.is_empty();
    let policy_chain = policy
        .pointer("/web_conduit/search_provider_order")
        .or_else(|| policy.get("search_provider_order"))
        .map(parse_provider_list)
        .unwrap_or_default();
    let configured = if request_chain.is_empty() {
        policy_chain
    } else {
        request_chain
    };
    let configured = if configured.is_empty() {
        default_provider_chain_vec()
    } else {
        configured
    };

    let mut prefix = Vec::<String>::new();
    match hint.as_str() {
        "bing" | "bing_rss" => return vec!["bing_rss".to_string()],
        "duckduckgo" | "ddg" => {
            prefix.push("duckduckgo".to_string());
            prefix.push("duckduckgo_lite".to_string());
            prefix.push("bing_rss".to_string());
        }
        "serper" | "serperdev" => {
            prefix.push("serperdev".to_string());
        }
        _ => {}
    }
    let mut merged = prefix;
    merged.extend(configured);
    merged.extend(default_provider_chain_vec());
    let deduped = dedupe_preserve(merged);
    let hint_explicit = matches!(
        hint.as_str(),
        "bing" | "bing_rss" | "duckduckgo" | "ddg" | "serper" | "serperdev"
    );
    if hint_explicit || request_chain_explicit {
        return deduped;
    }
    let mut credential_ready = Vec::<String>::new();
    let mut missing_credential = Vec::<String>::new();
    for provider in deduped {
        if provider_has_runtime_credential_with(&provider, resolve_env) {
            credential_ready.push(provider);
        } else {
            missing_credential.push(provider);
        }
    }
    credential_ready.extend(missing_credential);
    credential_ready
}

pub(crate) fn provider_chain_from_request(
    provider_hint: &str,
    request: &Value,
    policy: &Value,
) -> Vec<String> {
    provider_chain_from_request_with_env(provider_hint, request, policy, |key| {
        std::env::var(key).ok()
    })
}

pub(crate) fn circuit_policy(policy: &Value) -> CircuitPolicy {
    let scope = policy
        .pointer("/web_conduit/provider_circuit_breaker")
        .or_else(|| policy.get("provider_circuit_breaker"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let enabled = scope
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let failure_threshold = scope
        .get("failure_threshold")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, 12);
    let open_for_secs = scope
        .get("open_for_secs")
        .and_then(Value::as_i64)
        .unwrap_or(5 * 60)
        .clamp(30, 4 * 60 * 60);
    CircuitPolicy {
        enabled,
        failure_threshold,
        open_for_secs,
    }
}

fn provider_health_path(root: &Path) -> PathBuf {
    runtime_state_path(root, PROVIDER_HEALTH_REL)
}

fn load_provider_health(root: &Path) -> Value {
    read_json_or(&provider_health_path(root), default_provider_health_state())
}

fn write_provider_health(root: &Path, state: &Value) {
    let _ = write_json_atomic(&provider_health_path(root), state);
}

pub(crate) fn provider_circuit_open_until(
    root: &Path,
    provider: &str,
    policy: &Value,
) -> Option<i64> {
    let breaker = circuit_policy(policy);
    if !breaker.enabled {
        return None;
    }
    let now_ts = Utc::now().timestamp();
    let provider_id = normalize_provider_token(provider)?;
    let mut state = load_provider_health(root);
    let open_until = state
        .pointer(&format!("/providers/{provider_id}/circuit_open_until"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if open_until > now_ts {
        return Some(open_until);
    }
    if open_until > 0 {
        if let Some(obj) = state
            .get_mut("providers")
            .and_then(Value::as_object_mut)
            .and_then(|providers| providers.get_mut(&provider_id))
            .and_then(Value::as_object_mut)
        {
            obj.insert("circuit_open_until".to_string(), json!(0));
        }
        write_provider_health(root, &state);
    }
    None
}

pub(crate) fn record_provider_attempt(
    root: &Path,
    provider: &str,
    success: bool,
    error: &str,
    policy: &Value,
) {
    let provider_id = match normalize_provider_token(provider) {
        Some(value) => value,
        None => return,
    };
    let breaker = circuit_policy(policy);
    let now = crate::now_iso();
    let now_ts = Utc::now().timestamp();
    let mut state = load_provider_health(root);
    let providers = state
        .get_mut("providers")
        .and_then(Value::as_object_mut)
        .cloned()
        .unwrap_or_default();
    let mut providers = providers;
    let mut row = providers
        .get(&provider_id)
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !row.is_object() {
        row = json!({});
    }
    let mut failures = row
        .get("consecutive_failures")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if success {
        if let Some(obj) = row.as_object_mut() {
            obj.insert("consecutive_failures".to_string(), json!(0));
            obj.insert("circuit_open_until".to_string(), json!(0));
            obj.insert("last_success_at".to_string(), json!(now));
            obj.insert("last_error".to_string(), Value::String(String::new()));
        }
    } else {
        failures = failures.saturating_add(1);
        let mut open_until = row
            .get("circuit_open_until")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if breaker.enabled && failures >= breaker.failure_threshold {
            open_until = now_ts + breaker.open_for_secs;
        }
        if let Some(obj) = row.as_object_mut() {
            obj.insert("consecutive_failures".to_string(), json!(failures));
            obj.insert("circuit_open_until".to_string(), json!(open_until.max(0)));
            obj.insert("last_failure_at".to_string(), json!(now));
            obj.insert("last_error".to_string(), json!(clean_text(error, 280)));
        }
    }
    providers.insert(provider_id, row);
    state["version"] = json!(1);
    state["providers"] = Value::Object(providers);
    write_provider_health(root, &state);
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

