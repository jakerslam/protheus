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

fn classify_metric(metric_text: &str, target_text: &str, source_text: &str) -> String {
    let metric = normalize_spaces(metric_text).to_ascii_lowercase();
    let text = normalize_spaces(&format!("{} {} {}", metric_text, target_text, source_text))
        .to_ascii_lowercase();
    let contains_any = |tokens: &[&str]| tokens.iter().any(|token| text.contains(token));

    if metric.is_empty() && contains_any(&["reply", "interview"]) {
        return "reply_or_interview_count".to_string();
    }
    if metric.is_empty()
        && text.contains("outreach")
        && contains_any(&["artifact", "draft", "offer", "proposal"])
    {
        return "outreach_artifact".to_string();
    }

    match metric.as_str() {
        "validation_metric" | "validation_check" | "verification_metric" | "verification_check" => {
            return "postconditions_ok".to_string()
        }
        "outreach_artifact" => return "outreach_artifact".to_string(),
        "reply_or_interview_count"
        | "reply_count"
        | "interview_count"
        | "outreach_reply_count"
        | "outreach_interview_count" => return "reply_or_interview_count".to_string(),
        "artifact_count"
        | "experiment_artifact"
        | "collector_success_runs"
        | "hypothesis_signal_lift"
        | "outreach_artifact_count"
        | "offer_draft_count"
        | "proposal_draft_count" => return "artifact_count".to_string(),
        "verification_checks_passed" | "postconditions_ok" => {
            return "postconditions_ok".to_string()
        }
        "collector_failure_streak" | "queue_outcome_logged" => {
            return "queue_outcome_logged".to_string()
        }
        "entries_count" => return "entries_count".to_string(),
        "revenue_actions_count" => return "revenue_actions_count".to_string(),
        "token_usage" => return "token_usage".to_string(),
        "duration_ms" => return "duration_ms".to_string(),
        "execution_success" => return "execution_success".to_string(),
        _ => {}
    }

    if contains_any(&["reply", "interview"]) {
        return "reply_or_interview_count".to_string();
    }
    if text.contains("outreach") && contains_any(&["artifact", "draft", "offer", "proposal"]) {
        return "outreach_artifact".to_string();
    }
    if contains_any(&[
        "artifact",
        "draft",
        "experiment",
        "patch",
        "plan",
        "deliverable",
    ]) {
        return "artifact_count".to_string();
    }
    if contains_any(&[
        "postcondition",
        "contract",
        "verify",
        "verification",
        "check pass",
        "checks pass",
    ]) {
        return "postconditions_ok".to_string();
    }
    if contains_any(&["receipt", "evidence", "queue outcome", "logged"]) {
        return "queue_outcome_logged".to_string();
    }
    if text.contains("revenue") {
        return "revenue_actions_count".to_string();
    }
    if contains_any(&["entries", "entry", "notes"]) {
        return "entries_count".to_string();
    }
    if contains_any(&["token", "tokens"]) {
        return "token_usage".to_string();
    }
    if contains_any(&[
        "latency",
        "duration",
        "time",
        " ms",
        "msec",
        "millisecond",
        "second",
        " sec",
        " min",
        "minute",
    ]) {
        return "duration_ms".to_string();
    }
    if contains_any(&[
        "execute",
        "executed",
        "execution",
        "run",
        "runnable",
        "success",
    ]) {
        return "execution_success".to_string();
    }
    "execution_success".to_string()
}

fn compile_success_criteria_rows(
    rows: Option<&Value>,
    source: &str,
) -> Vec<SuccessCriteriaCompiledRow> {
    let mut out = Vec::<SuccessCriteriaCompiledRow>::new();
    let source = normalize_text(source);
    let src = if source.is_empty() {
        "success_criteria".to_string()
    } else {
        source
    };
    let Some(Value::Array(entries)) = rows else {
        return out;
    };

    let mut seen = BTreeSet::<String>::new();
    for row in entries {
        let (metric_raw, target_raw, horizon_raw) = if let Some(raw) = row.as_str() {
            (String::new(), normalize_spaces(raw), String::new())
        } else if let Some(obj) = row.as_object() {
            (
                normalize_spaces(&value_to_string(obj.get("metric")).to_ascii_lowercase()),
                normalize_spaces(
                    &[
                        value_to_string(obj.get("target")),
                        value_to_string(obj.get("threshold")),
                        value_to_string(obj.get("description")),
                        value_to_string(obj.get("goal")),
                    ]
                    .into_iter()
                    .find(|value| !value.is_empty())
                    .unwrap_or_default(),
                ),
                normalize_spaces(
                    &[
                        value_to_string(obj.get("horizon")),
                        value_to_string(obj.get("window")),
                        value_to_string(obj.get("by")),
                    ]
                    .into_iter()
                    .find(|value| !value.is_empty())
                    .unwrap_or_default(),
                ),
            )
        } else {
            continue;
        };

        if metric_raw.is_empty() && target_raw.is_empty() && horizon_raw.is_empty() {
            continue;
        }
        let metric = classify_metric(&metric_raw, &target_raw, &src);
        let horizon = if horizon_raw.is_empty() {
            parse_horizon(&target_raw)
        } else {
            horizon_raw
        };
        let target = normalize_target(&metric, &target_raw, &horizon);
        let key = format!("{}|{}|{}|{}", src, metric, target, horizon).to_ascii_lowercase();
        if seen.insert(key) {
            out.push(SuccessCriteriaCompiledRow {
                source: src.clone(),
                metric,
                target,
            });
        }
    }
    out
}

