// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};

use crate::contract_lane_utils as lane_utils;
use crate::success_criteria_compiler_kernel;
use crate::{deterministic_receipt_hash, now_iso};

const HIGH_TIER_TYPE_TOKENS: &[&str] = &[
    "self",
    "mutat",
    "security",
    "routing",
    "governance",
    "integrity",
    "policy",
    "strategy",
    "kernel",
    "spine",
    "attestation",
];
const HIGH_TIER_CMD_TOKENS: &[&str] = &[
    "systems/security/",
    "systems/spine/",
    "config/directives/",
    "strategy_controller",
    "policy_rootd",
    "capability_lease",
    "integrity_kernel",
    "startup_attestation",
];
const ROLLBACK_TOKENS: &[&str] = &["rollback", "revert", "undo", "restore"];
const NETWORK_DANGER_TOKENS: &[&str] = &["curl", "wget", "invoke-webrequest", "fetch(", "axios."];
const HIGH_TIER_PASS_B_TOKENS: &[&str] = &[
    "strategy",
    "policy",
    "security",
    "routing",
    "integrity",
    "governance",
];
const WEB_PROVIDER_CONTRACT_TARGETS: &[&str] = &[
    "brave",
    "duckduckgo",
    "exa",
    "firecrawl",
    "google",
    "minimax",
    "moonshot",
    "perplexity",
    "tavily",
    "xai",
];
const WEB_PROVIDER_AUTH_TARGETS: &[&str] = &["openai_codex", "github_copilot"];

