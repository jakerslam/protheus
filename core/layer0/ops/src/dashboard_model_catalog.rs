// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::fs;
use std::path::Path;

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

fn parse_i64(value: Option<&Value>, fallback: i64) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(fallback)
}

fn parse_bool(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
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
    specialty: String,
    specialty_tags: Vec<String>,
    is_local: bool,
    needs_key: bool,
    auth_status: String,
    power_signal: i64,
    cost_signal: i64,
    param_count_billion: i64,
    context_size: i64,
}

fn scale_to_five(value: i64, min: i64, max: i64) -> i64 {
    if max <= min {
        return 3;
    }
    let ratio = (value - min) as f64 / (max - min) as f64;
    (1.0 + ratio * 4.0).round().clamp(1.0, 5.0) as i64
}

fn registry_rows(root: &Path, snapshot: &Value) -> Vec<ModelRow> {
    let registry = read_json(&root.join(PROVIDER_REGISTRY_REL)).unwrap_or_else(|| json!({}));
    let providers = registry
        .get("providers")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut rows = Vec::<ModelRow>::new();

    for provider_row in providers.values() {
        let provider = clean_text(provider_row.get("id").and_then(Value::as_str).unwrap_or(""), 80);
        if provider.is_empty() {
            continue;
        }
        let is_provider_local = parse_bool(provider_row.get("is_local"), false);
        let needs_key = parse_bool(provider_row.get("needs_key"), false);
        let auth_status = clean_text(
            provider_row
                .get("auth_status")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            40,
        );

        let profiles = provider_row
            .get("model_profiles")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        for (model_name, profile) in profiles {
            let model = clean_text(&model_name, 140);
            if model.is_empty() {
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
                240,
            );
            let is_local = is_provider_local
                || deployment_kind.contains("local")
                || deployment_kind.contains("ollama")
                || !local_download_path.is_empty();
            let power_signal = parse_i64(profile.get("power_rating"), 0).max(0).max(
                if param_count_billion > 0 {
                    ((param_count_billion as f64).log10() * 2.0).round() as i64
                } else {
                    0
                },
            );
            let cost_signal = parse_i64(profile.get("cost_rating"), 0).max(0).max(if is_local {
                ((param_count_billion as f64 / 20.0).ceil() as i64).clamp(1, 5)
            } else {
                0
            });
            rows.push(ModelRow {
                provider: provider.clone(),
                model,
                specialty,
                specialty_tags,
                is_local,
                needs_key,
                auth_status: auth_status.clone(),
                power_signal,
                cost_signal,
                param_count_billion,
                context_size,
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
        if !model.is_empty() {
            rows.push(ModelRow {
                provider,
                model,
                specialty: "general".to_string(),
                specialty_tags: vec!["general".to_string()],
                is_local: false,
                needs_key: false,
                auth_status: "unknown".to_string(),
                power_signal: 3,
                cost_signal: 3,
                param_count_billion: 0,
                context_size: 0,
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
            json!({
                "id": format!("{}:{}", row.provider, row.model),
                "provider": row.provider,
                "model": row.model,
                "is_local": row.is_local,
                "specialty": row.specialty,
                "specialty_tags": row.specialty_tags,
                "params_billion": row.param_count_billion,
                "context_size": row.context_size,
                "power_scale": scale_to_five(row.power_signal, power_min, power_max),
                "cost_scale": scale_to_five(row.cost_signal, cost_min, cost_max),
                "context_scale": scale_to_five(row.context_size, context_min, context_max),
                "needs_key": row.needs_key,
                "auth_status": row.auth_status
            })
        })
        .collect::<Vec<_>>();
    models.sort_by(|a, b| {
        clean_text(a.get("provider").and_then(Value::as_str).unwrap_or(""), 80)
            .cmp(&clean_text(b.get("provider").and_then(Value::as_str).unwrap_or(""), 80))
            .then(
                clean_text(a.get("model").and_then(Value::as_str).unwrap_or(""), 140).cmp(
                    &clean_text(b.get("model").and_then(Value::as_str).unwrap_or(""), 140),
                ),
            )
    });
    json!({"ok": true, "models": models})
}

pub fn route_decision_payload(root: &Path, snapshot: &Value, request: &Value) -> Value {
    let catalog = catalog_payload(root, snapshot);
    let mut rows = catalog
        .get("models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let prefer_local = parse_bool(request.get("prefer_local"), false)
        || parse_bool(request.get("offline_required"), false);
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
    let budget_mode = clean_text(
        request
            .get("budget_mode")
            .and_then(Value::as_str)
            .unwrap_or("balanced"),
        40,
    )
    .to_ascii_lowercase();

    rows.sort_by(|a, b| {
        let score_a = route_score(a, prefer_local, &complexity, &task_type, &budget_mode);
        let score_b = route_score(b, prefer_local, &complexity, &task_type, &budget_mode);
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                clean_text(a.get("id").and_then(Value::as_str).unwrap_or(""), 200).cmp(
                    &clean_text(b.get("id").and_then(Value::as_str).unwrap_or(""), 200),
                )
            })
    });

    let selected = rows.first().cloned().unwrap_or_else(|| json!({}));
    let top = rows.into_iter().take(5).collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "dashboard_model_route_decision",
        "selected": selected,
        "candidates": top,
        "input": {
            "prefer_local": prefer_local,
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
    let power = parse_i64(row.get("power_scale"), 3) as f64;
    let cost = parse_i64(row.get("cost_scale"), 3) as f64;
    let context = parse_i64(row.get("context_scale"), 3) as f64;
    let is_local = parse_bool(row.get("is_local"), false);
    let needs_key = parse_bool(row.get("needs_key"), false);
    let auth_status = clean_text(row.get("auth_status").and_then(Value::as_str).unwrap_or(""), 40)
        .to_ascii_lowercase();
    let specialty = clean_text(row.get("specialty").and_then(Value::as_str).unwrap_or(""), 40)
        .to_ascii_lowercase();

    let mut score = 0.0;
    score += power * if complexity == "high" || complexity == "deep" { 1.8 } else { 0.9 };
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
    if needs_key && auth_status != "set" && auth_status != "ok" {
        score -= 1.5;
    }
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
        assert_eq!(rows.len(), 2);
        assert!(rows
            .iter()
            .all(|row| parse_i64(row.get("power_scale"), 0) >= 1 && parse_i64(row.get("power_scale"), 0) <= 5));
        assert!(rows.iter().any(|row| {
            row.get("provider").and_then(Value::as_str) == Some("ollama")
                && row.get("is_local").and_then(Value::as_bool) == Some(true)
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
    }
}
