// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// Rust-native operator tooling kernel.
// This consolidates legacy script authority into core commands while keeping
// thin compatibility wrappers possible at the client edge.

use base64::prelude::{Engine as _, BASE64_STANDARD};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

const DEFAULT_MODEL: &str = "ollama/kimi-k2.5:cloud";
const DEFAULT_OUTPUT_VALIDATE_MAX_QUERY: usize = 6000;
const DEFAULT_OUTPUT_VALIDATE_MAX_CREATIVE: usize = 9000;
const DEFAULT_OUTPUT_VALIDATE_MAX_GOVERNANCE: usize = 7000;

fn usage() {
    println!("Usage:");
    println!("  protheus-ops operator-tooling-kernel status [--control_runtime-root=<path>]");
    println!("  protheus-ops operator-tooling-kernel route-model [--payload-base64=<base64_json>] [--policy-path=<path>]");
    println!("  protheus-ops operator-tooling-kernel escalate-model [--payload-base64=<base64_json>] [--policy-path=<path>]");
    println!("  protheus-ops operator-tooling-kernel plan-auto [--payload-base64=<base64_json>]");
    println!("  protheus-ops operator-tooling-kernel plan-validate [--payload-base64=<base64_json>]");
    println!("  protheus-ops operator-tooling-kernel postflight-validate [--payload-base64=<base64_json>]");
    println!("  protheus-ops operator-tooling-kernel output-validate [--payload-base64=<base64_json>]");
    println!("  protheus-ops operator-tooling-kernel state-read [--key=<dot.path>] [--state-path=<path>]");
    println!("  protheus-ops operator-tooling-kernel state-write [--payload-base64=<base64_json>] [--state-path=<path>]");
    println!("  protheus-ops operator-tooling-kernel decision-log-append [--title=<text>] [--reason=<text>] [--verify=<cmd>] [--rollback=<hint>] [--details-base64=<base64_json>] [--decision-log-path=<path>]");
    println!("  protheus-ops operator-tooling-kernel safe-apply [--payload-base64=<base64_json>] [--timeout-ms=<n>]");
    println!("  protheus-ops operator-tooling-kernel memory-search --query=<text> [--limit=<n>] [--control_runtime-root=<path>]");
    println!("  protheus-ops operator-tooling-kernel memory-summarize --query=<text> [--limit=<n>] [--control_runtime-root=<path>]");
    println!("  protheus-ops operator-tooling-kernel memory-last-change [--limit=<n>] [--control_runtime-root=<path>]");
    println!("  protheus-ops operator-tooling-kernel membrief --query=<text> [--limit=<n>] [--control_runtime-root=<path>]");
    println!("  protheus-ops operator-tooling-kernel trace-find --trace-id=<id> [--limit=<n>] [--control_runtime-root=<path>]");
    println!("  protheus-ops operator-tooling-kernel sync-allowed-models [--control_runtime-root=<path>] [--policy-path=<path>]");
    println!("  protheus-ops operator-tooling-kernel smoke-routing [--control_runtime-root=<path>] [--policy-path=<path>]");
    println!("  protheus-ops operator-tooling-kernel spawn-safe [--payload-base64=<base64_json>] [--require-plan=1|0] [--strict-plan=1|0]");
    println!("  protheus-ops operator-tooling-kernel smart-spawn [--payload-base64=<base64_json>]");
    println!("  protheus-ops operator-tooling-kernel auto-spawn [--payload-base64=<base64_json>] [--max-attempts=<n>]");
    println!("  protheus-ops operator-tooling-kernel execute-handoff [--payload-base64=<base64_json>]");
    println!("  protheus-ops operator-tooling-kernel safe-run <domain> [args...] [--timeout-ms=<n>] [--retries=<n>]");
    println!("  protheus-ops operator-tooling-kernel control_runtime-health [--since-hours=<n>] [--control_runtime-root=<path>]");
    println!("  protheus-ops operator-tooling-kernel cron-drift [--control_runtime-root=<path>] [--workspace-root=<path>]");
    println!("  protheus-ops operator-tooling-kernel cron-sync [--control_runtime-root=<path>] [--workspace-root=<path>]");
    println!("  protheus-ops operator-tooling-kernel doctor [--control_runtime-root=<path>] [--workspace-root=<path>]");
    println!("  protheus-ops operator-tooling-kernel audit-plane [--control_runtime-root=<path>] [--workspace-root=<path>]");
    println!("  protheus-ops operator-tooling-kernel daily-brief [--control_runtime-root=<path>] [--workspace-root=<path>]");
    println!("  protheus-ops operator-tooling-kernel fail-playbook [--control_runtime-root=<path>] [--workspace-root=<path>]");
}

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "{\"ok\":false}".to_string())
    );
}

