// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};
use std::cmp::Ordering;
#[cfg(test)]
use std::fs;
use std::path::Path;

const SESSION_ANALYTICS_TUNING_REL: &str =
    "local/state/ops/session_command_tracking/nightly_tuning.json";

#[cfg(test)]
const PROVIDER_REGISTRY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_registry.json";

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn bool_env(name: &str, fallback: bool) -> bool {
    match std::env::var(name) {
        Ok(raw) => matches!(
            clean_text(&raw, 40).to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => fallback,
    }
}

fn read_json(path: &Path) -> Option<Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
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
) -> (String, String, Option<Value>) {
    let provider = clean_text(preferred_provider, 80);
    let model = clean_text(preferred_model, 240);
    let needs_route = provider.is_empty()
        || provider.eq_ignore_ascii_case("auto")
        || model.is_empty()
        || model.eq_ignore_ascii_case("auto")
        || !model_ref_available(root, snapshot, &provider, &model);
    if !needs_route {
        return (provider, model, None);
    }

    let route = route_decision_payload(root, snapshot, request);
    let routed_provider = clean_text(
        route
            .pointer("/route/provider")
            .or_else(|| route.pointer("/selected/provider"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let routed_model = clean_text(
        route
            .pointer("/route/model")
            .or_else(|| route.pointer("/selected/model"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    if routed_provider.is_empty() || routed_model.is_empty() {
        return (provider, model, None);
    }
    (routed_provider, routed_model, Some(route))
}

pub fn route_decision_payload(root: &Path, snapshot: &Value, request: &Value) -> Value {
    let tuning = load_session_analytics_tuning(root);
    let catalog = catalog_payload(root, snapshot);
    let mut rows = catalog
        .get("models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let offline_required = parse_bool(request.get("offline_required"), false);
    let prefer_local = parse_bool(request.get("prefer_local"), false) || offline_required;
    let complexity = clean_text(
        request
            .get("complexity")
            .and_then(Value::as_str)
            .unwrap_or("general"),
        40,
    )
    .to_ascii_lowercase();
    let task_type = clean_text(
        request
            .get("task_type")
            .or_else(|| request.get("role"))
            .and_then(Value::as_str)
            .unwrap_or("general"),
        80,
    )
    .to_ascii_lowercase();
    let mut budget_mode = clean_text(
        request
            .get("budget_mode")
            .and_then(Value::as_str)
            .unwrap_or("balanced"),
        40,
    )
    .to_ascii_lowercase();
    let tuned_budget_mode = clean_text(
        tuning
            .pointer("/routing/default_budget_mode")
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    )
    .to_ascii_lowercase();
    let default_budget_override_applied =
        if (budget_mode.is_empty() || budget_mode == "balanced") && !tuned_budget_mode.is_empty() {
            budget_mode = tuned_budget_mode.clone();
            true
        } else {
            false
        };
    let model_biases = tuning
        .pointer("/routing/model_bias")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if !model_biases.is_empty() {
        for row in &mut rows {
            let provider = clean_text(
                row.get("provider").and_then(Value::as_str).unwrap_or(""),
                80,
            );
            let model = clean_text(row.get("model").and_then(Value::as_str).unwrap_or(""), 240);
            let key = format!("{provider}/{model}");
            let bias = parse_f64_value(
                model_biases
                    .get(row.get("id").and_then(Value::as_str).unwrap_or(""))
                    .or_else(|| model_biases.get(&key))
                    .or_else(|| model_biases.get(&model)),
            );
            if bias.abs() > f64::EPSILON {
                row["route_bias"] = json!(bias);
            }
        }
    }
    if offline_required {
        rows.retain(|row| parse_bool(row.get("is_local"), false));
        let has_ollama = rows.iter().any(|row| {
            clean_text(
                row.get("provider").and_then(Value::as_str).unwrap_or(""),
                80,
            )
            .eq_ignore_ascii_case("ollama")
                && parse_bool(row.get("available"), false)
        });
        if has_ollama {
            rows.retain(|row| {
                clean_text(
                    row.get("provider").and_then(Value::as_str).unwrap_or(""),
                    80,
                )
                .eq_ignore_ascii_case("ollama")
            });
        }
    }

    rows.sort_by(|a, b| {
        let score_a = route_score(a, prefer_local, &complexity, &task_type, &budget_mode);
        let score_b = route_score(b, prefer_local, &complexity, &task_type, &budget_mode);
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                clean_text(a.get("id").and_then(Value::as_str).unwrap_or(""), 200).cmp(&clean_text(
                    b.get("id").and_then(Value::as_str).unwrap_or(""),
                    200,
                ))
            })
    });

    let routing_policy = crate::dashboard_provider_runtime::routing_policy(root);
    let strategy = clean_text(
        routing_policy
            .pointer("/load_balancing/strategy")
            .and_then(Value::as_str)
            .unwrap_or("score_weighted"),
        40,
    )
    .to_ascii_lowercase();
    let strategy_is_round_robin = strategy == "round_robin";
    let pool_limit = if strategy_is_round_robin {
        rows.len().min(3)
    } else {
        1
    }
    .max(1);
    let selected_index = if strategy_is_round_robin {
        let selector_seed = crate::deterministic_receipt_hash(&json!({
            "agent_id": request.get("agent_id").cloned().unwrap_or(Value::Null),
            "task_type": task_type,
            "complexity": complexity,
            "budget_mode": budget_mode,
            "token_count": request.get("token_count").cloned().unwrap_or(Value::Null),
            "seed": routing_policy.pointer("/load_balancing/seed").cloned().unwrap_or_else(|| json!("stable"))
        }));
        let hex = selector_seed.chars().take(8).collect::<String>();
        let seed = u64::from_str_radix(&hex, 16).unwrap_or(0);
        (seed as usize) % pool_limit
    } else {
        0
    };
    let selected = rows
        .get(selected_index)
        .cloned()
        .or_else(|| rows.first().cloned())
        .unwrap_or_else(|| json!({}));
    let top = rows.into_iter().take(5).collect::<Vec<_>>();
    let selected_provider = clean_text(
        selected
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let selected_model = clean_text(
        selected.get("model").and_then(Value::as_str).unwrap_or(""),
        240,
    );
    let fallback_chain = crate::dashboard_provider_runtime::routing_fallback_chain(
        root,
        &selected_provider,
        &selected_model,
    );
    let retry_policy = routing_policy
        .get("retry")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let route = json!({
        "provider": selected.get("provider").cloned().unwrap_or_else(|| json!("")),
        "model": selected.get("model").cloned().unwrap_or_else(|| json!("")),
        "model_id": selected.get("id").cloned().unwrap_or_else(|| json!("")),
        "selected_provider": selected.get("provider").cloned().unwrap_or_else(|| json!("")),
        "selected_model": selected.get("model").cloned().unwrap_or_else(|| json!("")),
        "selected_model_id": selected.get("id").cloned().unwrap_or_else(|| json!("")),
        "context_window": selected
            .get("context_window")
            .cloned()
            .unwrap_or_else(|| json!(0)),
        "context_window_tokens": selected
            .get("context_window_tokens")
            .cloned()
            .unwrap_or_else(|| json!(0)),
        "fallback_chain": fallback_chain,
        "retry_policy": retry_policy,
        "load_balancing": routing_policy
            .get("load_balancing")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "selection_strategy": strategy,
        "selection_index": selected_index
    });
    json!({
        "ok": true,
        "type": "dashboard_model_route_decision",
        "selected": selected,
        "route": route,
        "selected_provider": selected.get("provider").cloned().unwrap_or_else(|| json!("")),
        "selected_model": selected.get("model").cloned().unwrap_or_else(|| json!("")),
        "selected_model_id": selected.get("id").cloned().unwrap_or_else(|| json!("")),
        "candidates": top,
        "routing_policy": routing_policy,
        "analytics_tuning": {
            "enabled": bool_env("INFRING_SESSION_ANALYTICS_ROUTING_ENABLED", true),
            "default_budget_override_applied": default_budget_override_applied,
            "default_budget_mode": tuned_budget_mode,
            "model_bias_entries": model_biases.len()
        },
        "input": {
            "prefer_local": prefer_local,
            "offline_required": offline_required,
            "complexity": complexity,
            "task_type": task_type,
            "budget_mode": budget_mode
        }
    })
}

fn route_score(
    row: &Value,
    prefer_local: bool,
    complexity: &str,
    task_type: &str,
    budget_mode: &str,
) -> f64 {
    if !parse_bool(row.get("available"), true) {
        return -1000.0;
    }
    let power = parse_i64(row.get("power_scale"), 3) as f64;
    let cost = parse_i64(row.get("cost_scale"), 3) as f64;
    let context = parse_i64(row.get("context_scale"), 3) as f64;
    let is_local = parse_bool(row.get("is_local"), false);
    let needs_key = parse_bool(row.get("needs_key"), false);
    let auth_status = clean_text(
        row.get("auth_status").and_then(Value::as_str).unwrap_or(""),
        40,
    )
    .to_ascii_lowercase();
    let specialty = clean_text(
        row.get("specialty").and_then(Value::as_str).unwrap_or(""),
        40,
    )
    .to_ascii_lowercase();
    let route_bias = parse_f64_value(row.get("route_bias"));

    let mut score = 0.0;
    score += power
        * if complexity == "high" || complexity == "deep" {
            1.8
        } else {
            0.9
        };
    score += context * if task_type.contains("long") { 1.2 } else { 0.4 };
    score += if budget_mode.contains("cheap") || budget_mode.contains("low") {
        (6.0 - cost) * 1.2
    } else {
        power * 0.4
    };
    if task_type.contains("code") && (specialty.contains("code") || specialty.contains("dev")) {
        score += 2.0;
    }
    if prefer_local {
        score += if is_local { 4.0 } else { -4.0 };
    }
    if needs_key && !crate::dashboard_provider_runtime::auth_status_configured(&auth_status) {
        score -= 1.5;
    }
    score += route_bias;
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_emits_scaled_metadata_and_local_flag() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(PROVIDER_REGISTRY_REL),
            &json!({
                "providers": {
                    "ollama": {
                        "id": "ollama",
                        "is_local": true,
                        "needs_key": false,
                        "auth_status": "ok",
                        "model_profiles": {
                            "qwen2.5-coder:7b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 7, "specialty":"coding"}
                        }
                    },
                    "openai": {
                        "id": "openai",
                        "is_local": false,
                        "needs_key": true,
                        "auth_status": "not_set",
                        "model_profiles": {
                            "gpt-5": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 70, "specialty":"general"}
                        }
                    }
                }
            }),
        );
        let catalog = catalog_payload(root.path(), &json!({"ok": true}));
        let rows = catalog
            .get("models")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.len() >= 2);
        assert!(rows
            .iter()
            .any(|row| { row.get("id").and_then(Value::as_str) == Some("openai/gpt-5") }));
        assert!(rows
            .iter()
            .all(|row| parse_i64(row.get("power_scale"), 0) >= 1
                && parse_i64(row.get("power_scale"), 0) <= 5));
        assert!(rows.iter().any(|row| {
            row.get("provider").and_then(Value::as_str) == Some("ollama")
                && row.get("is_local").and_then(Value::as_bool) == Some(true)
        }));
        assert!(rows.iter().any(|row| {
            row.get("id").and_then(Value::as_str) == Some("ollama/qwen2.5-coder:7b")
                && row.get("available").and_then(Value::as_bool) == Some(true)
        }));
    }

    #[test]
    fn route_prefers_local_when_offline_required() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(PROVIDER_REGISTRY_REL),
            &json!({
                "providers": {
                    "ollama": {
                        "id": "ollama",
                        "is_local": true,
                        "needs_key": false,
                        "auth_status": "ok",
                        "model_profiles": {
                            "smallthinker:4b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 4, "specialty":"general"}
                        }
                    },
                    "openai": {
                        "id": "openai",
                        "is_local": false,
                        "needs_key": true,
                        "auth_status": "set",
                        "model_profiles": {
                            "gpt-5": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 70, "specialty":"general"}
                        }
                    }
                }
            }),
        );
        let decision = route_decision_payload(
            root.path(),
            &json!({"ok": true}),
            &json!({"offline_required": true, "task_type": "general"}),
        );
        assert_eq!(
            decision
                .get("selected")
                .and_then(|v| v.get("provider"))
                .and_then(Value::as_str),
            Some("ollama")
        );
        assert!(decision
            .pointer("/route/fallback_chain")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(
            decision
                .pointer("/route/retry_policy/max_total_attempts")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 1
        );
    }

    #[test]
    fn route_applies_session_analytics_tuning_budget_and_model_bias() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(PROVIDER_REGISTRY_REL),
            &json!({
                "providers": {
                    "ollama": {
                        "id": "ollama",
                        "is_local": true,
                        "needs_key": false,
                        "auth_status": "ok",
                        "model_profiles": {
                            "qwen2.5-coder:7b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 7, "specialty":"coding"},
                            "smallthinker:4b": {"power_rating": 3, "cost_rating": 3, "param_count_billion": 4, "specialty":"general"}
                        }
                    }
                }
            }),
        );
        write_json(
            &root.path().join(SESSION_ANALYTICS_TUNING_REL),
            &json!({
                "routing": {
                    "default_budget_mode": "cheap",
                    "model_bias": {
                        "ollama/qwen2.5-coder:7b": 1.2
                    }
                }
            }),
        );
        let decision = route_decision_payload(
            root.path(),
            &json!({"ok": true}),
            &json!({
                "task_type": "code",
                "budget_mode": "balanced",
                "offline_required": true,
                "prefer_local": true
            }),
        );
        assert_eq!(
            decision
                .pointer("/analytics_tuning/default_budget_override_applied")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            decision
                .pointer("/input/budget_mode")
                .and_then(Value::as_str),
            Some("cheap")
        );
        assert_eq!(
            decision
                .pointer("/analytics_tuning/model_bias_entries")
                .and_then(Value::as_u64),
            Some(1)
        );
        let qwen_row = decision
            .get("candidates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 200)
                        == "ollama/qwen2.5-coder:7b"
                })
            })
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            qwen_row.get("route_bias").and_then(Value::as_f64),
            Some(1.2)
        );
    }

    #[test]
    fn hosted_download_stub_stays_unavailable_without_chat_backend() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(PROVIDER_REGISTRY_REL),
            &json!({
                "providers": {
                    "cohere": {
                        "id": "cohere",
                        "display_name": "Cohere",
                        "is_local": false,
                        "needs_key": true,
                        "auth_status": "not_set",
                        "reachable": false,
                        "model_profiles": {
                            "command-r": {
                                "power_rating": 3,
                                "cost_rating": 3,
                                "deployment_kind": "api",
                                "local_download_path": "/tmp/cohere/command-r",
                                "download_available": true
                            }
                        }
                    }
                }
            }),
        );
        let catalog = catalog_payload(root.path(), &json!({"ok": true}));
        let row = catalog
            .get("models")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row.get("id").and_then(Value::as_str) == Some("cohere/command-r"))
            })
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(row.get("is_local").and_then(Value::as_bool), Some(false));
        assert_eq!(
            row.get("supports_chat").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(row.get("available").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn catalog_filters_placeholder_model_rows() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(PROVIDER_REGISTRY_REL),
            &json!({
                "providers": {
                    "ollama": {
                        "id": "ollama",
                        "is_local": true,
                        "needs_key": false,
                        "auth_status": "configured",
                        "reachable": true,
                        "model_profiles": {
                            "model": {"power_rating": 1, "cost_rating": 1, "specialty":"general"},
                            "qwen2.5-coder:7b": {"power_rating": 2, "cost_rating": 1, "specialty":"coding"}
                        }
                    }
                }
            }),
        );
        let catalog = catalog_payload(root.path(), &json!({"ok": true}));
        let rows = catalog
            .get("models")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some("ollama/qwen2.5-coder:7b")));
        assert!(!rows
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some("ollama/model")));
    }

    #[test]
    fn catalog_default_seed_is_not_truncated_to_three_rows() {
        let root = tempfile::tempdir().expect("tempdir");
        let catalog = catalog_payload(root.path(), &json!({"ok": true}));
        let rows = catalog
            .get("models")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            rows.len() >= 12,
            "catalog should expose broad provider/model surface, got {} rows",
            rows.len()
        );
        assert!(rows
            .iter()
            .any(|row| { row.get("id").and_then(Value::as_str) == Some("ollama/qwen3:4b") }));
        assert!(rows.iter().any(|row| {
            row.get("id").and_then(Value::as_str) == Some("openrouter/google/gemini-2.5-flash")
        }));
    }
}
