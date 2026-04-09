// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

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
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
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
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = receipt_envelope(kind, ok);
    out["payload"] = payload;
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let mut out = receipt_envelope(kind, false);
    out["error"] = Value::String(error.to_string());
    out["fail_closed"] = Value::Bool(true);
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
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

fn validate_lensmap_annotation(annotation: Option<&Value>) -> Value {
    let Some(annotation) = annotation else {
        return json!({ "ok": true, "reason_code": "lensmap_annotation_not_provided" });
    };
    let Some(obj) = annotation.as_object() else {
        return json!({ "ok": false, "reason_code": "lensmap_annotation_invalid_type" });
    };

    let node_id = obj
        .get("node_id")
        .or_else(|| obj.get("nodeId"))
        .map(value_as_text)
        .unwrap_or_default();
    if node_id.is_empty() {
        return json!({ "ok": false, "reason_code": "lensmap_annotation_missing_node_id" });
    }

    let tags = obj
        .get("tags")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let jots = obj
        .get("jots")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if tags.is_empty() && jots.is_empty() {
        return json!({ "ok": false, "reason_code": "lensmap_annotation_missing_tags_or_jots" });
    }

    let mut seen = BTreeSet::<String>::new();
    for tag in tags {
        let normalized = value_as_text(&tag).to_ascii_lowercase();
        if normalized.is_empty() {
            return json!({ "ok": false, "reason_code": "lensmap_annotation_empty_tag" });
        }
        if !seen.insert(normalized) {
            return json!({ "ok": false, "reason_code": "lensmap_annotation_duplicate_tag" });
        }
    }

    json!({ "ok": true, "reason_code": "lensmap_annotation_valid" })
}

fn merged_policy(raw: Option<&Value>) -> Policy {
    let mut policy = Policy::default();
    let Some(obj) = raw.and_then(Value::as_object) else {
        return policy;
    };

    if let Some(value) = obj.get("index_first_required").and_then(Value::as_bool) {
        policy.index_first_required = value;
    }
    if let Some(value) = obj.get("max_burn_slo_tokens").and_then(Value::as_i64) {
        policy.max_burn_slo_tokens = value;
    }
    if let Some(value) = obj.get("max_recall_top").and_then(Value::as_i64) {
        policy.max_recall_top = value;
    }
    if let Some(value) = obj.get("max_max_files").and_then(Value::as_i64) {
        policy.max_max_files = value;
    }
    if let Some(value) = obj.get("max_expand_lines").and_then(Value::as_i64) {
        policy.max_expand_lines = value;
    }
    if let Some(value) = obj
        .get("bootstrap_hydration_token_cap")
        .and_then(Value::as_i64)
    {
        policy.bootstrap_hydration_token_cap = value;
    }
    if let Some(value) = obj.get("block_stale_override").and_then(Value::as_bool) {
        policy.block_stale_override = value;
    }
    policy
}