fn with_receipt(mut out: Value) -> Value {
    if out.get("ts").is_none() {
        out["ts"] = json!(crate::now_iso());
    }
    if out.get("receipt_hash").is_none() {
        out["receipt_hash"] = json!(crate::deterministic_receipt_hash(&out));
    }
    out
}

fn bool_flag(flags: &std::collections::HashMap<String, String>, key: &str, fallback: bool) -> bool {
    let Some(raw) = flags.get(key) else {
        return fallback;
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn parse_usize_flag(
    flags: &std::collections::HashMap<String, String>,
    key: &str,
    fallback: usize,
    min: usize,
    max: usize,
) -> usize {
    flags
        .get(key)
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .map(|n| n.clamp(min, max))
        .unwrap_or(fallback)
}

fn clean_text(value: &str, max_len: usize) -> String {
    crate::clean(value, max_len)
}

fn path_from_flag(root: &Path, raw: Option<&String>) -> Option<PathBuf> {
    raw.map(|token| {
        let path = PathBuf::from(token.trim());
        if path.is_absolute() {
            path
        } else {
            root.join(path)
        }
    })
}

fn control_runtime_root(root: &Path, parsed: &crate::ParsedArgs) -> PathBuf {
    if let Some(path) = path_from_flag(root, parsed.flags.get("control_runtime-root")) {
        return path;
    }
    let workspace_candidate = root.to_path_buf();
    let workspace_has_runtime = workspace_candidate.join("agents/main/agent").exists()
        || workspace_candidate.join("logs").exists()
        || workspace_candidate.join("state").exists();
    if workspace_has_runtime {
        return workspace_candidate;
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".control_runtime"))
        .unwrap_or_else(|| workspace_candidate)
}

fn agent_root(control_runtime_root: &Path) -> PathBuf {
    control_runtime_root.join("agents/main/agent")
}

fn routing_policy_path(root: &Path, parsed: &crate::ParsedArgs) -> PathBuf {
    path_from_flag(root, parsed.flags.get("policy-path"))
        .unwrap_or_else(|| agent_root(root).join("routing-policy.json"))
}

fn state_path(root: &Path, parsed: &crate::ParsedArgs) -> PathBuf {
    path_from_flag(root, parsed.flags.get("state-path"))
        .unwrap_or_else(|| agent_root(root).join("state.json"))
}

fn decision_log_path(root: &Path, parsed: &crate::ParsedArgs) -> PathBuf {
    path_from_flag(root, parsed.flags.get("decision-log-path"))
        .unwrap_or_else(|| agent_root(root).join("decisions.md"))
}

fn parse_json(raw: &str) -> Option<Value> {
    serde_json::from_str::<Value>(raw).ok()
}

fn payload_from_parsed(parsed: &crate::ParsedArgs) -> Value {
    if let Some(base64_raw) = parsed.flags.get("payload-base64") {
        if let Ok(bytes) = BASE64_STANDARD.decode(base64_raw.trim()) {
            if let Ok(text) = String::from_utf8(bytes) {
                if let Some(value) = parse_json(text.trim()) {
                    return value;
                }
            }
        }
    }
    if let Some(raw) = parsed.flags.get("payload") {
        if let Some(value) = parse_json(raw.trim()) {
            return value;
        }
    }
    if !io::stdin().is_terminal() {
        let mut buffer = String::new();
        if io::stdin().read_to_string(&mut buffer).is_ok() {
            if let Some(value) = parse_json(buffer.trim()) {
                return value;
            }
        }
    }
    json!({})
}

fn details_from_flag_or_payload(parsed: &crate::ParsedArgs, payload: &Value) -> Value {
    if let Some(base64_raw) = parsed.flags.get("details-base64") {
        if let Ok(bytes) = BASE64_STANDARD.decode(base64_raw.trim()) {
            if let Ok(text) = String::from_utf8(bytes) {
                if let Some(value) = parse_json(text.trim()) {
                    return value;
                }
            }
        }
    }
    payload.get("details").cloned().unwrap_or_else(|| json!({}))
}

fn read_json_file(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(raw.trim()).ok()
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("mkdir_failed:{err}"))?;
    }
    let encoded = serde_json::to_string_pretty(value)
        .map_err(|err| format!("json_encode_failed:{err}"))?;
    fs::write(path, format!("{encoded}\n")).map_err(|err| format!("write_failed:{err}"))
}

