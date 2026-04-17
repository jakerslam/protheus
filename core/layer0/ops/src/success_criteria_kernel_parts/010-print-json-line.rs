// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const ALL_KNOWN_METRICS: &[&str] = &[
    "execution_success",
    "postconditions_ok",
    "queue_outcome_logged",
    "retry_count",
    "timeout_count",
    "abort_count",
    "retry_backoff_ms",
    "artifact_count",
    "entries_count",
    "revenue_actions_count",
    "token_usage",
    "duration_ms",
    "outreach_artifact",
    "reply_or_interview_count",
];
const PROPOSAL_BASE_METRICS: &[&str] = &[
    "execution_success",
    "postconditions_ok",
    "queue_outcome_logged",
    "retry_count",
    "timeout_count",
    "abort_count",
    "retry_backoff_ms",
    "artifact_count",
    "entries_count",
    "revenue_actions_count",
    "token_usage",
    "duration_ms",
];
const OUTREACH_METRICS: &[&str] = &["outreach_artifact", "reply_or_interview_count"];
const CONTRACT_SAFE_BACKFILL_ROWS: &[(&str, &str, &str)] = &[
    (
        "contract_backfill",
        "execution_success",
        "execution success",
    ),
    (
        "contract_backfill",
        "postconditions_ok",
        "postconditions pass",
    ),
    (
        "contract_backfill",
        "queue_outcome_logged",
        "outcome receipt logged",
    ),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuccessCriteriaCompiledRow {
    pub source: String,
    pub metric: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvaluateCheck {
    index: u32,
    source: String,
    metric: String,
    target: String,
    evaluated: bool,
    pass: Option<bool>,
    reason: String,
    comparator: Option<String>,
    value: Option<Value>,
    threshold: Option<Value>,
    unit: Option<String>,
}

#[derive(Debug, Clone)]
struct EvaluationVerdict {
    evaluated: bool,
    pass: Option<bool>,
    reason: String,
    comparator: Option<String>,
    value: Option<Value>,
    target: Option<Value>,
    unit: Option<String>,
}

#[derive(Debug, Clone)]
struct CapabilityMetricContract {
    capability_key: Option<String>,
    enforced: bool,
    allowed_metrics: Option<HashSet<String>>,
}

#[derive(Debug, Clone, Deserialize)]
struct ParseRowsPayload {
    #[serde(default)]
    proposal: Option<Value>,
    #[serde(default)]
    capability_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct EvaluatePayload {
    #[serde(default)]
    proposal: Option<Value>,
    #[serde(default)]
    context: Option<Value>,
    #[serde(default)]
    policy: Option<Value>,
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn usage() {
    println!("success-criteria-kernel commands:");
    println!("  protheus-ops success-criteria-kernel status");
    println!("  protheus-ops success-criteria-kernel parse-rows --payload-base64=<base64_json>");
    println!("  protheus-ops success-criteria-kernel evaluate --payload-base64=<base64_json>");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": true,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
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
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn load_payload(argv: &[String]) -> Result<Value, String> {
    if let Some(payload) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&payload)
            .map_err(|err| format!("success_criteria_kernel_payload_decode_failed:{err}"));
    }
    if let Some(payload_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(payload_b64.as_bytes())
            .map_err(|err| format!("success_criteria_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("success_criteria_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("success_criteria_kernel_payload_decode_failed:{err}"));
    }
    if let Some(path) = lane_utils::parse_flag(argv, "payload-file", false) {
        let text = fs::read_to_string(path.trim())
            .map_err(|err| format!("success_criteria_kernel_payload_file_read_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("success_criteria_kernel_payload_decode_failed:{err}"));
    }
    Err("success_criteria_kernel_missing_payload".to_string())
}

fn normalize_text(raw: &str) -> String {
    raw.trim().to_string()
}

fn normalize_spaces(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn js_like_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(v) => v.trim().to_string(),
        _ => value.to_string().trim_matches('"').trim().to_string(),
    }
}

fn value_to_string(value: Option<&Value>) -> String {
    value.map(js_like_string).unwrap_or_default()
}

fn normalize_capability_key(raw: &str) -> String {
    normalize_spaces(raw).to_ascii_lowercase()
}

fn parse_first_int(text: &str, fallback: i64) -> i64 {
    let re = Regex::new(r"\b(\d+)\b").expect("valid parse_first_int regex");
    re.captures(text)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<i64>().ok())
        .unwrap_or(fallback)
}

fn parse_comparator(text: &str, fallback: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let lte = [
        "<=",
        "≤",
        "at most",
        "within",
        "under",
        "below",
        "maximum",
        "max",
        "less than",
    ];
    if lte.iter().any(|token| lower.contains(token)) {
        return "lte".to_string();
    }
    let gte = [
        ">=",
        "≥",
        "at least",
        "over",
        "above",
        "minimum",
        "min",
        "more than",
    ];
    if gte.iter().any(|token| lower.contains(token)) {
        return "gte".to_string();
    }
    fallback.to_string()
}

fn parse_duration_limit_ms(text: &str) -> Option<i64> {
    let re = Regex::new(
        r"(?i)(\d+(?:\.\d+)?)\s*(ms|msec|millisecond(?:s)?|s|sec|secs|second(?:s)?|m|min|mins|minute(?:s)?)",
    )
    .expect("valid duration regex");
    let caps = re.captures(text)?;
    let value = caps.get(1)?.as_str().parse::<f64>().ok()?;
    let unit = caps.get(2)?.as_str().to_ascii_lowercase();
    let scaled = if matches!(unit.as_str(), "m" | "min" | "mins") || unit.starts_with("minute") {
        value * 60_000.0
    } else if matches!(unit.as_str(), "s" | "sec" | "secs") || unit.starts_with("second") {
        value * 1_000.0
    } else {
        value
    };
    Some(scaled.round() as i64)
}

fn parse_token_limit(text: &str) -> Option<i64> {
    let re = Regex::new(
        r"(?i)(\d+(?:\.\d+)?)\s*(k|m)?\s*tokens?|tokens?\s*(?:<=|≥|>=|≤|<|>|=|at most|at least|under|over|below|above|within|max(?:imum)?|min(?:imum)?)?\s*(\d+(?:\.\d+)?)(?:\s*(k|m))?",
    )
    .expect("valid token regex");
    let caps = re.captures(text)?;
    let raw = caps.get(1).or_else(|| caps.get(3))?.as_str();
    let mut value = raw.parse::<f64>().ok()?;
    let suffix = caps
        .get(2)
        .or_else(|| caps.get(4))
        .map(|m| m.as_str().to_ascii_lowercase())
        .unwrap_or_default();
    match suffix.as_str() {
        "k" => value *= 1000.0,
        "m" => value *= 1_000_000.0,
        _ => {}
    }
    Some(value.round() as i64)
}

fn parse_horizon(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let re = Regex::new(
        r"\b(\d+\s*(?:h|hr|hour|hours|d|day|days|w|week|weeks|min|mins|minute|minutes|run|runs))\b",
    )
    .expect("valid horizon regex");
    if let Some(caps) = re.captures(&lower) {
        return normalize_spaces(caps.get(1).map(|m| m.as_str()).unwrap_or_default());
    }
    if lower.contains("next run") {
        return "next run".to_string();
    }
    if lower.contains("next 2 runs") {
        return "2 runs".to_string();
    }
    if lower.contains("24h") {
        return "24h".to_string();
    }
    if lower.contains("48h") {
        return "48h".to_string();
    }
    if lower.contains("7d") {
        return "7d".to_string();
    }
    String::new()
}

fn capability_allows_outreach(capability_key: &str) -> bool {
    if capability_key.is_empty() {
        return true;
    }
    if capability_key.starts_with("proposal:") {
        let re = Regex::new(
            r"\b(opportunity|outreach|lead|sales|bizdev|revenue|freelance|contract|gig|external_intel|client|prospect)\b",
        )
        .expect("valid outreach capability regex");
        return re.is_match(capability_key);
    }
    true
}

fn remap_metric_for_capability(metric: &str, capability_key: &str) -> String {
    let norm = normalize_spaces(metric).to_ascii_lowercase();
    if !capability_allows_outreach(capability_key)
        && matches!(
            norm.as_str(),
            "reply_or_interview_count" | "outreach_artifact"
        )
    {
        return "artifact_count".to_string();
    }
    if norm.is_empty() {
        "execution_success".to_string()
    } else {
        norm
    }
}

fn normalize_target(metric: &str, target_text: &str, horizon_text: &str) -> String {
    let text = normalize_spaces(&format!("{} {}", target_text, horizon_text)).to_ascii_lowercase();
    match metric {
        "execution_success" => "execution success".to_string(),
        "postconditions_ok" => "postconditions pass".to_string(),
        "queue_outcome_logged" => "outcome receipt logged".to_string(),
        "retry_count" => format!(
            "{}{} retries",
            if parse_comparator(&text, "gte") == "lte" {
                "<="
            } else {
                ">="
            },
            parse_first_int(&text, 1)
        ),
        "timeout_count" => format!(
            "{}{} timeouts",
            if parse_comparator(&text, "gte") == "lte" {
                "<="
            } else {
                ">="
            },
            parse_first_int(&text, 1)
        ),
        "abort_count" => format!(
            "{}{} aborts",
            if parse_comparator(&text, "gte") == "lte" {
                "<="
            } else {
                ">="
            },
            parse_first_int(&text, 1)
        ),
        "retry_backoff_ms" => format!(
            "retry backoff {}{}ms",
            if parse_comparator(&text, "lte") == "gte" {
                ">="
            } else {
                "<="
            },
            parse_duration_limit_ms(&text).unwrap_or(5_000)
        ),
        "artifact_count" => format!(
            "{}{} artifact",
            if parse_comparator(&text, "gte") == "lte" {
                "<="
            } else {
                ">="
            },
            parse_first_int(&text, 1)
        ),
        "outreach_artifact" => format!(
            "{}{} outreach artifact",
            if parse_comparator(&text, "gte") == "lte" {
                "<="
            } else {
                ">="
            },
            parse_first_int(&text, 1)
        ),
        "reply_or_interview_count" => format!(
            "{}{} reply/interview signal",
            if parse_comparator(&text, "gte") == "lte" {
                "<="
            } else {
                ">="
            },
            parse_first_int(&text, 1)
        ),
        "entries_count" => format!(
            "{}{} entries",
            if parse_comparator(&text, "gte") == "lte" {
                "<="
            } else {
                ">="
            },
            parse_first_int(&text, 1)
        ),
        "revenue_actions_count" => format!(
            "{}{} revenue actions",
            if parse_comparator(&text, "gte") == "lte" {
                "<="
            } else {
                ">="
            },
            parse_first_int(&text, 1)
        ),
        "token_usage" => format!(
            "tokens {}{}",
            if parse_comparator(&text, "lte") == "gte" {
                ">="
            } else {
                "<="
            },
            parse_token_limit(&text).unwrap_or(1200)
        ),
        "duration_ms" => format!(
            "duration {}{}ms",
            if parse_comparator(&text, "lte") == "gte" {
                ">="
            } else {
                "<="
            },
            parse_duration_limit_ms(&text).unwrap_or(15_000)
        ),
        _ => {
            let out = normalize_spaces(target_text);
            if out.is_empty() {
                "execution success".to_string()
            } else {
                out
            }
        }
    }
}
