// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

const NON_EXECUTING_COMMANDS: &[&str] = &["status", "verify", "health", "help"];
const INDEX_BYPASS_FLAGS: &[&str] = &[
    "bypass",
    "bypass-index",
    "allow-direct-file",
    "allow_full_scan",
    "allow-full-scan",
];
const DIRECT_READ_FLAGS: &[&str] = &[
    "file",
    "path",
    "full-file",
    "full_file",
    "direct-file",
    "direct_file",
];
const STALE_OVERRIDE_FLAGS: &[&str] = &["allow-stale", "allow_stale", "stale-ok", "stale_ok"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Policy {
    index_first_required: bool,
    max_burn_slo_tokens: i64,
    max_recall_top: i64,
    max_max_files: i64,
    max_expand_lines: i64,
    bootstrap_hydration_token_cap: i64,
    block_stale_override: bool,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            index_first_required: true,
            max_burn_slo_tokens: 200,
            max_recall_top: 50,
            max_max_files: 20,
            max_expand_lines: 300,
            bootstrap_hydration_token_cap: 48,
            block_stale_override: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ParsedCliArgs {
    positional: Vec<String>,
    flags: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct KernelPayload {
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    options: Option<Value>,
    #[serde(default)]
    scores: Option<Vec<Value>>,
    #[serde(default)]
    ids: Option<Vec<Value>>,
    #[serde(default)]
    annotation: Option<Value>,
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    validation: Option<Value>,
    #[serde(default)]
    context: Option<Value>,
    #[serde(default)]
    fallback: Option<String>,
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn receipt_envelope(kind: &str, ok: bool) -> Value {
    let ts = now_iso();
    json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string()
    })
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn usage() {
    println!("memory-policy-kernel commands:");
    println!("  protheus-ops memory-policy-kernel status");
    println!("  protheus-ops memory-policy-kernel parse-cli --payload-base64=<base64_json>");
    println!("  protheus-ops memory-policy-kernel command-name --payload-base64=<base64_json>");
    println!("  protheus-ops memory-policy-kernel validate --payload-base64=<base64_json>");
    println!("  protheus-ops memory-policy-kernel validate-ranking --payload-base64=<base64_json>");
    println!("  protheus-ops memory-policy-kernel validate-lensmap --payload-base64=<base64_json>");
    println!("  protheus-ops memory-policy-kernel severity-rank --payload-base64=<base64_json>");
    println!("  protheus-ops memory-policy-kernel guard-failure --payload-base64=<base64_json>");
}

fn load_payload(argv: &[String]) -> Result<KernelPayload, String> {
    if let Some(payload) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<KernelPayload>(&payload)
            .map_err(|err| format!("memory_policy_kernel_payload_decode_failed:{err}"));
    }
    if let Some(payload_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(payload_b64.as_bytes())
            .map_err(|err| format!("memory_policy_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("memory_policy_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<KernelPayload>(&text)
            .map_err(|err| format!("memory_policy_kernel_payload_decode_failed:{err}"));
    }
    Err("memory_policy_kernel_missing_payload".to_string())
}

fn parse_cli_args(args: &[String]) -> ParsedCliArgs {
    let mut positional = Vec::new();
    let mut flags = BTreeMap::new();
    for raw in args {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(body) = token.strip_prefix("--") {
            if let Some((key, value)) = body.split_once('=') {
                flags.insert(key.trim().to_string(), value.trim().to_string());
            } else {
                flags.insert(body.trim().to_string(), "1".to_string());
            }
        } else {
            positional.push(token.to_string());
        }
    }
    ParsedCliArgs { positional, flags }
}

fn read_numeric(flags: &BTreeMap<String, String>, names: &[&str], fallback: i64) -> i64 {
    for name in names {
        if let Some(raw) = flags.get(*name) {
            if let Ok(value) = raw.parse::<f64>() {
                if value.is_finite() {
                    return value.floor() as i64;
                }
            }
        }
    }
    fallback
}

fn read_boolean(flags: &BTreeMap<String, String>, names: &[&str], fallback: bool) -> bool {
    for name in names {
        if let Some(raw) = flags.get(*name) {
            let normalized = raw.trim().to_ascii_lowercase();
            if normalized.is_empty() || matches!(normalized.as_str(), "1" | "true" | "yes" | "on") {
                return true;
            }
            if matches!(normalized.as_str(), "0" | "false" | "no" | "off") {
                return false;
            }
        }
    }
    fallback
}

fn read_json_flag(flags: &BTreeMap<String, String>, names: &[&str]) -> Option<Value> {
    for name in names {
        if let Some(raw) = flags.get(*name) {
            return serde_json::from_str::<Value>(raw).ok();
        }
    }
    None
}

fn severity_rank_value(raw: &str) -> i64 {
    match raw.trim().to_ascii_lowercase().as_str() {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

fn value_as_text(value: &Value) -> String {
    match value {
        Value::String(inner) => inner.trim().to_string(),
        Value::Null => String::new(),
        _ => value.to_string().trim_matches('"').trim().to_string(),
    }
}

fn build_failure(reason_code: &str, details: Value) -> Value {
    json!({
        "ok": false,
        "type": "memory_policy_validation",
        "reason_code": reason_code,
        "details": details
    })
}

fn validate_descending_ranking(scores: &[Value], ids: &[Value]) -> Value {
    if scores.len() != ids.len() {
        return json!({ "ok": false, "reason_code": "ranking_shape_mismatch" });
    }
    for (idx, score_value) in scores.iter().enumerate() {
        let Some(score) = score_value.as_f64() else {
            return json!({ "ok": false, "reason_code": "ranking_non_finite_score" });
        };
        if !score.is_finite() {
            return json!({ "ok": false, "reason_code": "ranking_non_finite_score" });
        }
        if idx == 0 {
            continue;
        }
        let Some(prev_score) = scores[idx - 1].as_f64() else {
            return json!({ "ok": false, "reason_code": "ranking_non_finite_score" });
        };
        if score > prev_score {
            return json!({ "ok": false, "reason_code": "ranking_not_descending" });
        }
        if (score - prev_score).abs() < f64::EPSILON {
            let current_id = value_as_text(&ids[idx]);
            let previous_id = value_as_text(&ids[idx - 1]);
            if current_id < previous_id {
                return json!({ "ok": false, "reason_code": "ranking_tie_not_stable" });
            }
        }
    }
    json!({ "ok": true, "reason_code": "ranking_descending_stable" })
}