fn usage() {
    println!("quorum-validator-kernel commands:");
    println!("  protheus-ops quorum-validator-kernel evaluate --payload-base64=<base64_json>");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
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
            .map_err(|err| format!("quorum_validator_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("quorum_validator_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("quorum_validator_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("quorum_validator_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
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
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_risk(value: Option<&Value>) -> &'static str {
    match normalize_text(value).to_ascii_lowercase().as_str() {
        "high" => "high",
        "low" => "low",
        _ => "medium",
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn normalize_web_provider(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "kimi" | "moonshot" => "moonshot".to_string(),
        "grok" | "xai" => "xai".to_string(),
        "duck_duck_go" | "duckduckgo" => "duckduckgo".to_string(),
        "brave_search" | "brave" => "brave".to_string(),
        other => other.to_string(),
    }
}

fn proposal_web_provider(proposal: &Map<String, Value>) -> String {
    let action_spec = as_object(proposal.get("action_spec"));
    let meta = as_object(proposal.get("meta"));
    let raw = [
        proposal.get("web_provider"),
        proposal.get("provider"),
        action_spec.and_then(|row| row.get("web_provider")),
        action_spec.and_then(|row| row.get("provider")),
        meta.and_then(|row| row.get("web_provider")),
        meta.and_then(|row| row.get("provider")),
    ]
    .into_iter()
    .map(as_str)
    .find(|row| !row.is_empty())
    .unwrap_or_default();
    normalize_web_provider(&raw)
}

fn proposal_web_auth_ready(proposal: &Map<String, Value>, blob: &str) -> bool {
    let meta = as_object(proposal.get("meta"));
    if meta
        .and_then(|row| row.get("web_auth_present"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        return true;
    }
    if meta
        .and_then(|row| row.get("web_auth_resolved"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        return true;
    }
    contains_any(blob, &["auth", "token", "credential"])
}

fn suggestion_blob(proposal: &Map<String, Value>) -> String {
    let action_spec = as_object(proposal.get("action_spec"));
    let meta = as_object(proposal.get("meta"));
    [
        proposal.get("type"),
        proposal.get("title"),
        proposal.get("suggested_next_command"),
        proposal.get("description"),
        proposal.get("summary"),
        meta.and_then(|row| row.get("summary")),
        action_spec.and_then(|row| row.get("command")),
        action_spec.and_then(|row| row.get("rollback_command")),
        proposal.get("provider"),
        proposal.get("web_provider"),
        meta.and_then(|row| row.get("provider")),
        meta.and_then(|row| row.get("web_provider")),
    ]
    .into_iter()
    .map(normalize_text)
    .filter(|row| !row.is_empty())
    .collect::<Vec<_>>()
    .join(" | ")
}

fn count_measurable_criteria(proposal: &Map<String, Value>) -> usize {
    let payload = json!({
        "proposal": Value::Object(proposal.clone()),
        "opts": {
            "include_verify": true,
            "include_validation": true,
            "allow_fallback": false,
        }
    });
    success_criteria_compiler_kernel::compile_proposal_success_criteria(payload_obj(&payload))
        .iter()
        .filter(|row| row.get("measurable").and_then(Value::as_bool) == Some(true))
        .count()
}

fn has_rollback_signal(proposal: &Map<String, Value>, blob: &str) -> bool {
    let action_spec = as_object(proposal.get("action_spec"));
    let meta = as_object(proposal.get("meta"));
    let rollback_field = [
        action_spec.and_then(|row| row.get("rollback_command")),
        proposal.get("rollback_plan"),
        meta.and_then(|row| row.get("rollback_plan")),
    ]
    .into_iter()
    .map(normalize_text)
    .find(|row| !row.is_empty())
    .unwrap_or_default()
    .to_ascii_lowercase();
    contains_any(&rollback_field, ROLLBACK_TOKENS) || contains_any(blob, ROLLBACK_TOKENS)
}

fn is_bound_objective(raw: &str) -> bool {
    let raw = raw.trim();
    if !raw.starts_with('T') {
        return false;
    }
    let mut chars = raw.chars().skip(1).peekable();
    let mut saw_digit = false;
    while let Some(ch) = chars.peek() {
        if ch.is_ascii_digit() {
            saw_digit = true;
            chars.next();
        } else {
            break;
        }
    }
    if !saw_digit || chars.next() != Some('_') {
        return false;
    }
    let tail: String = chars.collect();
    !tail.is_empty()
        && tail
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn pass_a(proposal: &Map<String, Value>) -> Value {
    let risk = normalize_risk(proposal.get("risk"));
    let blob = suggestion_blob(proposal).to_ascii_lowercase();
    let proposal_type = normalize_text(proposal.get("type")).to_ascii_lowercase();
    let action_spec = as_object(proposal.get("action_spec"));

    let high_tier = risk == "high"
        || contains_any(&proposal_type, HIGH_TIER_TYPE_TOKENS)
        || contains_any(&blob, HIGH_TIER_CMD_TOKENS);
    let has_command = !normalize_text(proposal.get("suggested_next_command")).is_empty()
        || !normalize_text(action_spec.and_then(|row| row.get("command"))).is_empty();
    let measurable_count = count_measurable_criteria(proposal) as u64;
    let has_rollback = has_rollback_signal(proposal, &blob);
    let allow = has_command && measurable_count >= 1 && has_rollback;

    json!({
        "name": "primary",
        "high_tier": high_tier,
        "allow": allow,
        "signals": {
            "has_command": has_command,
            "measurable_count": measurable_count,
            "has_rollback": has_rollback,
        }
    })
}

fn pass_b(proposal: &Map<String, Value>) -> Value {
    let risk = normalize_risk(proposal.get("risk"));
    let blob = suggestion_blob(proposal).to_ascii_lowercase();
    let proposal_type = normalize_text(proposal.get("type")).to_ascii_lowercase();
    let action_spec = as_object(proposal.get("action_spec"));
    let meta = as_object(proposal.get("meta"));
    let objective_id = [
        meta.and_then(|row| row.get("directive_objective_id")),
        meta.and_then(|row| row.get("objective_id")),
        action_spec.and_then(|row| row.get("objective_id")),
    ]
    .into_iter()
    .map(normalize_text)
    .find(|row| !row.is_empty())
    .unwrap_or_default();

    let high_tier = risk == "high"
        || contains_any(&blob, HIGH_TIER_CMD_TOKENS)
        || contains_any(&proposal_type, HIGH_TIER_PASS_B_TOKENS);
    let has_bound_objective = is_bound_objective(&objective_id);
    let network_danger = contains_any(&blob, NETWORK_DANGER_TOKENS);
    let dry_run_or_preview = blob.contains("--dry-run")
        || blob.contains("--dry_run")
        || blob.contains("preview")
        || blob.contains("score_only");
    let web_requested = contains_any(
        &blob,
        &["web_search", "web_fetch", "web tooling", "internet", "search provider"],
    );
    let provider = proposal_web_provider(proposal);
    let provider_contract_ok = !web_requested
        || (!provider.is_empty()
            && WEB_PROVIDER_CONTRACT_TARGETS
                .iter()
                .any(|target| target == &provider.as_str()));
    let auth_contract_ok = !web_requested || proposal_web_auth_ready(proposal, &blob);
    let provider_auth_contract_targets = WEB_PROVIDER_AUTH_TARGETS.to_vec();
    let allow = has_bound_objective
        && !network_danger
        && dry_run_or_preview
        && provider_contract_ok
        && auth_contract_ok;

    json!({
        "name": "secondary",
        "high_tier": high_tier,
        "allow": allow,
        "signals": {
            "bound_objective": has_bound_objective,
            "network_danger": network_danger,
            "dry_run_or_preview": dry_run_or_preview,
            "web_requested": web_requested,
            "web_provider": provider,
            "provider_contract_ok": provider_contract_ok,
            "auth_contract_ok": auth_contract_ok,
            "provider_contract_targets": WEB_PROVIDER_CONTRACT_TARGETS,
            "provider_auth_contract_targets": provider_auth_contract_targets,
        }
    })
}

pub(crate) fn evaluate_proposal_quorum(proposal: &Map<String, Value>) -> Value {
    let a = pass_a(proposal);
    let b = pass_b(proposal);
    let requires_quorum = a.get("high_tier").and_then(Value::as_bool).unwrap_or(false)
        || b.get("high_tier").and_then(Value::as_bool).unwrap_or(false);

    if !requires_quorum {
        return json!({
            "requires_quorum": false,
            "ok": true,
            "agreement": true,
            "reason": "not_required",
            "passes": [a, b],
        });
    }

    let a_allow = a.get("allow").and_then(Value::as_bool).unwrap_or(false);
    let b_allow = b.get("allow").and_then(Value::as_bool).unwrap_or(false);
    let agreement = a_allow == b_allow;
    let ok = agreement && a_allow && b_allow;
    let reason = if !agreement {
        "validator_disagreement"
    } else if !ok {
        "validators_denied"
    } else {
        "approved"
    };

    json!({
        "requires_quorum": true,
        "ok": ok,
        "agreement": agreement,
        "reason": reason,
        "passes": [a, b],
    })
}

fn run_command(command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "evaluate" => {
            let proposal = as_object(payload.get("proposal"))
                .cloned()
                .unwrap_or_default();
            Ok(json!({
                "ok": true,
                "verdict": evaluate_proposal_quorum(&proposal),
            }))
        }
        _ => Err("quorum_validator_kernel_unknown_command".to_string()),
    }
}

pub fn run(_root: &std::path::Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|row| row.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("quorum_validator_kernel", &err));
            return 1;
        }
    };
    match run_command(command, payload_obj(&payload)) {
        Ok(out) => {
            print_json_line(&cli_receipt("quorum_validator_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("quorum_validator_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quorum_validator_requires_both_passes_for_high_tier_proposal() {
        let proposal = json!({
            "type": "security_policy_update",
            "risk": "high",
            "suggested_next_command": "policy_rootd preview --dry-run --objective=T7_HARDEN",
            "meta": {
                "directive_objective_id": "T7_HARDEN",
                "rollback_plan": "rollback to previous policy"
            },
            "success_criteria": [
                { "metric": "latency", "target": "< 2s" }
            ]
        });
        let verdict = evaluate_proposal_quorum(payload_obj(&proposal));
        assert_eq!(verdict.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            verdict.get("reason").and_then(Value::as_str),
            Some("approved")
        );
    }

    #[test]
    fn quorum_validator_detects_disagreement() {
        let proposal = json!({
            "type": "strategy_change",
            "risk": "high",
            "suggested_next_command": "strategy_controller apply",
            "meta": {
                "directive_objective_id": "T3_ALPHA",
                "rollback_plan": "rollback to last approved strategy"
            },
            "success_criteria": [
                { "metric": "latency", "target": "< 2s" }
            ]
        });
        let verdict = evaluate_proposal_quorum(payload_obj(&proposal));
        assert_eq!(verdict.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            verdict.get("reason").and_then(Value::as_str),
            Some("validator_disagreement")
        );
    }
}
