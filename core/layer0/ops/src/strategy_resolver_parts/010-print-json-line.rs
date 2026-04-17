// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/execution (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{clean, deterministic_receipt_hash, now_iso};

const DEFAULT_STRATEGY_DIR_REL: &str = "client/runtime/config/strategies";
const DEFAULT_WEAVER_OVERLAY_REL: &str =
    "client/runtime/local/state/autonomy/weaver/strategy_overlay.json";

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn load_payload(argv: &[String]) -> Result<Value, String> {
    if let Some(payload) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&payload)
            .map_err(|err| format!("strategy_resolver_payload_decode_failed:{err}"));
    }
    if let Some(payload_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(payload_b64.as_bytes())
            .map_err(|err| format!("strategy_resolver_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("strategy_resolver_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("strategy_resolver_payload_decode_failed:{err}"));
    }
    if let Some(path) = lane_utils::parse_flag(argv, "payload-file", false) {
        let text = fs::read_to_string(path.trim())
            .map_err(|err| format!("strategy_resolver_payload_file_read_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("strategy_resolver_payload_decode_failed:{err}"));
    }
    Err("strategy_resolver_missing_payload".to_string())
}

fn as_str(value: Option<&Value>) -> String {
    value
        .map(|v| match v {
            Value::String(s) => s.trim().to_string(),
            Value::Null => String::new(),
            _ => v.to_string().trim_matches('"').trim().to_string(),
        })
        .unwrap_or_default()
}

fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(n)) => n.as_i64().map(|v| v != 0).unwrap_or(fallback),
        Some(Value::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => fallback,
        },
        _ => fallback,
    }
}