fn norm_tags(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|raw| clean_text(raw, 64).to_ascii_lowercase())
                .filter(|tag| !tag.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn high_risk_tags() -> HashSet<&'static str> {
    [
        "finance",
        "auth",
        "security",
        "payments",
        "legal",
        "compliance",
        "keys",
        "secrets",
        "prod",
        "deployment",
    ]
    .into_iter()
    .collect::<HashSet<_>>()
}

fn route_rule_matches(tags: &HashSet<String>, rule: &Value) -> bool {
    let cond = rule
        .get("if")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let tags_any = cond
        .get("tags_any")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let tags_all = cond
        .get("tags_all")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let tags_none = cond
        .get("tags_none")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if !tags_any.is_empty() {
        let any_match = tags_any
            .iter()
            .filter_map(Value::as_str)
            .map(|raw| clean_text(raw, 64).to_ascii_lowercase())
            .any(|tag| tags.contains(&tag));
        if !any_match {
            return false;
        }
    }
    if !tags_all.is_empty() {
        let all_match = tags_all
            .iter()
            .filter_map(Value::as_str)
            .map(|raw| clean_text(raw, 64).to_ascii_lowercase())
            .all(|tag| tags.contains(&tag));
        if !all_match {
            return false;
        }
    }
    if !tags_none.is_empty() {
        let none_match = tags_none
            .iter()
            .filter_map(Value::as_str)
            .map(|raw| clean_text(raw, 64).to_ascii_lowercase())
            .all(|tag| !tags.contains(&tag));
        if !none_match {
            return false;
        }
    }
    true
}

fn first_model_for_tier(policy: &Value, tier: &str, default_model: &str) -> String {
    let tiers = policy
        .get("tiers")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let Some(value) = tiers.get(tier) else {
        return default_model.to_string();
    };
    if let Some(model) = value.as_str() {
        let cleaned = clean_text(model, 240);
        if !cleaned.is_empty() {
            return cleaned;
        }
    }
    if let Some(rows) = value.as_array() {
        if let Some(model) = rows.first().and_then(Value::as_str) {
            let cleaned = clean_text(model, 240);
            if !cleaned.is_empty() {
                return cleaned;
            }
        }
    }
    default_model.to_string()
}

fn tier_for_model(policy: &Value, model: &str) -> Option<String> {
    let tiers = policy.get("tiers").and_then(Value::as_object)?;
    for (tier, value) in tiers {
        if value.as_str().map(|v| clean_text(v, 240)) == Some(model.to_string()) {
            return Some(clean_text(tier, 40));
        }
        if let Some(rows) = value.as_array() {
            if rows
                .first()
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 240))
                == Some(model.to_string())
            {
                return Some(clean_text(tier, 40));
            }
        }
    }
    None
}

fn route_model_with_policy(policy: &Value, tags: &[String], default_model: &str) -> Value {
    let rules = policy
        .get("rules")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let tag_set = tags.iter().cloned().collect::<HashSet<_>>();
    let mut default_rule: Option<(usize, Value)> = None;

    for (idx, rule) in rules.iter().enumerate() {
        let is_default = rule
            .get("if")
            .and_then(Value::as_object)
            .and_then(|cond| cond.get("default"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if is_default {
            default_rule = Some((idx, rule.clone()));
            continue;
        }
        if route_rule_matches(&tag_set, rule) {
            let tier = clean_text(
                rule.get("use").and_then(Value::as_str).unwrap_or("tier2"),
                40,
            );
            let model = first_model_for_tier(policy, &tier, default_model);
            return json!({
                "model": model,
                "tier": tier,
                "matched_rule_index": idx,
                "matched_default_rule": false
            });
        }
    }

    if let Some((idx, rule)) = default_rule {
        let tier = clean_text(
            rule.get("use").and_then(Value::as_str).unwrap_or("tier2"),
            40,
        );
        let model = first_model_for_tier(policy, &tier, default_model);
        return json!({
            "model": model,
            "tier": tier,
            "matched_rule_index": idx,
            "matched_default_rule": true
        });
    }

    let tier = "tier2";
    json!({
        "model": first_model_for_tier(policy, tier, default_model),
        "tier": tier,
        "matched_rule_index": Value::Null,
        "matched_default_rule": false
    })
}
