// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::Path;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const TRIT_PAIN: i64 = -1;
const TRIT_UNKNOWN: i64 = 0;
const TRIT_OK: i64 = 1;

fn usage() {
    println!("ternary-belief-kernel commands:");
    println!("  protheus-ops ternary-belief-kernel evaluate --payload-base64=<json>");
    println!("  protheus-ops ternary-belief-kernel merge --payload-base64=<json>");
    println!("  protheus-ops ternary-belief-kernel serialize --payload-base64=<json>");
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
            .map_err(|err| format!("ternary_belief_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("ternary_belief_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("ternary_belief_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("ternary_belief_payload_decode_failed:{err}"));
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
        Some(v) => v.to_string(),
        None => String::new(),
    };
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn normalize_token(raw: Option<&Value>, max_len: usize) -> String {
    let lowered = clean_text(raw, max_len).to_ascii_lowercase();
    let mut out = String::new();
    let mut prev_us = false;
    for ch in lowered.chars() {
        let mapped = if ch.is_ascii_alphanumeric() { ch } else { '_' };
        if mapped == '_' {
            if prev_us || out.is_empty() {
                continue;
            }
            prev_us = true;
            out.push('_');
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
    value.and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

fn clamp_number(value: Option<&Value>, lo: f64, hi: f64, fallback: f64) -> f64 {
    as_f64(value).unwrap_or(fallback).clamp(lo, hi)
}

fn round_to(value: f64, digits: u32) -> f64 {
    let factor = 10_f64.powi(i32::try_from(digits).unwrap_or(4));
    (value * factor).round() / factor
}

fn parse_ts_ms(value: Option<&Value>) -> Option<i64> {
    let text = clean_text(value, 80);
    if text.is_empty() {
        return None;
    }
    chrono::DateTime::parse_from_rfc3339(&text)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn normalize_trit(value: Option<&Value>) -> i64 {
    match value {
        Some(Value::Number(n)) => {
            let num = n.as_f64().unwrap_or(0.0);
            if num > 0.0 {
                TRIT_OK
            } else if num < 0.0 {
                TRIT_PAIN
            } else {
                TRIT_UNKNOWN
            }
        }
        Some(Value::Bool(v)) => {
            if *v {
                TRIT_OK
            } else {
                TRIT_PAIN
            }
        }
        _ => {
            let token = normalize_token(value, 64);
            match token.as_str() {
                "ok" | "pass" | "allow" | "approved" | "healthy" | "up" | "true" | "success"
                | "green" | "ready" => TRIT_OK,
                "pain" | "fail" | "failed" | "error" | "blocked" | "deny" | "denied"
                | "critical" | "false" | "down" | "red" => TRIT_PAIN,
                _ => TRIT_UNKNOWN,
            }
        }
    }
}

fn trit_label(trit: i64) -> &'static str {
    match trit {
        TRIT_PAIN => "pain",
        TRIT_OK => "ok",
        _ => "unknown",
    }
}

fn normalize_weight(value: Option<&Value>, fallback: f64) -> f64 {
    let n = as_f64(value).unwrap_or(fallback);
    if !n.is_finite() || n <= 0.0 {
        fallback
    } else {
        n
    }
}

fn normalize_source(value: Option<&Value>, idx: usize) -> String {
    let text = clean_text(value, 120);
    if text.is_empty() {
        format!("signal_{}", idx + 1)
    } else {
        text
    }
}

fn source_trust_value(source_trust: Option<&Value>, source: &str, fallback: f64) -> f64 {
    let Some(map) = source_trust.and_then(Value::as_object) else {
        return fallback;
    };
    let direct = map
        .get(source)
        .or_else(|| map.get(&source.to_ascii_lowercase()));
    if let Some(value) = direct {
        if let Some(n) = as_f64(Some(value)) {
            return n;
        }
        if let Some(obj) = value.as_object() {
            if let Some(n) = as_f64(obj.get("trust")) {
                return n;
            }
            if let Some(n) = as_f64(obj.get("weight")) {
                return n;
            }
        }
    }
    fallback
}

fn signal_freshness_factor(signal_ts_ms: Option<i64>, now_ms: i64, half_life_hours: f64) -> f64 {
    let Some(signal_ms) = signal_ts_ms else {
        return 1.0;
    };
    let age_ms = (now_ms - signal_ms).max(0) as f64;
    let half_life_ms = half_life_hours.max(1.0) * 60.0 * 60.0 * 1000.0;
    let decay_power = age_ms / half_life_ms;
    (0.5_f64.powf(decay_power)).clamp(0.05, 1.0)
}

fn majority_trit(values: &[i64], weights: &[f64], tie_breaker: &str) -> i64 {
    let mut pain = 0.0;
    let mut unknown = 0.0;
    let mut ok = 0.0;
    for (idx, trit) in values.iter().enumerate() {
        let weight = *weights.get(idx).unwrap_or(&1.0);
        match *trit {
            TRIT_PAIN => pain += weight,
            TRIT_OK => ok += weight,
            _ => unknown += weight,
        }
    }
    if pain > ok && pain > unknown {
        return TRIT_PAIN;
    }
    if ok > pain && ok > unknown {
        return TRIT_OK;
    }
    if unknown > pain && unknown > ok {
        return TRIT_UNKNOWN;
    }
    match tie_breaker {
        "pain" => TRIT_PAIN,
        "ok" => TRIT_OK,
        "first_non_zero" => values
            .iter()
            .copied()
            .find(|v| *v != TRIT_UNKNOWN)
            .unwrap_or(TRIT_UNKNOWN),
        _ => TRIT_UNKNOWN,
    }
}

fn consensus_trit(values: &[i64]) -> i64 {
    let non_zero = values
        .iter()
        .copied()
        .filter(|v| *v != TRIT_UNKNOWN)
        .collect::<Vec<_>>();
    if non_zero.is_empty() {
        return TRIT_UNKNOWN;
    }
    let has_pain = non_zero.iter().any(|v| *v == TRIT_PAIN);
    let has_ok = non_zero.iter().any(|v| *v == TRIT_OK);
    if has_pain && has_ok {
        TRIT_UNKNOWN
    } else if has_pain {
        TRIT_PAIN
    } else {
        TRIT_OK
    }
}

fn propagate_trit(parent: i64, child: i64, mode: &str) -> i64 {
    match mode {
        "strict" => {
            if parent == TRIT_PAIN || child == TRIT_PAIN {
                TRIT_PAIN
            } else if parent == TRIT_OK && child == TRIT_OK {
                TRIT_OK
            } else {
                TRIT_UNKNOWN
            }
        }
        "permissive" => {
            if parent == TRIT_OK || child == TRIT_OK {
                TRIT_OK
            } else if parent == TRIT_PAIN && child == TRIT_PAIN {
                TRIT_PAIN
            } else {
                TRIT_UNKNOWN
            }
        }
        _ => {
            if child == TRIT_PAIN {
                TRIT_PAIN
            } else if parent == TRIT_PAIN && child == TRIT_UNKNOWN {
                TRIT_PAIN
            } else if parent == TRIT_OK && child == TRIT_OK {
                TRIT_OK
            } else if parent == TRIT_UNKNOWN && child == TRIT_OK {
                TRIT_OK
            } else {
                TRIT_UNKNOWN
            }
        }
    }
}

fn serialize_trit_vector(values: &[i64]) -> Value {
    let digits = values
        .iter()
        .map(|trit| match *trit {
            TRIT_PAIN => '-',
            TRIT_OK => '+',
            _ => '0',
        })
        .collect::<String>();
    let rows = values
        .iter()
        .map(|trit| match *trit {
            TRIT_PAIN => Value::String("-1".to_string()),
            TRIT_OK => Value::String("1".to_string()),
            _ => Value::String("0".to_string()),
        })
        .collect::<Vec<_>>();
    json!({
        "schema_id": "balanced_trit_vector",
        "schema_version": "1.0.0",
        "encoding": "balanced_ternary_sign",
        "digits": digits,
        "values": rows
    })
}

fn belief_summary(trit: i64, score: f64, confidence: f64, weight: f64) -> Value {
    json!({
        "trit": trit,
        "trit_label": trit_label(trit),
        "score": round_to(score, 4),
        "confidence": round_to(confidence, 4),
        "weight": round_to(weight, 4)
    })
}

fn classify_belief_trit(score: f64, positive_threshold: f64, negative_threshold: f64) -> i64 {
    if score >= positive_threshold {
        TRIT_OK
    } else if score <= negative_threshold {
        TRIT_PAIN
    } else {
        TRIT_UNKNOWN
    }
}
