// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Number, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso, parse_args};

const THRESHOLD_KEYS: &[&str] = &[
    "min_signal_quality",
    "min_sensory_signal_score",
    "min_sensory_relevance_score",
    "min_directive_fit",
    "min_actionability_score",
    "min_eye_score_ema",
    "min_composite_eligibility",
];

const RANKING_WEIGHT_KEYS: &[&str] = &[
    "composite",
    "actionability",
    "directive_fit",
    "signal_quality",
    "expected_value",
    "time_to_value",
    "risk_penalty",
];

const VALUE_CURRENCY_KEYS: &[&str] = &[
    "revenue",
    "delivery",
    "user_value",
    "quality",
    "time_savings",
    "learning",
];

fn usage() {
    println!("outcome-fitness-kernel commands:");
    println!("  protheus-ops outcome-fitness-kernel load-policy --payload-base64=<json>");
    println!("  protheus-ops outcome-fitness-kernel normalize-threshold-overrides --payload-base64=<json>");
    println!(
        "  protheus-ops outcome-fitness-kernel normalize-ranking-weights --payload-base64=<json>"
    );
    println!("  protheus-ops outcome-fitness-kernel normalize-proposal-type-threshold-offsets --payload-base64=<json>");
    println!("  protheus-ops outcome-fitness-kernel normalize-promotion-policy-overrides --payload-base64=<json>");
    println!("  protheus-ops outcome-fitness-kernel normalize-value-currency-policy-overrides --payload-base64=<json>");
    println!(
        "  protheus-ops outcome-fitness-kernel normalize-proposal-type-key --payload-base64=<json>"
    );
    println!("  protheus-ops outcome-fitness-kernel normalize-value-currency-token --payload-base64=<json>");
    println!("  protheus-ops outcome-fitness-kernel proposal-type-threshold-offsets-for --payload-base64=<json>");
}

fn stamp_receipt(value: &mut Value) {
    value["receipt_hash"] = Value::String(deterministic_receipt_hash(value));
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload,
    });
    stamp_receipt(&mut out);
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true,
    });
    stamp_receipt(&mut out);
    out
}

fn emit_cli_receipt(kind: &str, payload: Value) -> i32 {
    print_json_line(&cli_receipt(kind, payload));
    0
}

fn emit_cli_error(kind: &str, error: &str) -> i32 {
    print_json_line(&cli_error(kind, error));
    1
}

