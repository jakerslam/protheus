// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};
use std::cmp::Ordering;
#[cfg(test)]
use std::fs;
use std::path::Path;

use crate::contract_lane_utils as lane_utils;

const SESSION_ANALYTICS_TUNING_REL: &str =
    "local/state/ops/session_command_tracking/nightly_tuning.json";

#[cfg(test)]
const PROVIDER_REGISTRY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_registry.json";

fn clean_text(raw: &str, max_len: usize) -> String {
    lane_utils::clean_text(Some(raw), max_len.max(1))
}

fn bool_env(name: &str, fallback: bool) -> bool {
    lane_utils::parse_bool(std::env::var(name).ok().as_deref(), fallback)
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn load_session_analytics_tuning(root: &Path) -> Value {
    if !bool_env("INFRING_SESSION_ANALYTICS_ROUTING_ENABLED", true) {
        return json!({});
    }
    read_json(&root.join(SESSION_ANALYTICS_TUNING_REL)).unwrap_or_else(|| json!({}))
}

fn parse_f64_value(value: Option<&Value>) -> f64 {
    value
        .and_then(|row| {
            row.as_f64()
                .or_else(|| row.as_i64().map(|num| num as f64))
                .or_else(|| row.as_u64().map(|num| num as f64))
                .or_else(|| {
                    row.as_str()
                        .and_then(|text| clean_text(text, 40).parse::<f64>().ok())
                })
        })
        .unwrap_or(0.0)
}

fn parse_i64(value: Option<&Value>, fallback: i64) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(fallback)
}

fn parse_bool(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn model_id_is_placeholder(model_id: &str) -> bool {
    matches!(
        clean_text(model_id, 240).to_ascii_lowercase().as_str(),
        "model" | "<model>" | "(model)"
    )
}

#[cfg(test)]
fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, raw);
    }
}

#[derive(Clone)]
struct ModelRow {
    provider: String,
    model: String,
    display_name: String,
    specialty: String,
    specialty_tags: Vec<String>,
    is_local: bool,
    supports_chat: bool,
    needs_key: bool,
    auth_status: String,
    reachable: bool,
    power_signal: i64,
    cost_signal: i64,
    param_count_billion: i64,
    context_size: i64,
    deployment_kind: String,
    local_download_path: String,
    download_available: bool,
    max_output_tokens: i64,
    tier: String,
}

fn scale_to_five(value: i64, min: i64, max: i64) -> i64 {
    if max <= min {
        return 3;
    }
    let ratio = (value - min) as f64 / (max - min) as f64;
    (1.0 + ratio * 4.0).round().clamp(1.0, 5.0) as i64
}

fn registry_rows(root: &Path, snapshot: &Value) -> Vec<ModelRow> {
    let mut rows = Vec::<ModelRow>::new();
    for provider_row in crate::dashboard_provider_runtime::provider_rows(root, snapshot) {
        let provider = clean_text(
            provider_row.get("id").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        if provider.is_empty() {
            continue;
        }
        let is_provider_local = parse_bool(provider_row.get("is_local"), false);
        let supports_chat = parse_bool(provider_row.get("supports_chat"), true);
        let needs_key = parse_bool(provider_row.get("needs_key"), false);
        let auth_status = clean_text(
            provider_row
                .get("auth_status")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            40,
        );
        let reachable = parse_bool(provider_row.get("reachable"), is_provider_local);

        let profiles = provider_row
            .get("model_profiles")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        for (model_name, profile) in profiles {
            let model = clean_text(&model_name, 140);
            if model.is_empty() || model_id_is_placeholder(&model) {
                continue;
            }
            let specialty = clean_text(
                profile
                    .get("specialty")
                    .and_then(Value::as_str)
                    .unwrap_or("general"),
                40,
            );
            let specialty_tags = profile
                .get("specialty_tags")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| v.as_str().map(|s| clean_text(s, 40)))
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>();
            let param_count_billion = parse_i64(profile.get("param_count_billion"), 0).max(0);
            let context_size = parse_i64(
                profile
                    .get("context_size")
                    .or_else(|| profile.get("context_window"))
                    .or_else(|| profile.get("context_tokens")),
                0,
            )
            .max(0);
            let deployment_kind = clean_text(
                profile
                    .get("deployment_kind")
                    .and_then(Value::as_str)
                    .unwrap_or("api"),
                40,
            )
            .to_ascii_lowercase();
            let local_download_path = clean_text(
                profile
                    .get("local_download_path")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                400,
            );
            let download_available = parse_bool(profile.get("download_available"), false)
                || !local_download_path.is_empty()
                || deployment_kind.contains("ollama")
                || deployment_kind.contains("local");
            let max_output_tokens = parse_i64(profile.get("max_output_tokens"), 0).max(0);
            let is_local = is_provider_local
                || deployment_kind.contains("local")
                || deployment_kind.contains("ollama");
            let power_signal =
                parse_i64(profile.get("power_rating"), 0)
                    .max(0)
                    .max(if param_count_billion > 0 {
                        ((param_count_billion as f64).log10() * 2.0).round() as i64
                    } else {
                        0
                    });
            let cost_signal = parse_i64(profile.get("cost_rating"), 0)
                .max(0)
                .max(if is_local {
                    ((param_count_billion as f64 / 20.0).ceil() as i64).clamp(1, 5)
                } else {
                    0
                });
            let tier = clean_text(
                profile
                    .get("tier")
                    .or_else(|| profile.get("specialty"))
                    .and_then(Value::as_str)
                    .unwrap_or("general"),
                40,
            );
            rows.push(ModelRow {
                provider: provider.clone(),
                model,
                display_name: clean_text(
                    profile
                        .get("display_name")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    160,
                ),
                specialty,
                specialty_tags,
                is_local,
                supports_chat,
                needs_key,
                auth_status: auth_status.clone(),
                reachable,
                power_signal,
                cost_signal,
                param_count_billion,
                context_size,
                deployment_kind,
                local_download_path,
                download_available,
                max_output_tokens,
                tier,
            });
        }
    }

    if rows.is_empty() {
        let provider = clean_text(
            snapshot
                .pointer("/app/settings/provider")
                .and_then(Value::as_str)
                .unwrap_or("auto"),
            80,
        );
        let model = clean_text(
            snapshot
                .pointer("/app/settings/model")
                .and_then(Value::as_str)
                .unwrap_or(""),
            140,
        );
        if !model.is_empty() && !model_id_is_placeholder(&model) {
            rows.push(ModelRow {
                provider,
                model,
                display_name: String::new(),
                specialty: "general".to_string(),
                specialty_tags: vec!["general".to_string()],
                is_local: false,
                supports_chat: false,
                needs_key: false,
                auth_status: "unknown".to_string(),
                reachable: false,
                power_signal: 3,
                cost_signal: 3,
                param_count_billion: 0,
                context_size: 0,
                deployment_kind: "api".to_string(),
                local_download_path: String::new(),
                download_available: false,
                max_output_tokens: 0,
                tier: "general".to_string(),
            });
        }
    }
    rows
}

