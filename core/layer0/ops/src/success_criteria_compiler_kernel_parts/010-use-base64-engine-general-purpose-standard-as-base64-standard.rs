// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

fn usage() {
    println!("success-criteria-compiler-kernel commands:");
    println!(
        "  protheus-ops success-criteria-compiler-kernel compile-rows --payload-base64=<json>"
    );
    println!(
        "  protheus-ops success-criteria-compiler-kernel compile-proposal --payload-base64=<json>"
    );
    println!("  protheus-ops success-criteria-compiler-kernel to-action-spec-rows --payload-base64=<json>");
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
        return serde_json::from_str::<Value>(&raw).map_err(|err| {
            format!("success_criteria_compiler_kernel_payload_decode_failed:{err}")
        });
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("success_criteria_compiler_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("success_criteria_compiler_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text).map_err(|err| {
            format!("success_criteria_compiler_kernel_payload_decode_failed:{err}")
        });
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_array<'a>(value: Option<&'a Value>) -> &'a Vec<Value> {
    value.and_then(Value::as_array).unwrap_or_else(|| {
        static EMPTY: OnceLock<Vec<Value>> = OnceLock::new();
        EMPTY.get_or_init(Vec::new)
    })
}

fn as_object<'a>(value: Option<&'a Value>) -> Option<&'a Map<String, Value>> {
    value.and_then(Value::as_object)
}

fn as_str(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn normalize_text(value: Option<&Value>) -> String {
    as_str(value)
}

fn normalize_spaces_str(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    let mut out = normalize_spaces_str(&normalize_text(value));
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn normalize_spaces(value: Option<&Value>) -> String {
    normalize_spaces_str(&normalize_text(value))
}

fn normalize_capability_key(value: Option<&Value>) -> String {
    normalize_spaces(value).to_ascii_lowercase()
}

fn outreach_capability_hint_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\b(opportunity|outreach|lead|sales|bizdev|revenue|freelance|contract|gig|external_intel|client|prospect)\b").unwrap()
    })
}

fn capability_allows_outreach(capability_key: &str) -> bool {
    if capability_key.is_empty() {
        return true;
    }
    if capability_key.starts_with("proposal:") {
        return outreach_capability_hint_re().is_match(capability_key);
    }
    true
}

fn remap_metric_for_capability(metric: &str, capability_key: &str) -> String {
    let norm_metric = normalize_spaces_str(metric).to_ascii_lowercase();
    if !capability_allows_outreach(capability_key)
        && (norm_metric == "reply_or_interview_count" || norm_metric == "outreach_artifact")
    {
        return "artifact_count".to_string();
    }
    if norm_metric.is_empty() {
        "execution_success".to_string()
    } else {
        norm_metric
    }
}

fn first_int_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(\d+)\b").unwrap())
}

fn parse_first_int(text: &str, fallback: i64) -> i64 {
    first_int_re()
        .captures(text)
        .and_then(|m| m.get(1))
        .and_then(|m| m.as_str().parse::<i64>().ok())
        .unwrap_or(fallback)
}

fn comparator_lte_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?:<=|≤|\bat most\b|\bwithin\b|\bunder\b|\bbelow\b|\bmax(?:imum)?\b|\bless than\b)",
        )
        .unwrap()
    })
}

fn comparator_gte_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?:>=|≥|\bat least\b|\bover\b|\babove\b|\bminimum\b|\bmin\b|\bmore than\b)")
            .unwrap()
    })
}

fn parse_comparator(text: &str, fallback: &str) -> &'static str {
    let lower = text.to_ascii_lowercase();
    if comparator_lte_re().is_match(&lower) {
        return "lte";
    }
    if comparator_gte_re().is_match(&lower) {
        return "gte";
    }
    if fallback == "lte" {
        "lte"
    } else {
        "gte"
    }
}

fn duration_limit_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(\d+(?:\.\d+)?)\s*(ms|msec|millisecond(?:s)?|s|sec|secs|second(?:s)?|m|min|mins|minute(?:s)?)",
        )
        .unwrap()
    })
}

fn parse_duration_limit_ms(text: &str) -> Option<i64> {
    let lower = text.to_ascii_lowercase();
    let captures = duration_limit_re().captures(&lower)?;
    let mut value = captures.get(1)?.as_str().parse::<f64>().ok()?;
    let unit = captures.get(2)?.as_str();
    if matches!(unit, "m" | "min" | "mins") || unit.starts_with("minute") {
        value *= 60.0 * 1000.0;
    } else if matches!(unit, "s" | "sec" | "secs") || unit.starts_with("second") {
        value *= 1000.0;
    }
    Some(value.round() as i64)
}

fn token_limit_re_a() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\d+(?:\.\d+)?)\s*(k|m)?\s*tokens?").unwrap())
}

fn token_limit_re_b() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"tokens?\s*(?:<=|≥|>=|≤|<|>|=|at most|at least|under|over|below|above|within|max(?:imum)?|min(?:imum)?)?\s*(\d+(?:\.\d+)?)(?:\s*(k|m))?")
            .unwrap()
    })
}

fn parse_token_limit(text: &str) -> Option<i64> {
    let lower = text.to_ascii_lowercase();
    let captures = token_limit_re_a()
        .captures(&lower)
        .or_else(|| token_limit_re_b().captures(&lower))?;
    let mut value = captures.get(1)?.as_str().parse::<f64>().ok()?;
    let suffix = captures.get(2).map(|m| m.as_str()).unwrap_or("");
    if suffix == "k" {
        value *= 1000.0;
    } else if suffix == "m" {
        value *= 1_000_000.0;
    }
    Some(value.round() as i64)
}

fn horizon_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"\b(\d+\s*(?:h|hr|hour|hours|d|day|days|w|week|weeks|min|mins|minute|minutes|run|runs))\b",
        )
        .unwrap()
    })
}