fn run_payload_command<F>(argv: &[String], kind: &str, map: F) -> i32
where
    F: FnOnce(Value) -> Value,
{
    match payload_json(argv) {
        Ok(payload) => emit_cli_receipt(kind, map(payload)),
        Err(err) => emit_cli_error(kind, &err),
    }
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("outcome_fitness_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("outcome_fitness_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("outcome_fitness_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("outcome_fitness_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn clean_text(value: impl ToString, max_len: usize) -> String {
    let mut out = value.to_string().trim().to_string();
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn as_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

fn to_number(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(v)) => v.trim().parse::<f64>().ok(),
        Some(Value::Bool(v)) => Some(if *v { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn clamp_number(value: Option<&Value>, lo: f64, hi: f64, fallback: f64) -> f64 {
    let Some(mut n) = to_number(value) else {
        return fallback;
    };
    if !n.is_finite() {
        return fallback;
    }
    if n < lo {
        n = lo;
    }
    if n > hi {
        n = hi;
    }
    n
}

fn clamp_int(value: Option<&Value>, lo: i64, hi: i64, fallback: i64) -> i64 {
    let Some(mut n) = to_number(value).map(|v| v.floor() as i64) else {
        return fallback;
    };
    if n < lo {
        n = lo;
    }
    if n > hi {
        n = hi;
    }
    n
}

fn json_number(value: f64) -> Value {
    Value::Number(Number::from_f64(value).unwrap_or_else(|| Number::from(0)))
}

fn round_to_places(value: f64, places: u32) -> f64 {
    let factor = 10f64.powi(places as i32);
    (value * factor).round() / factor
}

fn normalize_key(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    let mut previous_underscore = false;
    for ch in raw.trim().to_ascii_lowercase().chars() {
        let normalized = if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '-') {
            previous_underscore = false;
            ch
        } else {
            if previous_underscore {
                continue;
            }
            previous_underscore = true;
            '_'
        };
        out.push(normalized);
        if out.len() >= max_len {
            break;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        String::new()
    } else {
        trimmed.to_string()
    }
}

fn normalize_proposal_type_key(raw: &str) -> String {
    normalize_key(raw, 64)
}

fn normalize_value_currency_token(raw: &str) -> String {
    let token = normalize_key(raw, 64);
    if VALUE_CURRENCY_KEYS.contains(&token.as_str()) {
        token
    } else {
        String::new()
    }
}

fn normalize_threshold_overrides(value: Option<&Value>) -> Map<String, Value> {
    let mut out = Map::new();
    let Some(obj) = value.and_then(Value::as_object) else {
        return out;
    };
    for key in THRESHOLD_KEYS {
        if let Some(v) = obj.get(*key).and_then(|row| to_number(Some(row))) {
            if v.is_finite() {
                out.insert((*key).to_string(), json_number(v));
            }
        }
    }
    out
}

fn normalize_ranking_weights(value: Option<&Value>) -> Option<Map<String, Value>> {
    let obj = value.and_then(Value::as_object)?;
    let mut rows = Vec::<(&str, f64)>::new();
    let mut total = 0.0;
    for key in RANKING_WEIGHT_KEYS {
        let Some(weight) = obj.get(*key).and_then(|row| to_number(Some(row))) else {
            continue;
        };
        if !weight.is_finite() || weight < 0.0 {
            continue;
        }
        total += weight;
        rows.push((key, weight));
    }
    if total <= 0.0 {
        return None;
    }
    let mut out = Map::new();
    for (key, weight) in rows {
        out.insert(
            key.to_string(),
            json_number(round_to_places(weight / total, 6)),
        );
    }
    Some(out)
}

fn normalize_proposal_type_threshold_offsets(value: Option<&Value>) -> Map<String, Value> {
    let mut out = Map::new();
    let Some(obj) = value.and_then(Value::as_object) else {
        return out;
    };
    for (raw_key, row) in obj {
        let key = normalize_proposal_type_key(raw_key);
        if key.is_empty() {
            continue;
        }
        let normalized = normalize_threshold_overrides(Some(row));
        if normalized.is_empty() {
            continue;
        }
        out.insert(key, Value::Object(normalized));
    }
    out
}

fn normalize_promotion_policy_overrides(value: Option<&Value>) -> Map<String, Value> {
    let mut out = Map::new();
    let Some(obj) = value.and_then(Value::as_object) else {
        return out;
    };
    if obj.contains_key("disable_legacy_fallback_after_quality_receipts") {
        out.insert(
            "disable_legacy_fallback_after_quality_receipts".to_string(),
            Value::from(clamp_int(
                obj.get("disable_legacy_fallback_after_quality_receipts"),
                0,
                10_000,
                10,
            )),
        );
    }
    if obj.contains_key("max_success_criteria_quality_insufficient_rate") {
        out.insert(
            "max_success_criteria_quality_insufficient_rate".to_string(),
            json_number(round_to_places(
                clamp_number(
                    obj.get("max_success_criteria_quality_insufficient_rate"),
                    0.0,
                    1.0,
                    0.4,
                ),
                3,
            )),
        );
    }
    out
}

fn normalize_promotion_policy_audit(value: Option<&Value>) -> Value {
    let empty = json!({});
    let src = value.unwrap_or(&empty);
    let quality_lock = src
        .get("quality_lock")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    json!({
        "quality_lock": {
            "active": quality_lock.get("active").and_then(Value::as_bool).unwrap_or(false),
            "was_locked": quality_lock.get("was_locked").and_then(Value::as_bool).unwrap_or(false),
            "stable_window_streak": clamp_int(quality_lock.get("stable_window_streak"), 0, 10_000, 0),
            "unstable_window_streak": clamp_int(quality_lock.get("unstable_window_streak"), 0, 10_000, 0),
            "min_stable_windows": clamp_int(quality_lock.get("min_stable_windows"), 0, 10_000, 0),
            "release_unstable_windows": clamp_int(quality_lock.get("release_unstable_windows"), 0, 10_000, 0),
            "min_realized_score": clamp_number(quality_lock.get("min_realized_score"), 0.0, 100.0, 0.0),
            "min_quality_receipts": clamp_int(quality_lock.get("min_quality_receipts"), 0, 10_000, 0),
            "max_insufficient_rate": round_to_places(clamp_number(quality_lock.get("max_insufficient_rate"), 0.0, 1.0, 1.0), 3),
        }
    })
}
