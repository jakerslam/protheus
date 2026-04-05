// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// Imported pattern contract (RTK intake):
// - source: local/workspace/vendor/rtk/src/hooks/permissions.rs
// - concept: deny/ask permission verdicts over compound commands using Bash(*) patterns.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::Path;

use crate::contract_lane_utils as lane_utils;
use crate::session_command_discovery_kernel::split_command_chain_for_kernel;
use crate::{deterministic_receipt_hash, now_iso};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PermissionVerdict {
    Allow,
    Deny,
    Ask,
}

impl PermissionVerdict {
    fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::Ask => "ask",
        }
    }
}

fn usage() {
    println!("command-permission-kernel commands:");
    println!(
        "  protheus-ops command-permission-kernel <evaluate|match-pattern|extract-pattern> [--payload=<json>|--payload-base64=<base64_json>]"
    );
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload
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
        "fail_closed": true
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("command_permission_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("command_permission_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("command_permission_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("command_permission_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn extract_bash_pattern(rule: &str) -> String {
    if let Some(inner) = rule.strip_prefix("Bash(") {
        if let Some(pattern) = inner.strip_suffix(')') {
            return clean_text(pattern, 240);
        }
    }
    clean_text(rule, 240)
}

fn append_rules(value: Option<&Value>, out: &mut Vec<String>) {
    let Some(arr) = value.and_then(Value::as_array) else {
        return;
    };
    for row in arr {
        let Some(raw) = row.as_str() else {
            continue;
        };
        let rule = extract_bash_pattern(raw);
        if !rule.is_empty() {
            out.push(rule);
        }
    }
}

fn load_rules(payload: &Map<String, Value>) -> (Vec<String>, Vec<String>) {
    let mut deny = Vec::<String>::new();
    let mut ask = Vec::<String>::new();
    append_rules(payload.get("deny_rules"), &mut deny);
    append_rules(payload.get("ask_rules"), &mut ask);
    if let Some(permissions) = payload.get("permissions").and_then(Value::as_object) {
        append_rules(permissions.get("deny"), &mut deny);
        append_rules(permissions.get("ask"), &mut ask);
    }
    (deny, ask)
}

fn glob_matches(cmd: &str, pattern: &str) -> bool {
    let normalized = pattern.replace(":*", " *").replace("*:", "* ");
    let parts = normalized.split('*').collect::<Vec<_>>();
    if parts.iter().all(|row| row.is_empty()) {
        return true;
    }
    let mut offset = 0usize;
    for (idx, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if idx == 0 {
            if !cmd.starts_with(part) {
                return false;
            }
            offset = part.len();
            continue;
        }
        if idx == parts.len() - 1 {
            return cmd[offset..].ends_with(*part);
        }
        if let Some(pos) = cmd[offset..].find(*part) {
            offset += pos + part.len();
        } else {
            return false;
        }
    }
    true
}

fn command_matches_pattern(cmd: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        let normalized = prefix.trim_end_matches(':').trim_end();
        if normalized.is_empty() || normalized == "*" {
            return true;
        }
        if !normalized.contains('*') {
            return cmd == normalized || cmd.starts_with(&format!("{normalized} "));
        }
    }
    if pattern.contains('*') {
        return glob_matches(cmd, pattern);
    }
    cmd == pattern || cmd.starts_with(&format!("{pattern} "))
}

fn evaluate_with_rules(
    command: &str,
    deny_rules: &[String],
    ask_rules: &[String],
) -> (PermissionVerdict, Vec<Value>) {
    let mut matched = Vec::<Value>::new();
    let mut saw_ask = false;
    for segment in split_command_chain_for_kernel(command) {
        let segment = clean_text(&segment, 600);
        if segment.is_empty() {
            continue;
        }
        for pattern in deny_rules {
            if command_matches_pattern(segment.as_str(), pattern.as_str()) {
                matched.push(json!({
                  "segment": segment,
                  "pattern": pattern,
                  "verdict": "deny"
                }));
                return (PermissionVerdict::Deny, matched);
            }
        }
        for pattern in ask_rules {
            if command_matches_pattern(segment.as_str(), pattern.as_str()) {
                saw_ask = true;
                matched.push(json!({
                  "segment": segment,
                  "pattern": pattern,
                  "verdict": "ask"
                }));
                break;
            }
        }
    }
    if saw_ask {
        (PermissionVerdict::Ask, matched)
    } else {
        (PermissionVerdict::Allow, matched)
    }
}

pub fn run(_root: &Path, argv: &[String]) -> i32 {
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
            print_json_line(&cli_error("command_permission_kernel_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let response = match command.as_str() {
        "extract-pattern" => {
            let rule = clean_text(input.get("rule").and_then(Value::as_str).unwrap_or(""), 400);
            cli_receipt(
                "command_permission_kernel_extract_pattern",
                json!({
                  "ok": true,
                  "rule": rule,
                  "pattern": extract_bash_pattern(&rule)
                }),
            )
        }
        "match-pattern" => {
            let cmd = clean_text(
                input.get("command").and_then(Value::as_str).unwrap_or(""),
                2000,
            );
            let pattern = clean_text(
                input.get("pattern").and_then(Value::as_str).unwrap_or(""),
                240,
            );
            cli_receipt(
                "command_permission_kernel_match_pattern",
                json!({
                  "ok": true,
                  "command": cmd,
                  "pattern": pattern,
                  "matched": command_matches_pattern(&cmd, &pattern)
                }),
            )
        }
        "evaluate" => {
            let cmd = clean_text(
                input.get("command").and_then(Value::as_str).unwrap_or(""),
                4000,
            );
            let (deny_rules, ask_rules) = load_rules(input);
            let (verdict, matched) = evaluate_with_rules(cmd.as_str(), &deny_rules, &ask_rules);
            cli_receipt(
                "command_permission_kernel_evaluate",
                json!({
                  "ok": true,
                  "command": cmd,
                  "deny_rules_count": deny_rules.len(),
                  "ask_rules_count": ask_rules.len(),
                  "verdict": verdict.as_str(),
                  "matched": matched
                }),
            )
        }
        _ => cli_error(
            "command_permission_kernel_error",
            "command_permission_kernel_unknown_command",
        ),
    };
    let ok = response.get("ok").and_then(Value::as_bool).unwrap_or(false);
    print_json_line(&response);
    if ok {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_takes_precedence_over_ask() {
        let (verdict, _) = evaluate_with_rules(
            "git push origin main",
            &vec!["git push*".to_string()],
            &vec!["git *".to_string()],
        );
        assert_eq!(verdict, PermissionVerdict::Deny);
    }

    #[test]
    fn ask_triggers_when_no_deny_matches() {
        let (verdict, _) = evaluate_with_rules(
            "cargo test --workspace",
            &vec!["git push*".to_string()],
            &vec!["cargo *".to_string()],
        );
        assert_eq!(verdict, PermissionVerdict::Ask);
    }

    #[test]
    fn wildcard_glob_handles_middle_star() {
        assert!(command_matches_pattern(
            "docker compose logs api",
            "docker * logs*"
        ));
    }

    #[test]
    fn extracts_bash_wrapped_pattern() {
        assert_eq!(extract_bash_pattern("Bash(git push*)"), "git push*");
    }
}