fn compile_proposal_success_criteria(
    proposal: Option<&Value>,
    capability_key: &str,
) -> Vec<SuccessCriteriaCompiledRow> {
    let proposal = proposal.and_then(Value::as_object);
    let action_spec = proposal
        .and_then(|obj| obj.get("action_spec"))
        .and_then(Value::as_object);

    let mut compiled = Vec::<SuccessCriteriaCompiledRow>::new();
    compiled.extend(compile_success_criteria_rows(
        proposal.and_then(|obj| obj.get("success_criteria")),
        "success_criteria",
    ));
    compiled.extend(compile_success_criteria_rows(
        action_spec.and_then(|obj| obj.get("success_criteria")),
        "action_spec.success_criteria",
    ));
    compiled.extend(compile_success_criteria_rows(
        action_spec.and_then(|obj| obj.get("verify")),
        "action_spec.verify",
    ));
    compiled.extend(compile_success_criteria_rows(
        proposal.and_then(|obj| obj.get("validation")),
        "validation",
    ));

    if compiled.is_empty() {
        compiled.push(SuccessCriteriaCompiledRow {
            source: "compiler_fallback".to_string(),
            metric: "execution_success".to_string(),
            target: "execution success".to_string(),
        });
    }

    let mut out = Vec::<SuccessCriteriaCompiledRow>::new();
    let mut seen = BTreeSet::<String>::new();
    for row in compiled {
        let metric = remap_metric_for_capability(&row.metric, capability_key);
        let target = normalize_target(&metric, &row.target, "");
        let source = if row.source.trim().is_empty() {
            "success_criteria".to_string()
        } else {
            normalize_spaces(&row.source)
        };
        let key = format!("{}|{}|{}", source, metric, target).to_ascii_lowercase();
        if seen.insert(key) {
            out.push(SuccessCriteriaCompiledRow {
                source,
                metric,
                target,
            });
        }
    }
    out
}

pub fn parse_success_criteria_rows_from_proposal(
    proposal: Option<&Value>,
    capability_key: &str,
) -> Vec<SuccessCriteriaCompiledRow> {
    let compiled = compile_proposal_success_criteria(proposal, capability_key);
    let mut out = Vec::<SuccessCriteriaCompiledRow>::new();
    let mut seen = BTreeSet::<String>::new();
    for row in compiled {
        let metric = normalize_spaces(&row.metric).to_ascii_lowercase();
        let target = normalize_spaces(&row.target);
        if metric.is_empty() && target.is_empty() {
            continue;
        }
        let key = format!("{}|{}", metric, target).to_ascii_lowercase();
        if seen.insert(key) {
            out.push(SuccessCriteriaCompiledRow {
                source: if row.source.trim().is_empty() {
                    "compiled".to_string()
                } else {
                    row.source
                },
                metric: if metric.is_empty() {
                    "execution_success".to_string()
                } else {
                    metric
                },
                target: if target.is_empty() {
                    "execution success".to_string()
                } else {
                    target
                },
            });
        }
    }
    out
}

fn capability_metric_contract(capability_key: &str) -> CapabilityMetricContract {
    let key = normalize_capability_key(capability_key);
    if key.is_empty() {
        return CapabilityMetricContract {
            capability_key: None,
            enforced: false,
            allowed_metrics: None,
        };
    }
    if key.starts_with("actuation:") {
        return CapabilityMetricContract {
            capability_key: Some(key),
            enforced: true,
            allowed_metrics: Some(ALL_KNOWN_METRICS.iter().map(|v| v.to_string()).collect()),
        };
    }
    if key.starts_with("proposal:") {
        let mut allowed: HashSet<String> = PROPOSAL_BASE_METRICS
            .iter()
            .map(|v| v.to_string())
            .collect();
        if capability_allows_outreach(&key) {
            for metric in OUTREACH_METRICS {
                allowed.insert((*metric).to_string());
            }
        }
        return CapabilityMetricContract {
            capability_key: Some(key),
            enforced: true,
            allowed_metrics: Some(allowed),
        };
    }
    CapabilityMetricContract {
        capability_key: Some(key),
        enforced: true,
        allowed_metrics: Some(ALL_KNOWN_METRICS.iter().map(|v| v.to_string()).collect()),
    }
}

