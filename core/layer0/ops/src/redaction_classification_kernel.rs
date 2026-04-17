// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

fn usage() {
    println!("redaction-classification-kernel commands:");
    println!(
        "  protheus-ops redaction-classification-kernel load-policy [--payload-base64=<json>]"
    );
    println!(
        "  protheus-ops redaction-classification-kernel classify-text [--payload-base64=<json>]"
    );
    println!(
        "  protheus-ops redaction-classification-kernel redact-text [--payload-base64=<json>]"
    );
    println!("  protheus-ops redaction-classification-kernel classify-and-redact [--payload-base64=<json>]");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out =
        json!({"ok": ok, "type": kind, "ts": ts, "date": ts[..10].to_string(), "payload": payload});
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({"ok": false, "type": kind, "ts": ts, "date": ts[..10].to_string(), "error": error, "fail_closed": true});
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
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
            .map_err(|err| format!("redaction_classification_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("redaction_classification_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("redaction_classification_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("redaction_classification_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    match value {
        Some(Value::String(v)) => v
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(max_len)
            .collect(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other
            .to_string()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(max_len)
            .collect(),
    }
}

fn resolve_policy_path(root: &Path, value: Option<&Value>) -> PathBuf {
    let raw = clean_text(value, 4096);
    if raw.is_empty() {
        return root.join("client/runtime/config/redaction_classification_policy.json");
    }
    let candidate = PathBuf::from(&raw);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn policy_path_has_forbidden_tokens(raw: &str) -> bool {
    let token = raw.trim();
    token.chars().any(|ch| ch == '\0' || ch.is_control())
        || token.split(['/', '\\']).any(|part| part == "..")
}

fn normalize_rule_action(raw: &str) -> String {
    let token = clean_text(Some(&Value::String(raw.to_string())), 32).to_ascii_lowercase();
    match token.as_str() {
        "mask" | "block" | "remove" | "replace" | "delete" | "redact" => "redact".to_string(),
        "allow" | "pass" => "allow".to_string(),
        _ => "redact".to_string(),
    }
}

fn read_policy(root: &Path, value: Option<&Value>) -> Result<(PathBuf, Value), String> {
    let raw_policy_path = clean_text(value, 4096);
    if !raw_policy_path.is_empty() && policy_path_has_forbidden_tokens(&raw_policy_path) {
        return Err("redaction_classification_kernel_policy_path_invalid".to_string());
    }
    let policy_path = resolve_policy_path(root, value);
    if !policy_path.exists() {
        return Ok((
            policy_path,
            json!({"patterns": [], "labels": [], "rules": []}),
        ));
    }
    let raw = fs::read_to_string(&policy_path)
        .map_err(|err| format!("redaction_classification_kernel_policy_read_failed:{err}"))?;
    let parsed = serde_json::from_str::<Value>(&raw)
        .map_err(|err| format!("redaction_classification_kernel_policy_decode_failed:{err}"))?;
    Ok((policy_path, parsed))
}

#[derive(Clone)]
struct CompiledRule {
    regex: Regex,
    label: String,
    action: String,
    rule_id: String,
}

fn compile_rules(policy: &Value) -> Vec<CompiledRule> {
    let mut compiled = Vec::new();
    if let Some(rows) = policy.get("patterns").and_then(Value::as_array) {
        for row in rows {
            let source = clean_text(row.get("pattern"), 512);
            let flags = clean_text(row.get("flags"), 16);
            let label = clean_text(row.get("label"), 80);
            if source.is_empty() {
                continue;
            }
            let mut builder = regex::RegexBuilder::new(&source);
            builder.case_insensitive(flags.contains('i'));
            builder.multi_line(flags.contains('m'));
            builder.dot_matches_new_line(flags.contains('s'));
            if let Ok(regex) = builder.build() {
                compiled.push(CompiledRule {
                    regex,
                    label: if label.is_empty() {
                        "sensitive".to_string()
                    } else {
                        label.clone()
                    },
                    action: normalize_rule_action(&clean_text(row.get("action"), 32)),
                    rule_id: clean_text(row.get("id"), 80),
                });
            }
        }
    }
    if let Some(rows) = policy.get("rules").and_then(Value::as_array) {
        for row in rows {
            let source = clean_text(row.get("regex"), 512);
            let flags = clean_text(row.get("flags"), 16);
            let label = clean_text(row.get("category").or_else(|| row.get("label")), 80);
            let action = clean_text(row.get("action"), 32).to_ascii_lowercase();
            let rule_id = clean_text(row.get("id"), 80);
            if source.is_empty() {
                continue;
            }
            let mut builder = regex::RegexBuilder::new(&source);
            builder.case_insensitive(flags.contains('i'));
            builder.multi_line(flags.contains('m'));
            builder.dot_matches_new_line(flags.contains('s'));
            if let Ok(regex) = builder.build() {
                compiled.push(CompiledRule {
                    regex,
                    label: if label.is_empty() {
                        "sensitive".to_string()
                    } else {
                        label
                    },
                    action: normalize_rule_action(&action),
                    rule_id,
                });
            }
        }
    }
    compiled
}

fn classify_text(text: &str, rules: &[CompiledRule]) -> Value {
    let mut findings = Vec::new();
    let mut labels = std::collections::BTreeSet::<String>::new();
    for rule in rules {
        for mat in rule.regex.find_iter(text) {
            labels.insert(rule.label.clone());
            findings.push(json!({
                "label": rule.label,
                "action": rule.action,
                "match": clean_text(Some(&Value::String(mat.as_str().to_string())), 120),
                "index": mat.start(),
                "rule_id": rule.rule_id,
            }));
        }
    }
    findings.sort_by(|a, b| {
        let a_idx = a.get("index").and_then(Value::as_u64).unwrap_or(0);
        let b_idx = b.get("index").and_then(Value::as_u64).unwrap_or(0);
        let a_label = a.get("label").and_then(Value::as_str).unwrap_or("");
        let b_label = b.get("label").and_then(Value::as_str).unwrap_or("");
        let a_rule = a.get("rule_id").and_then(Value::as_str).unwrap_or("");
        let b_rule = b.get("rule_id").and_then(Value::as_str).unwrap_or("");
        let a_match = a.get("match").and_then(Value::as_str).unwrap_or("");
        let b_match = b.get("match").and_then(Value::as_str).unwrap_or("");
        a_idx
            .cmp(&b_idx)
            .then_with(|| a_label.cmp(b_label))
            .then_with(|| a_rule.cmp(b_rule))
            .then_with(|| a_match.cmp(b_match))
    });
    findings.dedup_by(|a, b| {
        a.get("index").and_then(Value::as_u64).unwrap_or(0)
            == b.get("index").and_then(Value::as_u64).unwrap_or(0)
            && a.get("label").and_then(Value::as_str).unwrap_or("")
                == b.get("label").and_then(Value::as_str).unwrap_or("")
            && a.get("rule_id").and_then(Value::as_str).unwrap_or("")
                == b.get("rule_id").and_then(Value::as_str).unwrap_or("")
            && a.get("match").and_then(Value::as_str).unwrap_or("")
                == b.get("match").and_then(Value::as_str).unwrap_or("")
    });
    json!({"ok": true, "findings": findings, "labels": labels.into_iter().collect::<Vec<_>>()})
}

fn redact_text(text: &str, rules: &[CompiledRule], replacement: &str) -> Value {
    let mut out = text.to_string();
    for rule in rules {
        if matches!(rule.action.as_str(), "redact" | "block" | "mask") {
            out = rule.regex.replace_all(&out, replacement).to_string();
        }
    }
    json!({"ok": true, "text": out, "replacement": replacement})
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
            print_json_line(&cli_error("redaction_classification_kernel_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let replacement = {
        let raw = clean_text(input.get("replacement"), 120);
        if raw.is_empty() {
            "[REDACTED]".to_string()
        } else {
            raw
        }
    };
    let result = match command.as_str() {
        "load-policy" => match read_policy(
            root,
            input.get("policyPath").or_else(|| input.get("policy_path")),
        ) {
            Ok((policy_path, policy)) => cli_receipt(
                "redaction_classification_kernel_load_policy",
                json!({"ok": true, "policy_path": policy_path, "policy": policy}),
            ),
            Err(err) => cli_error("redaction_classification_kernel_error", &err),
        },
        "classify-text" => match read_policy(
            root,
            input.get("policyPath").or_else(|| input.get("policy_path")),
        ) {
            Ok((policy_path, policy)) => {
                let rules = compile_rules(&policy);
                let classified = classify_text(&clean_text(input.get("text"), 16384), &rules);
                cli_receipt(
                    "redaction_classification_kernel_classify_text",
                    json!({"ok": true, "policy_path": policy_path, "classification": classified}),
                )
            }
            Err(err) => cli_error("redaction_classification_kernel_error", &err),
        },
        "redact-text" => match read_policy(
            root,
            input.get("policyPath").or_else(|| input.get("policy_path")),
        ) {
            Ok((policy_path, policy)) => {
                let rules = compile_rules(&policy);
                let redaction =
                    redact_text(&clean_text(input.get("text"), 16384), &rules, &replacement);
                cli_receipt(
                    "redaction_classification_kernel_redact_text",
                    json!({"ok": true, "policy_path": policy_path, "redaction": redaction}),
                )
            }
            Err(err) => cli_error("redaction_classification_kernel_error", &err),
        },
        "classify-and-redact" => match read_policy(
            root,
            input.get("policyPath").or_else(|| input.get("policy_path")),
        ) {
            Ok((policy_path, policy)) => {
                let text = clean_text(input.get("text"), 16384);
                let rules = compile_rules(&policy);
                let classification = classify_text(&text, &rules);
                let redaction = redact_text(&text, &rules, &replacement);
                cli_receipt(
                    "redaction_classification_kernel_classify_and_redact",
                    json!({
                        "ok": true,
                        "policy_path": policy_path,
                        "classification": classification,
                        "redaction": redaction,
                    }),
                )
            }
            Err(err) => cli_error("redaction_classification_kernel_error", &err),
        },
        _ => cli_error(
            "redaction_classification_kernel_error",
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
    fn classify_supports_rules_schema() {
        let policy = json!({
            "rules": [
                {"id": "email", "category": "pii", "action": "redact", "regex": "[A-Z0-9._%+-]+@[A-Z0-9.-]+", "flags": "gi"}
            ]
        });
        let rules = compile_rules(&policy);
        let out = classify_text("reach me at jay@example.com", &rules);
        assert_eq!(
            out.get("labels")
                .and_then(Value::as_array)
                .map(|rows| rows.len())
                .unwrap_or(0),
            1
        );
    }

    #[test]
    fn redact_supports_patterns_schema() {
        let policy = json!({
            "patterns": [
                {"pattern": "secret", "flags": "gi", "label": "secret", "action": "redact"}
            ]
        });
        let rules = compile_rules(&policy);
        let out = redact_text("a secret value", &rules, "[REDACTED]");
        assert!(out
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("[REDACTED]"));
    }
}