fn as_i64(value: Option<&Value>) -> Option<i64> {
    match value {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

fn as_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn as_string_array(value: Option<&Value>) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = BTreeSet::<String>::new();
    if let Some(Value::Array(rows)) = value {
        for row in rows {
            let token = as_str(Some(row));
            if token.is_empty() {
                continue;
            }
            if seen.insert(token.clone()) {
                out.push(token);
            }
        }
    }
    out
}

fn normalize_status(raw: Option<&Value>) -> String {
    match as_str(raw).to_ascii_lowercase().as_str() {
        "disabled" | "off" | "paused" => "disabled".to_string(),
        _ => "active".to_string(),
    }
}

fn clamp_i64(v: i64, lo: i64, hi: i64) -> i64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

fn clamp_f64(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

fn normalize_ranking_weights(raw: Option<&Value>) -> Value {
    let mut defaults = Map::new();
    defaults.insert("composite".to_string(), json!(0.35));
    defaults.insert("actionability".to_string(), json!(0.2));
    defaults.insert("directive_fit".to_string(), json!(0.15));
    defaults.insert("signal_quality".to_string(), json!(0.15));
    defaults.insert("expected_value".to_string(), json!(0.1));
    defaults.insert("time_to_value".to_string(), json!(0.0));
    defaults.insert("risk_penalty".to_string(), json!(0.05));

    if let Some(Value::Object(obj)) = raw {
        for (key, value) in obj {
            if !defaults.contains_key(key) {
                continue;
            }
            if let Some(n) = as_f64(Some(value)) {
                if n.is_finite() && n >= 0.0 {
                    defaults.insert(key.clone(), json!(n));
                }
            }
        }
    }

    let total = defaults
        .values()
        .map(|v| as_f64(Some(v)).unwrap_or(0.0))
        .sum::<f64>();

    if total <= 0.0 {
        return Value::Object(defaults);
    }

    let mut normalized = Map::new();
    for (key, value) in defaults {
        let v = as_f64(Some(&value)).unwrap_or(0.0) / total;
        normalized.insert(key, json!((v * 1_000_000.0).round() / 1_000_000.0));
    }
    Value::Object(normalized)
}

fn normalize_campaigns(raw: Option<&Value>, active_only: bool) -> Value {
    let mut out = Vec::<Value>::new();
    if let Some(Value::Array(rows)) = raw {
        for row in rows {
            let Some(obj) = row.as_object() else {
                continue;
            };
            let id = as_str(obj.get("id")).to_ascii_lowercase();
            if id.is_empty() {
                continue;
            }
            let status = normalize_status(obj.get("status"));
            if active_only && status != "active" {
                continue;
            }
            let objective_id = {
                let primary = as_str(obj.get("objective_id"));
                if primary.is_empty() {
                    let fallback = as_str(obj.get("directive_ref"));
                    if fallback.is_empty() {
                        Value::Null
                    } else {
                        Value::String(fallback)
                    }
                } else {
                    Value::String(primary)
                }
            };
            let provider_plugin_ids = {
                let mut ids = BTreeSet::<String>::new();
                for raw_id in as_string_array(obj.get("provider_plugin_ids")) {
                    let token = clean(raw_id.as_str(), 64).to_ascii_lowercase();
                    if !token.is_empty() {
                        ids.insert(token);
                    }
                }
                ids.into_iter().collect::<Vec<_>>()
            };
            let provider_contract = {
                let contract = clean(as_str(obj.get("provider_contract")).as_str(), 80);
                if contract.is_empty() {
                    "webSearchProviders".to_string()
                } else {
                    contract
                }
            };

            let mut next = obj.clone();
            next.insert("id".to_string(), Value::String(id));
            next.insert("status".to_string(), Value::String(status));
            next.insert("objective_id".to_string(), objective_id);
            next.insert(
                "provider_plugin_ids".to_string(),
                Value::Array(
                    provider_plugin_ids
                        .iter()
                        .map(|value| Value::String(value.clone()))
                        .collect(),
                ),
            );
            next.insert(
                "provider_contract".to_string(),
                Value::String(provider_contract),
            );
            next.insert(
                "provider_resolution_mode".to_string(),
                Value::String(if provider_plugin_ids.is_empty() {
                    "manifest_fallback".to_string()
                } else {
                    "explicit_fast_path".to_string()
                }),
            );
            out.push(Value::Object(next));
        }
    }
    Value::Array(out)
}

fn normalize_promotion_policy(raw: Option<&Value>) -> Value {
    let src = raw.and_then(Value::as_object).cloned().unwrap_or_default();
    let min_days = clamp_i64(as_i64(src.get("min_days")).unwrap_or(7), 1, 90);
    let min_attempted = clamp_i64(as_i64(src.get("min_attempted")).unwrap_or(12), 0, 10000);
    let min_verified_rate = clamp_f64(
        as_f64(src.get("min_verified_rate")).unwrap_or(0.5),
        0.0,
        1.0,
    );
    let min_success_criteria_receipts = clamp_i64(
        as_i64(src.get("min_success_criteria_receipts")).unwrap_or(2),
        0,
        10000,
    );
    let min_success_criteria_pass_rate = clamp_f64(
        as_f64(src.get("min_success_criteria_pass_rate")).unwrap_or(0.6),
        0.0,
        1.0,
    );
    let min_objective_coverage = clamp_f64(
        as_f64(src.get("min_objective_coverage")).unwrap_or(0.25),
        0.0,
        1.0,
    );
    let max_objective_no_progress_rate = clamp_f64(
        as_f64(src.get("max_objective_no_progress_rate")).unwrap_or(0.9),
        0.0,
        1.0,
    );
    let max_reverted_rate = clamp_f64(
        as_f64(src.get("max_reverted_rate")).unwrap_or(0.35),
        0.0,
        1.0,
    );
    let max_stop_ratio = clamp_f64(as_f64(src.get("max_stop_ratio")).unwrap_or(0.75), 0.0, 1.0);
    let min_shipped = clamp_i64(as_i64(src.get("min_shipped")).unwrap_or(1), 0, 10000);
    let disable_legacy_fallback_after_quality_receipts = clamp_i64(
        as_i64(src.get("disable_legacy_fallback_after_quality_receipts")).unwrap_or(10),
        0,
        10000,
    );
    let max_success_criteria_quality_insufficient_rate = clamp_f64(
        as_f64(src.get("max_success_criteria_quality_insufficient_rate")).unwrap_or(0.4),
        0.0,
        1.0,
    );

    json!({
        "min_days": min_days,
        "min_attempted": min_attempted,
        "min_verified_rate": ((min_verified_rate * 1000.0).round() / 1000.0),
        "min_success_criteria_receipts": min_success_criteria_receipts,
        "min_success_criteria_pass_rate": ((min_success_criteria_pass_rate * 1000.0).round() / 1000.0),
        "min_objective_coverage": ((min_objective_coverage * 1000.0).round() / 1000.0),
        "max_objective_no_progress_rate": ((max_objective_no_progress_rate * 1000.0).round() / 1000.0),
        "max_reverted_rate": ((max_reverted_rate * 1000.0).round() / 1000.0),
        "max_stop_ratio": ((max_stop_ratio * 1000.0).round() / 1000.0),
        "min_shipped": min_shipped,
        "disable_legacy_fallback_after_quality_receipts": disable_legacy_fallback_after_quality_receipts,
        "max_success_criteria_quality_insufficient_rate": ((max_success_criteria_quality_insufficient_rate * 1000.0).round() / 1000.0)
    })
}
