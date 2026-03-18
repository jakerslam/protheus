// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

fn usage() {
    println!("trainability-matrix-kernel commands:");
    println!("  protheus-ops trainability-matrix-kernel default-policy [--payload-base64=<json>]");
    println!("  protheus-ops trainability-matrix-kernel normalize-policy [--payload-base64=<json>]");
    println!("  protheus-ops trainability-matrix-kernel load-policy [--payload-base64=<json>]");
    println!("  protheus-ops trainability-matrix-kernel evaluate [--payload-base64=<json>]");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({"ok": ok, "type": kind, "ts": ts, "date": ts[..10].to_string(), "payload": payload});
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
    println!("{}", serde_json::to_string(value).unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string()));
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw).map_err(|err| format!("trainability_matrix_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| format!("trainability_matrix_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes).map_err(|err| format!("trainability_matrix_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text).map_err(|err| format!("trainability_matrix_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(max_len).collect::<String>()
}

fn normalize_token(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    let mut prev_us = false;
    for ch in clean_text(raw, max_len).to_ascii_lowercase().chars() {
        let mapped = if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '/' | '-') { ch } else { '_' };
        if mapped == '_' {
            if prev_us || out.is_empty() { continue; }
            prev_us = true;
            out.push(mapped);
        } else {
            prev_us = false;
            out.push(mapped);
        }
        if out.len() >= max_len { break; }
    }
    out.trim_matches('_').to_string()
}

fn normalize_token_list(value: Option<&Value>, max_len: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    if let Some(Value::Array(rows)) = value {
        for row in rows {
            let token = normalize_token(&match row { Value::String(v) => v.clone(), Value::Null => String::new(), other => other.to_string() }, max_len);
            if !token.is_empty() && !out.contains(&token) {
                out.push(token);
            }
        }
    }
    out
}

fn default_policy() -> Value {
    json!({
        "version": "1.0",
        "default_allow": false,
        "require_consent_granted": true,
        "provider_rules": {
            "internal": {
                "allow": true,
                "allowed_license_ids": ["internal_protheus"],
                "allowed_consent_modes": ["operator_policy", "internal_system", "explicit_opt_in"],
                "note": "Internal first-party data retained by local operator policy."
            }
        }
    })
}

fn normalize_rule(value: Option<&Value>) -> Value {
    let obj = value.and_then(Value::as_object).cloned().unwrap_or_default();
    let note = clean_text(obj.get("note").and_then(Value::as_str).unwrap_or(""), 220);
    json!({
        "allow": obj.get("allow").and_then(Value::as_bool).unwrap_or(false),
        "allowed_license_ids": normalize_token_list(obj.get("allowed_license_ids"), 160),
        "allowed_consent_modes": normalize_token_list(obj.get("allowed_consent_modes"), 120),
        "note": if note.is_empty() { Value::Null } else { Value::String(note) }
    })
}

fn normalize_policy(raw: Option<&Value>) -> Value {
    let base = default_policy();
    let obj = raw.and_then(Value::as_object).cloned().unwrap_or_default();
    let provider_rules_raw = obj.get("provider_rules").and_then(Value::as_object).cloned().unwrap_or_else(|| {
        base.get("provider_rules").and_then(Value::as_object).cloned().unwrap_or_default()
    });
    let mut provider_rules = Map::new();
    for (provider, rule) in provider_rules_raw {
        let key = normalize_token(&provider, 120);
        if key.is_empty() { continue; }
        provider_rules.insert(key, normalize_rule(Some(&rule)));
    }
    let version = clean_text(obj.get("version").and_then(Value::as_str).unwrap_or("1.0"), 40);
    json!({
        "version": if version.is_empty() { "1.0".to_string() } else { version },
        "default_allow": obj.get("default_allow").and_then(Value::as_bool).unwrap_or(false),
        "require_consent_granted": obj.get("require_consent_granted").and_then(Value::as_bool).unwrap_or(true),
        "provider_rules": provider_rules,
    })
}

fn root_dir(repo_root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let raw = payload.get("root_dir").and_then(Value::as_str).unwrap_or("").trim();
    if raw.is_empty() {
        return repo_root.to_path_buf();
    }
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() { candidate } else { repo_root.join(candidate) }
}

fn default_policy_path(repo_root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Ok(raw) = std::env::var("TRAINABILITY_MATRIX_POLICY_PATH") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            return if candidate.is_absolute() { candidate } else { repo_root.join(candidate) };
        }
    }
    root_dir(repo_root, payload).join("config/trainability_matrix_policy.json")
}

fn load_policy(repo_root: &Path, payload: &Map<String, Value>) -> Value {
    let requested = payload.get("policy_path").and_then(Value::as_str).unwrap_or("").trim();
    let path = if requested.is_empty() {
        default_policy_path(repo_root, payload)
    } else {
        let candidate = PathBuf::from(requested);
        if candidate.is_absolute() { candidate } else { root_dir(repo_root, payload).join(candidate) }
    };
    let loaded = lane_utils::read_json(&path);
    normalize_policy(loaded.as_ref())
}