fn validate_memory_policy(args: &[String], options: Option<&Value>) -> Value {
    let policy = merged_policy(options.and_then(|value| value.get("policy")));
    let parsed = parse_cli_args(args);
    let command = options
        .and_then(|value| value.get("command"))
        .map(value_as_text)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            parsed
                .positional
                .first()
                .cloned()
                .unwrap_or_else(|| "status".to_string())
        })
        .trim()
        .to_ascii_lowercase();

    if NON_EXECUTING_COMMANDS.contains(&command.as_str()) {
        return json!({
            "ok": true,
            "type": "memory_policy_validation",
            "reason_code": "policy_not_required_for_status_command",
            "policy": policy,
        });
    }

    if policy.index_first_required {
        if read_boolean(&parsed.flags, INDEX_BYPASS_FLAGS, false) {
            return build_failure(
                "index_first_bypass_forbidden",
                json!({ "command": command }),
            );
        }
        if DIRECT_READ_FLAGS.iter().any(|flag| {
            parsed
                .flags
                .get(*flag)
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
        }) {
            return build_failure("direct_file_read_forbidden", json!({ "command": command }));
        }
    }

    let bootstrap = read_boolean(&parsed.flags, &["bootstrap"], false);
    let lazy_hydration = read_boolean(&parsed.flags, &["lazy-hydration", "lazy_hydration"], true);
    let hydration_tokens = read_numeric(
        &parsed.flags,
        &["estimated-hydration-tokens", "estimated_hydration_tokens"],
        0,
    );
    if bootstrap && !lazy_hydration {
        return build_failure(
            "bootstrap_requires_lazy_hydration",
            json!({ "command": command }),
        );
    }
    if bootstrap && hydration_tokens > policy.bootstrap_hydration_token_cap {
        return build_failure(
            "bootstrap_hydration_token_cap_exceeded",
            json!({
                "cap": policy.bootstrap_hydration_token_cap,
                "hydration_tokens": hydration_tokens,
            }),
        );
    }

    let burn_threshold = read_numeric(
        &parsed.flags,
        &[
            "burn-threshold",
            "burn_threshold",
            "burn-slo-threshold",
            "burn_slo_threshold",
        ],
        policy.max_burn_slo_tokens,
    );
    if burn_threshold > policy.max_burn_slo_tokens {
        return build_failure(
            "burn_slo_threshold_exceeded",
            json!({
                "configured_threshold": burn_threshold,
                "max_burn_slo_tokens": policy.max_burn_slo_tokens,
            }),
        );
    }

    if !read_boolean(&parsed.flags, &["fail-closed", "fail_closed"], true) {
        return build_failure("fail_closed_required", json!({ "command": command }));
    }

    let top = read_numeric(&parsed.flags, &["top", "recall-top", "recall_top"], 5);
    let max_files = read_numeric(&parsed.flags, &["max-files", "max_files"], 1);
    let expand_lines = read_numeric(&parsed.flags, &["expand-lines", "expand_lines"], 0);
    if top > policy.max_recall_top
        || max_files > policy.max_max_files
        || expand_lines > policy.max_expand_lines
    {
        return build_failure(
            "recall_budget_exceeded",
            json!({
                "top": top,
                "max_files": max_files,
                "expand_lines": expand_lines,
                "policy": {
                    "max_recall_top": policy.max_recall_top,
                    "max_max_files": policy.max_max_files,
                    "max_expand_lines": policy.max_expand_lines,
                }
            }),
        );
    }

    if policy.block_stale_override && read_boolean(&parsed.flags, STALE_OVERRIDE_FLAGS, false) {
        return build_failure("stale_override_forbidden", json!({ "command": command }));
    }

    let scores = read_json_flag(&parsed.flags, &["scores-json", "scores_json"]);
    let ids = read_json_flag(&parsed.flags, &["ids-json", "ids_json"]);
    if scores.is_some() || ids.is_some() {
        let scores_array = scores
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();
        let ids_array = ids
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();
        let ranking = validate_descending_ranking(&scores_array, &ids_array);
        if !ranking.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return build_failure(
                ranking
                    .get("reason_code")
                    .and_then(Value::as_str)
                    .unwrap_or("ranking_validation_failed"),
                json!({ "command": command }),
            );
        }
    }

    let annotation = read_json_flag(
        &parsed.flags,
        &["lensmap-annotation-json", "lensmap_annotation_json"],
    );
    if annotation.is_some() {
        let validation = validate_lensmap_annotation(annotation.as_ref());
        if !validation
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return build_failure(
                validation
                    .get("reason_code")
                    .and_then(Value::as_str)
                    .unwrap_or("lensmap_annotation_invalid"),
                json!({ "command": command }),
            );
        }
    }

    json!({
        "ok": true,
        "type": "memory_policy_validation",
        "reason_code": "policy_ok",
        "command": command,
        "policy": policy,
        "effective_budget": {
            "top": top,
            "max_files": max_files,
            "expand_lines": expand_lines,
            "burn_threshold": burn_threshold,
        }
    })
}