fn metric_allowed_by_contract(contract: &CapabilityMetricContract, metric_name: &str) -> bool {
    let Some(allowed) = contract.allowed_metrics.as_ref() else {
        return false;
    };
    let norm = metric_name.to_ascii_lowercase().replace([' ', '-'], "_");
    !norm.is_empty() && allowed.contains(&norm)
}

fn backfill_contract_safe_rows(
    rows: &[SuccessCriteriaCompiledRow],
    contract: &CapabilityMetricContract,
    min_count: i64,
) -> (Vec<SuccessCriteriaCompiledRow>, i64) {
    let mut out = rows.to_vec();
    if min_count <= 0 || !contract.enforced || contract.allowed_metrics.is_none() {
        return (out, 0);
    }
    let mut seen = BTreeSet::<String>::new();
    for row in &out {
        seen.insert(format!(
            "{}|{}",
            row.metric.to_ascii_lowercase().replace([' ', '-'], "_"),
            row.target.to_ascii_lowercase()
        ));
    }
    let mut supported_count = out
        .iter()
        .filter(|row| metric_allowed_by_contract(contract, &row.metric))
        .count() as i64;
    let mut added = 0i64;
    for (source, metric, target) in CONTRACT_SAFE_BACKFILL_ROWS {
        if supported_count >= min_count {
            break;
        }
        if !metric_allowed_by_contract(contract, metric) {
            continue;
        }
        let key = format!(
            "{}|{}",
            metric.to_ascii_lowercase().replace([' ', '-'], "_"),
            target.to_ascii_lowercase()
        );
        if seen.insert(key) {
            out.push(SuccessCriteriaCompiledRow {
                source: (*source).to_string(),
                metric: (*metric).to_string(),
                target: (*target).to_string(),
            });
            supported_count += 1;
            added += 1;
        }
    }
    (out, added)
}

fn as_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(v)) => v.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn compare_numeric(value: Option<f64>, threshold: Option<f64>, comparator: &str) -> Option<bool> {
    let value = value?;
    let threshold = threshold?;
    if comparator == "gte" {
        Some(value >= threshold)
    } else {
        Some(value <= threshold)
    }
}

fn bool_verdict(reason: &str, pass: bool, value: Value, target: Value) -> EvaluationVerdict {
    EvaluationVerdict {
        evaluated: true,
        pass: Some(pass),
        reason: reason.to_string(),
        comparator: None,
        value: Some(value),
        target: Some(target),
        unit: None,
    }
}

fn read_numeric_metric(context: &Value, keys: &[&str]) -> Option<f64> {
    let top = context.as_object()?;
    let metric_values = top.get("metric_values").and_then(Value::as_object);
    let dod_diff = top.get("dod_diff").and_then(Value::as_object);
    for key in keys {
        if let Some(value) = metric_values
            .and_then(|map| map.get(*key))
            .and_then(|v| as_f64(Some(v)))
        {
            return Some(value);
        }
        if let Some(value) = top.get(*key).and_then(|v| as_f64(Some(v))) {
            return Some(value);
        }
        if let Some(value) = dod_diff
            .and_then(|map| map.get(*key))
            .and_then(|v| as_f64(Some(v)))
        {
            return Some(value);
        }
    }
    None
}