fn evaluate(metadata: Option<&Value>, policy_input: Option<&Value>) -> Value {
    let policy = normalize_policy(policy_input.or_else(|| None));
    let policy_obj = policy.as_object().cloned().unwrap_or_default();
    let provider_rules = policy_obj.get("provider_rules").and_then(Value::as_object).cloned().unwrap_or_default();
    let meta = metadata.and_then(Value::as_object).cloned().unwrap_or_default();
    let source = meta.get("source").and_then(Value::as_object).cloned().unwrap_or_default();
    let license = meta.get("license").and_then(Value::as_object).cloned().unwrap_or_default();
    let consent = meta.get("consent").and_then(Value::as_object).cloned().unwrap_or_default();
    let provider = normalize_token(source.get("provider").and_then(Value::as_str).unwrap_or("unknown"), 120);
    let provider_key = if provider.is_empty() { "unknown".to_string() } else { provider };
    let rule = provider_rules.get(&provider_key).and_then(Value::as_object).cloned();
    let consent_status = normalize_token(consent.get("status").and_then(Value::as_str).unwrap_or("unknown"), 40);
    let consent_mode = normalize_token(consent.get("mode").and_then(Value::as_str).unwrap_or("unknown"), 120);
    let license_id = normalize_token(license.get("id").and_then(Value::as_str).unwrap_or(""), 160);

    let mut checks = Map::new();
    checks.insert("provider_known".to_string(), Value::Bool(rule.is_some()));
    let provider_allowed = rule.as_ref().map(|r| r.get("allow").and_then(Value::as_bool).unwrap_or(false)).unwrap_or(policy_obj.get("default_allow").and_then(Value::as_bool).unwrap_or(false));
    checks.insert("provider_allowed".to_string(), Value::Bool(provider_allowed));
    let consent_granted = consent_status == "granted";
    checks.insert("consent_granted".to_string(), Value::Bool(consent_granted));

    let mut license_allowed = true;
    let mut consent_mode_allowed = true;
    if let Some(rule_obj) = rule.as_ref() {
        if let Some(Value::Array(ids)) = rule_obj.get("allowed_license_ids") {
            if !ids.is_empty() {
                license_allowed = ids.iter().filter_map(Value::as_str).any(|v| v == license_id);
            }
        }
        if let Some(Value::Array(modes)) = rule_obj.get("allowed_consent_modes") {
            if !modes.is_empty() {
                consent_mode_allowed = modes.iter().filter_map(Value::as_str).any(|v| v == consent_mode);
            }
        }
    }
    checks.insert("license_allowed".to_string(), Value::Bool(license_allowed));
    checks.insert("consent_mode_allowed".to_string(), Value::Bool(consent_mode_allowed));

    let require_consent_granted = policy_obj.get("require_consent_granted").and_then(Value::as_bool).unwrap_or(true);
    let mut reasons = Vec::<String>::new();
    if rule.is_none() && !policy_obj.get("default_allow").and_then(Value::as_bool).unwrap_or(false) {
        reasons.push("unknown_provider_default_deny".to_string());
    }
    if !provider_allowed { reasons.push("provider_terms_deny".to_string()); }
    if require_consent_granted && !consent_granted { reasons.push("consent_not_granted".to_string()); }
    if !license_allowed { reasons.push("license_not_allowlisted".to_string()); }
    if !consent_mode_allowed { reasons.push("consent_mode_not_allowlisted".to_string()); }

    json!({
        "allow": reasons.is_empty(),
        "provider": provider_key,
        "policy_version": policy_obj.get("version").cloned().unwrap_or_else(|| json!("1.0")),
        "reason": reasons.first().cloned().unwrap_or_else(|| "allow".to_string()),
        "reasons": reasons,
        "checks": Value::Object(checks),
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv.first().map(|value| value.to_ascii_lowercase()).unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") { usage(); return 0; }
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => { print_json_line(&cli_error("trainability_matrix_kernel_error", &err)); return 1; }
    };
    let input = payload_obj(&payload);
    let result = match command.as_str() {
        "default-policy" => cli_receipt("trainability_matrix_kernel_default_policy", json!({ "ok": true, "policy": default_policy() })),
        "normalize-policy" => cli_receipt("trainability_matrix_kernel_normalize_policy", json!({ "ok": true, "policy": normalize_policy(input.get("policy")) })),
        "load-policy" => cli_receipt("trainability_matrix_kernel_load_policy", json!({ "ok": true, "policy": load_policy(root, input), "policy_path": default_policy_path(root, input) })),
        "evaluate" => cli_receipt("trainability_matrix_kernel_evaluate", json!({ "ok": true, "evaluation": evaluate(input.get("metadata"), input.get("policy")) })),
        _ => cli_error("trainability_matrix_kernel_error", &format!("unknown_command:{command}")),
    };
    let exit = if result.get("ok").and_then(Value::as_bool).unwrap_or(false) { 0 } else { 1 };
    print_json_line(&result);
    exit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_blocks_unknown_provider_without_default_allow() {
        let result = evaluate(Some(&json!({
            "source": {"provider": "external"},
            "license": {"id": "mit"},
            "consent": {"status": "granted", "mode": "explicit_opt_in"}
        })), None);
        assert_eq!(result["allow"], json!(false));
        assert_eq!(result["reason"], json!("unknown_provider_default_deny"));
    }
}