fn guard_failure_result(validation: Option<&Value>, context: Option<&Value>) -> Value {
    let reason = validation
        .and_then(|value| value.get("reason_code"))
        .and_then(Value::as_str)
        .unwrap_or("policy_validation_failed");
    let mut payload = Map::<String, Value>::new();
    payload.insert("ok".to_string(), Value::Bool(false));
    payload.insert(
        "type".to_string(),
        Value::String("memory_policy_guard_reject".to_string()),
    );
    payload.insert("reason".to_string(), Value::String(reason.to_string()));
    payload.insert(
        "layer".to_string(),
        Value::String("client_runtime_memory_guard".to_string()),
    );
    payload.insert("fail_closed".to_string(), Value::Bool(true));

    if let Some(context_obj) = context.and_then(Value::as_object) {
        for (key, value) in context_obj {
            payload.insert(key.clone(), value.clone());
        }
    }

    json!({
        "ok": false,
        "status": 2,
        "stdout": format!("{}\n", Value::Object(payload.clone())),
        "stderr": format!("memory_policy_guard_reject:{}\n", reason),
        "payload": Value::Object(payload),
    })
}

pub fn run(_cwd: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let payload = match command.as_str() {
        "status" => Ok(json!({
            "ok": true,
            "type": "memory_policy_kernel_status",
            "default_policy": Policy::default(),
        })),
        "parse-cli" => load_payload(argv).map(|payload| {
            json!({
                "ok": true,
                "parsed": parse_cli_args(&payload.args),
            })
        }),
        "command-name" => load_payload(argv).map(|payload| {
            let fallback = payload.fallback.unwrap_or_else(|| "status".to_string());
            let parsed = parse_cli_args(&payload.args);
            let command = parsed
                .positional
                .first()
                .cloned()
                .unwrap_or(fallback)
                .trim()
                .to_ascii_lowercase();
            json!({
                "ok": true,
                "command": command,
            })
        }),
        "validate" => load_payload(argv).map(|payload| {
            json!({
                "ok": true,
                "validation": validate_memory_policy(&payload.args, payload.options.as_ref()),
            })
        }),
        "validate-ranking" => load_payload(argv).map(|payload| {
            json!({
                "ok": true,
                "validation": validate_descending_ranking(
                    &payload.scores.unwrap_or_default(),
                    &payload.ids.unwrap_or_default(),
                ),
            })
        }),
        "validate-lensmap" => load_payload(argv).map(|payload| {
            json!({
                "ok": true,
                "validation": validate_lensmap_annotation(payload.annotation.as_ref()),
            })
        }),
        "severity-rank" => load_payload(argv).map(|payload| {
            let value = payload.value.unwrap_or(Value::Null);
            json!({
                "ok": true,
                "rank": severity_rank_value(&value_as_text(&value)),
            })
        }),
        "guard-failure" => load_payload(argv).map(|payload| {
            json!({
                "ok": true,
                "result": guard_failure_result(payload.validation.as_ref(), payload.context.as_ref()),
            })
        }),
        _ => Err(format!("memory_policy_kernel_unknown_command:{command}")),
    };

    match payload {
        Ok(payload) => {
            let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
            print_json_line(&cli_receipt(
                &format!("memory_policy_kernel_{}", command.replace('-', "_")),
                payload,
            ));
            if ok {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_json_line(&cli_error("memory_policy_kernel_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_direct_file_reads() {
        let validation = validate_memory_policy(
            &[
                "query-index".to_string(),
                "--session-id=s1".to_string(),
                "--path=local/workspace/memory/2026-03-15.md".to_string(),
            ],
            None,
        );
        assert_eq!(validation.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            validation.get("reason_code").and_then(Value::as_str),
            Some("direct_file_read_forbidden")
        );
    }

    #[test]
    fn validates_lensmap_annotation_rules() {
        let failed = validate_lensmap_annotation(Some(&json!({
            "node_id": "n1",
            "tags": [],
            "jots": [],
        })));
        assert_eq!(failed.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            failed.get("reason_code").and_then(Value::as_str),
            Some("lensmap_annotation_missing_tags_or_jots")
        );

        let passed = validate_lensmap_annotation(Some(&json!({
            "node_id": "n1",
            "tags": ["memory"],
            "jots": ["note"],
        })));
        assert_eq!(passed.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn guard_failure_result_is_fail_closed() {
        let result = guard_failure_result(
            Some(&json!({
                "reason_code": "index_first_bypass_forbidden"
            })),
            Some(&json!({
                "stage": "client_preflight"
            })),
        );
        assert_eq!(result.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("status").and_then(Value::as_i64), Some(2));
        assert!(result
            .get("stderr")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("memory_policy_guard_reject:index_first_bypass_forbidden"));
    }
}