fn evaluate_row(row: &SuccessCriteriaCompiledRow, context: &Value) -> EvaluationVerdict {
    let metric = row.metric.to_ascii_lowercase();
    let target = row.target.clone();
    let text = format!("{} {}", metric, target).to_ascii_lowercase();
    let text_words = text.replace(['_', '-'], " ");
    let metric_norm = metric.replace([' ', '-'], "_");
    let top = context.as_object();
    let outcome = top
        .and_then(|map| map.get("outcome"))
        .map(js_like_string)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let exec_ok = top
        .and_then(|map| map.get("exec_ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let dod_passed = top
        .and_then(|map| map.get("dod_passed"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let postconditions_ok = top
        .and_then(|map| map.get("postconditions_ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let queue_outcome_logged = top
        .and_then(|map| map.get("queue_outcome_logged"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let duration_ms = read_numeric_metric(context, &["duration_ms"]);
    let token_usage = top
        .and_then(|map| map.get("token_usage"))
        .and_then(Value::as_object);
    let effective_tokens = token_usage
        .and_then(|map| map.get("effective_tokens").and_then(|v| as_f64(Some(v))))
        .or_else(|| {
            token_usage.and_then(|map| map.get("actual_total_tokens").and_then(|v| as_f64(Some(v))))
        })
        .or_else(|| {
            token_usage.and_then(|map| map.get("estimated_tokens").and_then(|v| as_f64(Some(v))))
        });
    let artifacts_delta = read_numeric_metric(
        context,
        &["artifacts_delta", "artifacts_count", "artifact_count"],
    );
    let entries_delta = read_numeric_metric(context, &["entries_delta", "entries_count"]);
    let revenue_delta =
        read_numeric_metric(context, &["revenue_actions_delta", "revenue_actions_count"]);
    let has_any = |tokens: &[&str]| tokens.iter().any(|token| text_words.contains(token));

    let numeric_verdict = |reason: &str,
                           comparator: &str,
                           value: Option<f64>,
                           threshold: Option<f64>,
                           unavailable: &str,
                           unit: Option<&str>| {
        let pass = compare_numeric(value, threshold, comparator);
        match pass {
            Some(v) => EvaluationVerdict {
                evaluated: true,
                pass: Some(v),
                reason: reason.to_string(),
                comparator: Some(comparator.to_string()),
                value: value.map(|n| json!(n)),
                target: threshold.map(|n| json!(n)),
                unit: unit.map(|u| u.to_string()),
            },
            None => EvaluationVerdict {
                evaluated: false,
                pass: None,
                reason: unavailable.to_string(),
                comparator: None,
                value: None,
                target: None,
                unit: None,
            },
        }
    };

    match metric_norm.as_str() {
        "execution_success" => {
            return bool_verdict(
                "requires_execution_success",
                exec_ok,
                Value::Bool(exec_ok),
                Value::Bool(true),
            )
        }
        "postconditions_ok" => {
            return bool_verdict(
                "requires_postconditions_pass",
                postconditions_ok,
                Value::Bool(postconditions_ok),
                Value::Bool(true),
            )
        }
        "queue_outcome_logged" => {
            return bool_verdict(
                "requires_receipt_or_outcome_log",
                queue_outcome_logged,
                Value::Bool(queue_outcome_logged),
                Value::Bool(true),
            )
        }
        "artifact_count" => {
            let threshold = parse_first_int(&text, 1) as f64;
            let comparator = parse_comparator(&text, "gte");
            return numeric_verdict(
                "artifact_delta_check",
                &comparator,
                artifacts_delta,
                Some(threshold),
                "artifact_delta_unavailable",
                None,
            );
        }
        "entries_count" => {
            let threshold = parse_first_int(&text, 1) as f64;
            let comparator = parse_comparator(&text, "gte");
            return numeric_verdict(
                "entry_delta_check",
                &comparator,
                entries_delta,
                Some(threshold),
                "entry_delta_unavailable",
                None,
            );
        }
        "revenue_actions_count" => {
            let threshold = parse_first_int(&text, 1) as f64;
            let comparator = parse_comparator(&text, "gte");
            return numeric_verdict(
                "revenue_delta_check",
                &comparator,
                revenue_delta,
                Some(threshold),
                "revenue_delta_unavailable",
                None,
            );
        }
        "token_usage" => {
            let limit = parse_token_limit(&text).map(|v| v as f64);
            if limit.is_none() {
                return EvaluationVerdict {
                    evaluated: false,
                    pass: None,
                    reason: "token_limit_missing".to_string(),
                    comparator: None,
                    value: None,
                    target: None,
                    unit: None,
                };
            }
            let comparator = parse_comparator(&text, "lte");
            return numeric_verdict(
                "token_limit_check",
                &comparator,
                effective_tokens,
                limit,
                "token_usage_unavailable",
                None,
            );
        }
        "duration_ms" => {
            let limit = parse_duration_limit_ms(&text).map(|v| v as f64);
            if limit.is_none() {
                return EvaluationVerdict {
                    evaluated: false,
                    pass: None,
                    reason: "duration_limit_missing".to_string(),
                    comparator: None,
                    value: None,
                    target: None,
                    unit: None,
                };
            }
            let comparator = parse_comparator(&text, "lte");
            return numeric_verdict(
                "duration_limit_check",
                &comparator,
                duration_ms,
                limit,
                "duration_unavailable",
                Some("ms"),
            );
        }
        "outreach_artifact" => {
            let threshold = parse_first_int(&text, 1) as f64;
            let comparator = parse_comparator(&text, "gte");
            let value = read_numeric_metric(
                context,
                &[
                    "outreach_artifact",
                    "outreach_artifact_count",
                    "offer_draft_count",
                    "proposal_draft_count",
                ],
            )
            .or(artifacts_delta);
            return numeric_verdict(
                "outreach_artifact_check",
                &comparator,
                value,
                Some(threshold),
                "outreach_artifact_unavailable",
                None,
            );
        }
        "reply_or_interview_count" => {
            let threshold = parse_first_int(&text, 1) as f64;
            let comparator = parse_comparator(&text, "gte");
            let value = read_numeric_metric(context, &["reply_or_interview_count"]).or_else(|| {
                let reply = read_numeric_metric(context, &["reply_count", "outreach_reply_count"])
                    .unwrap_or(0.0);
                let interview =
                    read_numeric_metric(context, &["interview_count", "outreach_interview_count"])
                        .unwrap_or(0.0);
                if reply > 0.0 || interview > 0.0 {
                    Some(reply + interview)
                } else {
                    None
                }
            });
            return numeric_verdict(
                "reply_or_interview_count_check",
                &comparator,
                value,
                Some(threshold),
                "reply_or_interview_count_unavailable",
                None,
            );
        }
        _ => {}
    }

    if has_any(&[
        "ship",
        "shipped",
        "publish",
        "posted",
        "merged",
        "applied",
        "delivered",
    ]) {
        return EvaluationVerdict {
            evaluated: true,
            pass: Some(outcome == "shipped"),
            reason: "requires_shipped_outcome".to_string(),
            comparator: None,
            value: Some(Value::String(outcome.clone())),
            target: Some(Value::String("shipped".to_string())),
            unit: None,
        };
    }
    if has_any(&["no change", "nochange"]) {
        return EvaluationVerdict {
            evaluated: true,
            pass: Some(outcome == "no_change"),
            reason: "requires_no_change_outcome".to_string(),
            comparator: None,
            value: Some(Value::String(outcome.clone())),
            target: Some(Value::String("no_change".to_string())),
            unit: None,
        };
    }
    if has_any(&["revert", "rollback", "undo"]) && has_any(&["no", "without", "avoid", "prevent"]) {
        return EvaluationVerdict {
            evaluated: true,
            pass: Some(outcome != "reverted"),
            reason: "requires_non_reverted_outcome".to_string(),
            comparator: None,
            value: Some(Value::String(outcome.clone())),
            target: Some(Value::String("!=reverted".to_string())),
            unit: None,
        };
    }
    if has_any(&[
        "execute",
        "executed",
        "execution",
        "run",
        "runnable",
        "exit 0",
        "success",
    ]) {
        return bool_verdict(
            "requires_execution_success",
            exec_ok,
            Value::Bool(exec_ok),
            Value::Bool(true),
        );
    }
    if has_any(&[
        "postcondition",
        "contract",
        "verify",
        "verification",
        "validated",
        "check pass",
        "checks pass",
    ]) {
        return bool_verdict(
            "requires_postconditions_pass",
            postconditions_ok,
            Value::Bool(postconditions_ok),
            Value::Bool(true),
        );
    }
    if has_any(&["dod", "impact", "delta"]) {
        return bool_verdict(
            "requires_dod_pass",
            dod_passed,
            Value::Bool(dod_passed),
            Value::Bool(true),
        );
    }
    if has_any(&["artifact", "artifacts"]) {
        let threshold = parse_first_int(&text, 1) as f64;
        let comparator = parse_comparator(&text, "gte");
        return numeric_verdict(
            "artifact_delta_check",
            &comparator,
            artifacts_delta,
            Some(threshold),
            "artifact_delta_unavailable",
            None,
        );
    }
    if has_any(&["entries", "entry", "notes"]) {
        let threshold = parse_first_int(&text, 1) as f64;
        let comparator = parse_comparator(&text, "gte");
        return numeric_verdict(
            "entry_delta_check",
            &comparator,
            entries_delta,
            Some(threshold),
            "entry_delta_unavailable",
            None,
        );
    }
    if has_any(&["revenue"]) {
        let threshold = parse_first_int(&text, 1) as f64;
        let comparator = parse_comparator(&text, "gte");
        return numeric_verdict(
            "revenue_delta_check",
            &comparator,
            revenue_delta,
            Some(threshold),
            "revenue_delta_unavailable",
            None,
        );
    }
    if metric_norm == "outreach_artifact"
        || (has_any(&["outreach"]) && has_any(&["artifact", "draft", "offer", "proposal"]))
        || (has_any(&["draft", "offer", "proposal"])
            && has_any(&[
                "build",
                "generate",
                "generated",
                "create",
                "created",
                "artifact",
            ]))
    {
        let threshold = parse_first_int(&text, 1) as f64;
        let comparator = parse_comparator(&text, "gte");
        let value = read_numeric_metric(
            context,
            &[
                "outreach_artifact",
                "outreach_artifact_count",
                "offer_draft_count",
                "proposal_draft_count",
            ],
        )
        .or(artifacts_delta);
        return numeric_verdict(
            "outreach_artifact_check",
            &comparator,
            value,
            Some(threshold),
            "outreach_artifact_unavailable",
            None,
        );
    }
    if metric_norm == "reply_or_interview_count"
        || (has_any(&["reply", "interview"]) && has_any(&["count", "signal", "response", "kpi"]))
    {
        let threshold = parse_first_int(&text, 1) as f64;
        let comparator = parse_comparator(&text, "gte");
        let value = read_numeric_metric(context, &["reply_or_interview_count"]).or_else(|| {
            let reply = read_numeric_metric(context, &["reply_count", "outreach_reply_count"])
                .unwrap_or(0.0);
            let interview =
                read_numeric_metric(context, &["interview_count", "outreach_interview_count"])
                    .unwrap_or(0.0);
            if reply > 0.0 || interview > 0.0 {
                Some(reply + interview)
            } else {
                None
            }
        });
        return numeric_verdict(
            "reply_or_interview_count_check",
            &comparator,
            value,
            Some(threshold),
            "reply_or_interview_count_unavailable",
            None,
        );
    }
    if has_any(&["token", "tokens"]) {
        let limit = parse_token_limit(&text).map(|v| v as f64);
        if limit.is_none() {
            return EvaluationVerdict {
                evaluated: false,
                pass: None,
                reason: "token_limit_missing".to_string(),
                comparator: None,
                value: None,
                target: None,
                unit: None,
            };
        }
        let comparator = parse_comparator(&text, "lte");
        return numeric_verdict(
            "token_limit_check",
            &comparator,
            effective_tokens,
            limit,
            "token_usage_unavailable",
            None,
        );
    }
    if has_any(&[
        "latency",
        "duration",
        "time",
        " ms",
        "msec",
        "millisecond",
        "second",
        " sec",
        " min",
        "minute",
    ]) {
        let limit = parse_duration_limit_ms(&text).map(|v| v as f64);
        if limit.is_none() {
            return EvaluationVerdict {
                evaluated: false,
                pass: None,
                reason: "duration_limit_missing".to_string(),
                comparator: None,
                value: None,
                target: None,
                unit: None,
            };
        }
        let comparator = parse_comparator(&text, "lte");
        return numeric_verdict(
            "duration_limit_check",
            &comparator,
            duration_ms,
            limit,
            "duration_unavailable",
            Some("ms"),
        );
    }
    if has_any(&["receipt", "evidence", "queue outcome", "logged"]) {
        return bool_verdict(
            "requires_receipt_or_outcome_log",
            queue_outcome_logged,
            Value::Bool(queue_outcome_logged),
            Value::Bool(true),
        );
    }

    EvaluationVerdict {
        evaluated: false,
        pass: None,
        reason: "unsupported_metric".to_string(),
        comparator: None,
        value: None,
        target: None,
        unit: None,
    }
}

pub fn evaluate_success_criteria_value(
    proposal: Option<&Value>,
    context: Option<&Value>,
    policy: Option<&Value>,
) -> Value {
    let policy = policy.unwrap_or(&Value::Null);
    let context = context.unwrap_or(&Value::Null);
    let policy_obj = policy.as_object();
    let context_obj = context.as_object();
    let capability_key = policy_obj
        .and_then(|map| map.get("capability_key"))
        .map(js_like_string)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            context_obj
                .and_then(|map| map.get("capability_key"))
                .map(js_like_string)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default();
    let required = policy_obj
        .and_then(|map| map.get("required"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let min_count = policy_obj
        .and_then(|map| map.get("min_count"))
        .and_then(|v| as_f64(Some(v)))
        .map(|v| v.floor() as i64)
        .unwrap_or(1)
        .clamp(0, 10);
    let contract = capability_metric_contract(&capability_key);
    let enable_contract_backfill = policy_obj
        .and_then(|map| map.get("enable_contract_backfill"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let fail_on_contract_violation = policy_obj
        .and_then(|map| map.get("fail_on_contract_violation"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let enforce_contract = contract.enforced
        && policy_obj
            .and_then(|map| map.get("enforce_contract"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
    let enforce_min_supported = contract.enforced
        && policy_obj
            .and_then(|map| map.get("enforce_min_supported"))
            .and_then(Value::as_bool)
            .unwrap_or(true);

    let rows_raw = parse_success_criteria_rows_from_proposal(proposal, &capability_key);
    let (rows, contract_backfill_count) = if enable_contract_backfill {
        backfill_contract_safe_rows(&rows_raw, &contract, min_count)
    } else {
        (rows_raw, 0)
    };

    let mut results = Vec::<EvaluateCheck>::new();
    for (idx, row) in rows.iter().enumerate() {
        let metric_norm = row.metric.to_ascii_lowercase().replace([' ', '-'], "_");
        let blocked_by_contract = enforce_contract
            && !metric_norm.is_empty()
            && contract
                .allowed_metrics
                .as_ref()
                .map(|allowed| !allowed.contains(&metric_norm))
                .unwrap_or(false);
        let verdict = if blocked_by_contract {
            EvaluationVerdict {
                evaluated: false,
                pass: None,
                reason: "metric_not_allowed_for_capability".to_string(),
                comparator: None,
                value: None,
                target: None,
                unit: None,
            }
        } else {
            evaluate_row(row, context)
        };
        results.push(EvaluateCheck {
            index: (idx + 1) as u32,
            source: row.source.clone(),
            metric: row.metric.clone(),
            target: row.target.chars().take(180).collect(),
            evaluated: verdict.evaluated,
            pass: verdict.pass,
            reason: verdict.reason,
            comparator: verdict.comparator,
            value: verdict.value,
            threshold: verdict.target,
            unit: verdict.unit,
        });
    }

    let evaluated_count = results.iter().filter(|row| row.evaluated).count() as i64;
    let passed_count = results.iter().filter(|row| row.pass == Some(true)).count() as i64;
    let failed_rows = results
        .iter()
        .filter(|row| row.pass == Some(false))
        .collect::<Vec<_>>();
    let failed_count = failed_rows.len() as i64;
    let unknown_count = results.len() as i64 - evaluated_count;
    let unsupported_count = results
        .iter()
        .filter(|row| row.reason == "unsupported_metric")
        .count() as i64;
    let contract_not_allowed_count = results
        .iter()
        .filter(|row| row.reason == "metric_not_allowed_for_capability")
        .count() as i64;
    let structurally_supported_count =
        (results.len() as i64 - unsupported_count - contract_not_allowed_count).max(0);

    let mut passed = true;
    let mut primary_failure: Option<String> = None;
    if required {
        if rows.len() < min_count as usize {
            passed = false;
            primary_failure = Some("success_criteria_count_below_min".to_string());
        } else if passed_count < min_count {
            passed = false;
            primary_failure = Some(if let Some(first) = failed_rows.first() {
                format!("success_criteria_failed:{}", first.reason)
            } else {
                "success_criteria_pass_count_below_min".to_string()
            });
        } else if failed_count > 0 {
            passed = false;
            if let Some(first) = failed_rows.first() {
                primary_failure = Some(format!("success_criteria_failed:{}", first.reason));
            }
        }
    } else if failed_count > 0 {
        passed = false;
        if let Some(first) = failed_rows.first() {
            primary_failure = Some(format!("success_criteria_failed:{}", first.reason));
        }
    }

    if enforce_contract && fail_on_contract_violation && contract_not_allowed_count > 0 {
        passed = false;
        primary_failure =
            Some("success_criteria_failed:metric_not_allowed_for_capability".to_string());
    } else if enforce_min_supported && required && structurally_supported_count < min_count {
        passed = false;
        primary_failure =
            Some("success_criteria_failed:insufficient_supported_metrics".to_string());
    }

    let violation_rows = results
        .iter()
        .filter(|row| {
            row.reason == "unsupported_metric" || row.reason == "metric_not_allowed_for_capability"
        })
        .take(12)
        .map(|row| {
            json!({
                "index": row.index,
                "metric": row.metric,
                "reason": row.reason,
            })
        })
        .collect::<Vec<_>>();

    let allowed_metrics = contract
        .allowed_metrics
        .as_ref()
        .map(|set| {
            let mut rows = set.iter().cloned().collect::<Vec<_>>();
            rows.sort();
            rows
        })
        .unwrap_or_default();

    json!({
        "required": required,
        "min_count": min_count,
        "total_count": rows.len(),
        "evaluated_count": evaluated_count,
        "passed_count": passed_count,
        "failed_count": failed_count,
        "unknown_count": unknown_count,
        "unsupported_count": unsupported_count,
        "contract_not_allowed_count": contract_not_allowed_count,
        "structurally_supported_count": structurally_supported_count,
        "contract_backfill_count": contract_backfill_count,
        "pass_rate": if evaluated_count > 0 { Some(((passed_count as f64 / evaluated_count as f64) * 1000.0).round() / 1000.0) } else { None },
        "passed": passed,
        "primary_failure": primary_failure,
        "contract": {
            "capability_key": contract.capability_key,
            "enforced": enforce_contract,
            "fail_on_violation": fail_on_contract_violation,
            "min_supported_enforced": enforce_min_supported,
            "backfill_enabled": enable_contract_backfill,
            "backfill_count": contract_backfill_count,
            "allowed_metrics": allowed_metrics,
            "unsupported_count": unsupported_count,
            "not_allowed_count": contract_not_allowed_count,
            "structurally_supported_count": structurally_supported_count,
            "violation_count": violation_rows.len(),
            "violations": violation_rows,
        },
        "checks": results.into_iter().take(12).collect::<Vec<_>>(),
    })
}

pub fn run(_root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    match command.as_str() {
        "status" => {
            print_json_line(&cli_receipt(
                "success_criteria_kernel_status",
                json!({
                    "domain": "success-criteria-kernel",
                    "commands": ["status", "parse-rows", "evaluate"],
                }),
            ));
            0
        }
        "parse-rows" => {
            let payload = match load_payload(argv) {
                Ok(payload) => payload,
                Err(err) => {
                    print_json_line(&cli_error("success_criteria_kernel_parse_rows_error", &err));
                    return 1;
                }
            };
            let input = match serde_json::from_value::<ParseRowsPayload>(payload) {
                Ok(value) => value,
                Err(err) => {
                    print_json_line(&cli_error(
                        "success_criteria_kernel_parse_rows_error",
                        &format!("success_criteria_kernel_parse_rows_payload_invalid:{err}"),
                    ));
                    return 1;
                }
            };
            let rows = parse_success_criteria_rows_from_proposal(
                input.proposal.as_ref(),
                &input.capability_key.unwrap_or_default(),
            );
            print_json_line(&cli_receipt(
                "success_criteria_kernel_parse_rows",
                json!({ "rows": rows }),
            ));
            0
        }
        "evaluate" => {
            let payload = match load_payload(argv) {
                Ok(payload) => payload,
                Err(err) => {
                    print_json_line(&cli_error("success_criteria_kernel_evaluate_error", &err));
                    return 1;
                }
            };
            let input = match serde_json::from_value::<EvaluatePayload>(payload) {
                Ok(value) => value,
                Err(err) => {
                    print_json_line(&cli_error(
                        "success_criteria_kernel_evaluate_error",
                        &format!("success_criteria_kernel_evaluate_payload_invalid:{err}"),
                    ));
                    return 1;
                }
            };
            let result = evaluate_success_criteria_value(
                input.proposal.as_ref(),
                input.context.as_ref(),
                input.policy.as_ref(),
            );
            print_json_line(&cli_receipt(
                "success_criteria_kernel_evaluate",
                json!({ "result": result }),
            ));
            0
        }
        _ => {
            usage();
            print_json_line(&cli_error(
                "success_criteria_kernel_error",
                &format!("success_criteria_kernel_unknown_command:{command}"),
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_success_criteria_rows_supports_capability_remap_and_dedupe() {
        let proposal = json!({
            "success_criteria": [
                {"metric": "reply_or_interview_count", "target": ">=2"},
                {"metric": "reply_or_interview_count", "target": ">=2"},
                "postconditions pass within 24h"
            ],
            "action_spec": {
                "verify": ["receipt logged"]
            }
        });
        let rows =
            parse_success_criteria_rows_from_proposal(Some(&proposal), "proposal:internal_patch");
        assert!(rows.iter().any(|row| row.metric == "artifact_count"));
        assert!(rows.iter().any(|row| row.metric == "postconditions_ok"));
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn evaluate_success_criteria_fails_closed_on_numeric_overrun() {
        let proposal = json!({
            "success_criteria": [
                {"metric": "artifact_count", "target": ">=1 artifact"},
                {"metric": "token_usage", "target": "tokens <= 500"}
            ]
        });
        let context = json!({
            "exec_ok": true,
            "postconditions_ok": true,
            "queue_outcome_logged": true,
            "dod_diff": {
                "artifacts_delta": 2,
                "entries_delta": 0,
                "revenue_actions_delta": 0
            },
            "token_usage": {
                "effective_tokens": 900
            }
        });
        let policy = json!({
            "capability_key": "proposal:internal_patch",
            "required": true,
            "min_count": 2
        });
        let out = evaluate_success_criteria_value(Some(&proposal), Some(&context), Some(&policy));
        assert_eq!(out.get("passed").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("primary_failure").and_then(Value::as_str),
            Some("success_criteria_failed:token_limit_check")
        );
        assert_eq!(
            out.get("contract_not_allowed_count")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(out.get("passed_count").and_then(Value::as_i64), Some(1));
    }
}
