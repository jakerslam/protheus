// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// Imported pattern contract (RTK intake):
// - source: local/workspace/vendor/rtk/src/hooks/permissions.rs
// - concept: deny/ask permission verdicts over compound commands using Bash(*) patterns.

use serde_json::{json, Map, Value};
use std::path::Path;

use crate::contract_lane_utils as lane_utils;
use crate::session_command_discovery_kernel::split_command_chain_for_kernel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PermissionVerdict {
    Allow,
    Deny,
    Ask,
}

impl PermissionVerdict {
    pub(crate) fn as_str(self) -> &'static str {
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

fn parse_first_token_with_rest(command: &str) -> Option<(String, String)> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }
    let first = trimmed.chars().next()?;
    if first == '"' || first == '\'' {
        if let Some(end) = trimmed[1..].find(first) {
            let token = trimmed[1..1 + end].to_string();
            let rest = trimmed[1 + end + 1..].trim().to_string();
            return Some((token, rest));
        }
        return Some((trimmed[1..].to_string(), String::new()));
    }
    let split_at = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let token = trimmed[..split_at].to_string();
    let rest = trimmed[split_at..].trim().to_string();
    Some((token, rest))
}

fn is_env_assignment(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    let mut saw_eq = false;
    for ch in chars {
        if ch == '=' {
            saw_eq = true;
            break;
        }
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            return false;
        }
    }
    saw_eq
}

fn strip_env_prefix(raw: &str) -> String {
    let mut current = raw.trim().to_string();
    let options_with_value = [
        "-u",
        "--unset",
        "-c",
        "--chdir",
        "-s",
        "--split-string",
        "--default-signal",
        "--ignore-signal",
        "--block-signal",
    ];
    loop {
        let Some((token, rest)) = parse_first_token_with_rest(&current) else {
            return String::new();
        };
        if token == "--" || token == "-" {
            return rest;
        }
        if is_env_assignment(&token) {
            current = rest;
            continue;
        }
        if token.starts_with('-') {
            if options_with_value.contains(&token.as_str()) {
                let Some((_, rest_after_value)) = parse_first_token_with_rest(&rest) else {
                    return String::new();
                };
                current = rest_after_value;
                continue;
            }
            current = rest;
            continue;
        }
        return clean_text(&format!("{token} {rest}"), 2000);
    }
}

fn strip_timeout_prefix(raw: &str) -> String {
    let mut current = raw.trim().to_string();
    let options_with_value = ["-k", "--kill-after", "-s", "--signal"];
    let mut skipped_duration = false;
    loop {
        let Some((token, rest)) = parse_first_token_with_rest(&current) else {
            return String::new();
        };
        if token == "--" {
            return rest;
        }
        if token.starts_with('-') {
            if options_with_value.contains(&token.as_str()) {
                let Some((_, rest_after_value)) = parse_first_token_with_rest(&rest) else {
                    return String::new();
                };
                current = rest_after_value;
                continue;
            }
            current = rest;
            continue;
        }
        if !skipped_duration {
            skipped_duration = true;
            current = rest;
            continue;
        }
        return clean_text(&format!("{token} {rest}"), 2000);
    }
}

fn strip_nice_prefix(raw: &str) -> String {
    let mut current = raw.trim().to_string();
    let options_with_value = ["-n", "--adjustment", "--priority"];
    loop {
        let Some((token, rest)) = parse_first_token_with_rest(&current) else {
            return String::new();
        };
        if token == "--" {
            return rest;
        }
        if token.starts_with('-') {
            if token
                .strip_prefix('-')
                .map(|row| row.chars().all(|ch| ch.is_ascii_digit()))
                .unwrap_or(false)
            {
                current = rest;
                continue;
            }
            if options_with_value.contains(&token.as_str()) {
                let Some((_, rest_after_value)) = parse_first_token_with_rest(&rest) else {
                    return String::new();
                };
                current = rest_after_value;
                continue;
            }
            current = rest;
            continue;
        }
        return clean_text(&format!("{token} {rest}"), 2000);
    }
}