pub fn catalog_payload(root: &Path, snapshot: &Value) -> Value {
    let rows = registry_rows(root, snapshot);
    let power_min = rows.iter().map(|r| r.power_signal).min().unwrap_or(1);
    let power_max = rows.iter().map(|r| r.power_signal).max().unwrap_or(5);
    let cost_min = rows.iter().map(|r| r.cost_signal).min().unwrap_or(1);
    let cost_max = rows.iter().map(|r| r.cost_signal).max().unwrap_or(5);
    let context_min = rows.iter().map(|r| r.context_size).min().unwrap_or(0);
    let context_max = rows.iter().map(|r| r.context_size).max().unwrap_or(1);

    let mut models = rows
        .into_iter()
        .map(|row| {
            let power_rating = scale_to_five(row.power_signal, power_min, power_max);
            let cost_rating = scale_to_five(row.cost_signal, cost_min, cost_max);
            let context_rating = scale_to_five(row.context_size, context_min, context_max);
            let available = row.supports_chat
                && if row.is_local {
                    row.reachable
                } else {
                    !row.needs_key
                        || row.reachable
                        || crate::dashboard_provider_runtime::auth_status_configured(
                            &row.auth_status,
                        )
                };
            let display_name = if row.display_name.is_empty() {
                row.model.clone()
            } else {
                row.display_name.clone()
            };
            json!({
                "id": format!("{}/{}", row.provider, row.model),
                "provider": row.provider,
                "model": row.model,
                "model_name": row.model,
                "runtime_model": row.model,
                "display_name": display_name,
                "is_local": row.is_local,
                "supports_chat": row.supports_chat,
                "available": available,
                "reachable": row.reachable,
                "specialty": row.specialty,
                "specialty_tags": row.specialty_tags,
                "tier": row.tier,
                "params_billion": row.param_count_billion,
                "context_size": row.context_size,
                "context_window": row.context_size,
                "context_window_tokens": row.context_size,
                "power_scale": power_rating,
                "power_rating": power_rating,
                "cost_scale": cost_rating,
                "cost_rating": cost_rating,
                "context_scale": context_rating,
                "needs_key": row.needs_key,
                "auth_status": row.auth_status,
                "deployment_kind": row.deployment_kind,
                "local_download_path": row.local_download_path,
                "download_available": row.download_available,
                "max_output_tokens": row.max_output_tokens
            })
        })
        .collect::<Vec<_>>();
    models.sort_by(|a, b| {
        clean_text(a.get("provider").and_then(Value::as_str).unwrap_or(""), 80)
            .cmp(&clean_text(
                b.get("provider").and_then(Value::as_str).unwrap_or(""),
                80,
            ))
            .then(
                clean_text(a.get("model").and_then(Value::as_str).unwrap_or(""), 140).cmp(
                    &clean_text(b.get("model").and_then(Value::as_str).unwrap_or(""), 140),
                ),
            )
    });
    json!({"ok": true, "models": models})
}

pub fn model_ref_available(
    root: &Path,
    snapshot: &Value,
    provider_id: &str,
    model_name: &str,
) -> bool {
    let provider = clean_text(provider_id, 80).to_ascii_lowercase();
    let model = clean_text(model_name, 240);
    if provider.is_empty() || model.is_empty() {
        return false;
    }
    catalog_payload(root, snapshot)
        .get("models")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                clean_text(
                    row.get("provider").and_then(Value::as_str).unwrap_or(""),
                    80,
                )
                .eq_ignore_ascii_case(&provider)
                    && clean_text(row.get("model").and_then(Value::as_str).unwrap_or(""), 240)
                        == model
                    && parse_bool(row.get("available"), false)
            })
        })
        .unwrap_or(false)
}

pub fn resolve_model_selection(
    root: &Path,
    snapshot: &Value,
    preferred_provider: &str,
    preferred_model: &str,
    request: &Value,
