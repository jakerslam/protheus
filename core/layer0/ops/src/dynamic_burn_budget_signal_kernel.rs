// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

const DEFAULT_LATEST_REL: &str = "local/state/ops/dynamic_burn_budget_oracle/latest.json";
const PROVIDER_FAMILY_CONTRACT_TARGETS: &[&str] =
    &["anthropic", "fal", "google", "minimax", "moonshot"];

fn usage() {
    println!("dynamic-burn-budget-signal-kernel commands:");
    println!("  infring-ops dynamic-burn-budget-signal-kernel normalize-pressure [--payload-base64=<json>]");
    println!(
        "  infring-ops dynamic-burn-budget-signal-kernel pressure-rank [--payload-base64=<json>]"
    );
    println!(
        "  infring-ops dynamic-burn-budget-signal-kernel cost-pressure [--payload-base64=<json>]"
    );
    println!(
        "  infring-ops dynamic-burn-budget-signal-kernel load-signal [--payload-base64=<json>]"
    );
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("dynamic_burn_budget_signal_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("dynamic_burn_budget_signal_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("dynamic_burn_budget_signal_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("dynamic_burn_budget_signal_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn clean_text(raw: Option<&Value>, max_len: usize) -> String {
    let input = match raw {
        Some(Value::String(v)) => v.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    };
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn normalize_token(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    let mut prev_us = false;
    for ch in raw.to_ascii_lowercase().chars() {
        let mapped = if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '/' | '-') {
            ch
        } else {
            '_'
        };
        if mapped == '_' {
            if prev_us || out.is_empty() {
                continue;
            }
            prev_us = true;
            out.push(mapped);
        } else {
            prev_us = false;
            out.push(mapped);
        }
        if out.len() >= max_len {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

fn as_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(v)) => v.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn normalize_burn_pressure(value: Option<&Value>) -> &'static str {
    match normalize_token(&clean_text(value, 32), 32).as_str() {
        "critical" => "critical",
        "high" => "high",
        "medium" => "medium",
        "low" => "low",
        _ => "none",
    }
}

fn normalize_provider_family(value: Option<&Value>) -> String {
    let normalized = clean_text(value, 64).to_ascii_lowercase();
    match normalized.as_str() {
        "claude" | "anthropic" => "anthropic".to_string(),
        "fal_ai" | "fal" => "fal".to_string(),
        "gemini" | "google" => "google".to_string(),
        "minimax" => "minimax".to_string(),
        "kimi" | "moonshot" => "moonshot".to_string(),
        _ => normalized,
    }
}

fn signal_enabled(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(v)) => v.as_u64().unwrap_or(0) > 0,
        Some(Value::Array(rows)) => !rows.is_empty(),
        Some(Value::Object(map)) => !map.is_empty(),
        Some(Value::String(raw)) => {
            let token = raw.trim().to_ascii_lowercase();
            matches!(token.as_str(), "1" | "true" | "yes" | "on" | "ready")
        }
        _ => false,
    }
}

fn pressure_rank(value: &str) -> i64 {
    match value {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

fn map_cost_pressure(value: &str) -> f64 {
    match value {
        "critical" => 1.0,
        "high" => 0.75,
        "medium" => 0.45,
        "low" => 0.2,
        _ => 0.0,
    }
}

fn resolve_latest_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let raw = clean_text(
        payload.get("latest_path").or_else(|| payload.get("path")),
        520,
    );
    let chosen = if raw.is_empty() {
        DEFAULT_LATEST_REL.to_string()
    } else {
        raw
    };
    let candidate = PathBuf::from(&chosen);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn load_signal(root: &Path, payload: &Map<String, Value>) -> Value {
    let latest_path = resolve_latest_path(root, payload);
    let loaded = lane_utils::read_json(&latest_path).unwrap_or_else(|| json!(null));
    let projection = loaded
        .get("projection")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let pressure = normalize_burn_pressure(
        payload
            .get("pressure")
            .or_else(|| projection.get("pressure"))
            .or_else(|| loaded.get("pressure")),
    );
    let projected_runway_days = as_f64(
        projection
            .get("projected_runway_days_regime")
            .or_else(|| projection.get("projected_runway_days"))
            .or_else(|| loaded.get("projected_runway_days")),
    );
    let projected_days_to_reset = as_f64(
        projection
            .get("projected_days_to_reset")
            .or_else(|| loaded.get("projected_days_to_reset")),
    );
    let reason_codes = projection
        .get("reason_codes")
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| {
            loaded
                .get("reason_codes")
                .and_then(Value::as_array)
                .cloned()
        })
        .unwrap_or_default();
    let normalized_reasons = reason_codes
        .into_iter()
        .filter_map(|row| {
            Some(normalize_token(
                &match row {
                    Value::String(v) => v,
                    Value::Null => String::new(),
                    other => other.to_string(),
                },
                80,
            ))
        })
        .filter(|row| !row.is_empty())
        .take(24)
        .collect::<Vec<_>>();
    let provider_family = normalize_provider_family(
        payload
            .get("provider_family")
            .or_else(|| projection.get("provider_family"))
            .or_else(|| loaded.get("provider_family")),
    );
    let provider_family_contract_ok = provider_family.is_empty()
        || PROVIDER_FAMILY_CONTRACT_TARGETS
            .iter()
            .any(|target| target == &provider_family.as_str());
    let provider_runtime_contract = signal_enabled(
        payload
            .get("provider_runtime_contract")
            .or_else(|| projection.get("provider_runtime_contract"))
            .or_else(|| loaded.get("provider_runtime_contract")),
    );
    let provider_auth_contract = signal_enabled(
        payload
            .get("provider_auth_contract")
            .or_else(|| projection.get("provider_auth_contract"))
            .or_else(|| loaded.get("provider_auth_contract")),
    );
    let provider_registry_contract = signal_enabled(
        payload
            .get("provider_registry_contract")
            .or_else(|| projection.get("provider_registry_contract"))
            .or_else(|| loaded.get("provider_registry_contract")),
    );
    let provider_discovery_contract = signal_enabled(
        payload
            .get("provider_discovery_contract")
            .or_else(|| projection.get("provider_discovery_contract"))
            .or_else(|| loaded.get("provider_discovery_contract")),
    );
    let provider_contract_ready = provider_runtime_contract
        && provider_auth_contract
        && provider_registry_contract
        && provider_discovery_contract
        && provider_family_contract_ok;
    let available = loaded.is_object()
        && (loaded.get("ok").and_then(Value::as_bool).unwrap_or(false) || !projection.is_empty());
    let ts_value = {
        let raw = clean_text(
            loaded
                .get("ts")
                .or_else(|| loaded.get("updated_at"))
                .or_else(|| loaded.get("last_updated_at")),
            60,
        );
        if raw.is_empty() {
            Value::Null
        } else {
            Value::String(raw)
        }
    };
    json!({
        "available": available,
        "pressure": pressure,
        "pressure_rank": pressure_rank(pressure),
        "cost_pressure": map_cost_pressure(pressure),
        "projected_runway_days": projected_runway_days,
        "projected_days_to_reset": projected_days_to_reset,
        "providers_available": projection.get("providers_available").and_then(Value::as_i64).unwrap_or(0),
        "reason_codes": normalized_reasons,
        "provider_family": provider_family,
        "provider_family_contract_targets": PROVIDER_FAMILY_CONTRACT_TARGETS,
        "provider_family_contract_ok": provider_family_contract_ok,
        "provider_runtime_contract": provider_runtime_contract,
        "provider_auth_contract": provider_auth_contract,
        "provider_registry_contract": provider_registry_contract,
        "provider_discovery_contract": provider_discovery_contract,
        "provider_contract_ready": provider_contract_ready,
        "latest_path": latest_path,
        "latest_path_rel": lane_utils::rel_path(root, &latest_path),
        "ts": ts_value,
        "cadence": loaded.get("cadence").cloned().unwrap_or(Value::Null),
        "projection": Value::Object(projection),
        "payload": loaded,
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("dynamic_burn_budget_signal_kernel_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let result = match command.as_str() {
        "normalize-pressure" => {
            let pressure =
                normalize_burn_pressure(input.get("value").or_else(|| input.get("pressure")));
            cli_receipt(
                "dynamic_burn_budget_signal_kernel_normalize_pressure",
                json!({ "ok": true, "pressure": pressure }),
            )
        }
        "pressure-rank" => {
            let pressure =
                normalize_burn_pressure(input.get("value").or_else(|| input.get("pressure")));
            cli_receipt(
                "dynamic_burn_budget_signal_kernel_pressure_rank",
                json!({ "ok": true, "pressure": pressure, "rank": pressure_rank(pressure) }),
            )
        }
        "cost-pressure" => {
            let pressure =
                normalize_burn_pressure(input.get("value").or_else(|| input.get("pressure")));
            cli_receipt(
                "dynamic_burn_budget_signal_kernel_cost_pressure",
                json!({ "ok": true, "pressure": pressure, "cost_pressure": map_cost_pressure(pressure) }),
            )
        }
        "load-signal" => cli_receipt(
            "dynamic_burn_budget_signal_kernel_load_signal",
            json!({ "ok": true, "signal": load_signal(root, input) }),
        ),
        _ => cli_error(
            "dynamic_burn_budget_signal_kernel_error",
            &format!("unknown_command:{command}"),
        ),
    };
    let exit = if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    };
    print_json_line(&result);
    exit
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_pressure_correctly() {
        assert_eq!(normalize_burn_pressure(Some(&json!("HIGH"))), "high");
        assert_eq!(pressure_rank("high"), 3);
        assert_eq!(map_cost_pressure("critical"), 1.0);
    }
}