fn normalize_segment_for_permission(segment: &str) -> String {
    let mut current = clean_text(segment, 2000);
    if current.is_empty() {
        return current;
    }
    for _ in 0..4 {
        let Some((token, rest)) = parse_first_token_with_rest(&current) else {
            break;
        };
        let lowered = token.to_ascii_lowercase();
        let next = match lowered.as_str() {
            "sudo" | "command" | "nohup" | "setsid" => rest,
            "env" => strip_env_prefix(&rest),
            "timeout" | "gtimeout" => strip_timeout_prefix(&rest),
            "nice" => strip_nice_prefix(&rest),
            _ => break,
        };
        if next.trim().is_empty() || next == current {
            break;
        }
        current = clean_text(&next, 2000);
    }
    current
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

pub(crate) fn collect_permission_rules_from_map_for_kernel(
    payload: &Map<String, Value>,
) -> (Vec<String>, Vec<String>) {
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

pub(crate) fn collect_permission_rules_for_kernel(
    payload: Option<&Value>,
) -> (Vec<String>, Vec<String>) {
    payload
        .and_then(Value::as_object)
        .map(collect_permission_rules_from_map_for_kernel)
        .unwrap_or_else(|| (Vec::new(), Vec::new()))
}

fn load_rules(payload: &Map<String, Value>) -> (Vec<String>, Vec<String>) {
    collect_permission_rules_from_map_for_kernel(payload)
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
        let segment_raw = clean_text(&segment, 600);
        if segment_raw.is_empty() {
            continue;
        }
        let segment_normalized = clean_text(&normalize_segment_for_permission(&segment_raw), 600);
        for pattern in deny_rules {
            let direct_match = command_matches_pattern(segment_raw.as_str(), pattern.as_str());
            let normalized_match = !segment_normalized.is_empty()
                && segment_normalized != segment_raw
                && command_matches_pattern(segment_normalized.as_str(), pattern.as_str());
            if direct_match || normalized_match {
                matched.push(json!({
                  "segment": segment_raw,
                  "normalized_segment": segment_normalized,
                  "pattern": pattern,
                  "matched_via": if normalized_match { "normalized" } else { "direct" },
                  "verdict": "deny"
                }));
                return (PermissionVerdict::Deny, matched);
            }
        }
        for pattern in ask_rules {
            let direct_match = command_matches_pattern(segment_raw.as_str(), pattern.as_str());
            let normalized_match = !segment_normalized.is_empty()
                && segment_normalized != segment_raw
                && command_matches_pattern(segment_normalized.as_str(), pattern.as_str());
            if direct_match || normalized_match {
                saw_ask = true;
                matched.push(json!({
                  "segment": segment_raw,
                  "normalized_segment": segment_normalized,
                  "pattern": pattern,
                  "matched_via": if normalized_match { "normalized" } else { "direct" },
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

pub(crate) fn evaluate_command_permission_for_kernel(
    command: &str,
    deny_rules: &[String],
    ask_rules: &[String],
) -> (PermissionVerdict, Vec<Value>) {
    evaluate_with_rules(command, deny_rules, ask_rules)
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
    let payload = match lane_utils::payload_json(&argv[1..], "command_permission") {
        Ok(payload) => payload,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error("command_permission_kernel_error", &err));
            return 1;
        }
    };
    let input = lane_utils::payload_obj(&payload);
    let response = match command.as_str() {
        "extract-pattern" => {
            let rule = clean_text(input.get("rule").and_then(Value::as_str).unwrap_or(""), 400);
            lane_utils::cli_receipt(
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
            lane_utils::cli_receipt(
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
            lane_utils::cli_receipt(
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
        _ => lane_utils::cli_error(
            "command_permission_kernel_error",
            "command_permission_kernel_unknown_command",
        ),
    };
    let ok = response.get("ok").and_then(Value::as_bool).unwrap_or(false);
    lane_utils::print_json_line(&response);
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

    #[test]
    fn collects_rules_from_top_level_and_permissions_object() {
        let (deny, ask) = collect_permission_rules_for_kernel(Some(&json!({
            "deny_rules": ["Bash(git reset --hard*)"],
            "permissions": {
                "ask": ["curl *"]
            }
        })));
        assert_eq!(deny, vec!["git reset --hard*".to_string()]);
        assert_eq!(ask, vec!["curl *".to_string()]);
    }

    #[test]
    fn wrapper_normalization_allows_env_prefixed_commands_to_match() {
        let (verdict, matched) = evaluate_with_rules(
            "env FOO=bar cargo test --workspace",
            &vec![],
            &vec!["cargo *".to_string()],
        );
        assert_eq!(verdict, PermissionVerdict::Ask);
        assert_eq!(
            matched
                .first()
                .and_then(|row| row.get("matched_via"))
                .and_then(Value::as_str),
            Some("normalized")
        );
    }

    #[test]
    fn wrapper_normalization_allows_sudo_prefixed_commands_to_match() {
        let (verdict, _) = evaluate_with_rules(
            "sudo git push origin main",
            &vec!["git push*".to_string()],
            &vec![],
        );
        assert_eq!(verdict, PermissionVerdict::Deny);
    }
}
