// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

fn usage() {
    println!("policy-runtime-kernel commands:");
    println!("  protheus-ops policy-runtime-kernel deep-merge [--payload-base64=<json>]");
    println!("  protheus-ops policy-runtime-kernel resolve-policy-path [--payload-base64=<json>]");
    println!("  protheus-ops policy-runtime-kernel load-policy-runtime [--payload-base64=<json>]");
    println!("  protheus-ops policy-runtime-kernel resolve-policy-value-path [--payload-base64=<json>]");
}

fn cli_receipt(kind: &str, payload: Value) -> Value { let ts = now_iso(); let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true); let mut out = json!({"ok": ok, "type": kind, "ts": ts, "date": ts[..10].to_string(), "payload": payload}); out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out)); out }
fn cli_error(kind: &str, error: &str) -> Value { let ts = now_iso(); let mut out = json!({"ok": false, "type": kind, "ts": ts, "date": ts[..10].to_string(), "error": error, "fail_closed": true}); out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out)); out }
fn print_json_line(value: &Value) { println!("{}", serde_json::to_string(value).unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())); }

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw).map_err(|err| format!("policy_runtime_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| format!("policy_runtime_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes).map_err(|err| format!("policy_runtime_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text).map_err(|err| format!("policy_runtime_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn is_plain_object(value: &Value) -> bool { value.is_object() }

fn deep_merge(base_value: &Value, override_value: Option<&Value>) -> Value {
    match base_value {
        Value::Array(base_rows) => {
            if let Some(Value::Array(override_rows)) = override_value { Value::Array(override_rows.clone()) } else { Value::Array(base_rows.clone()) }
        }
        Value::Object(base_map) => {
            let override_map = override_value.and_then(Value::as_object);
            let mut out = Map::new();
            let mut keys = std::collections::BTreeSet::<String>::new();
            for key in base_map.keys() { keys.insert(key.clone()); }
            if let Some(override_map) = override_map { for key in override_map.keys() { keys.insert(key.clone()); } }
            for key in keys {
                let base_entry = base_map.get(&key).unwrap_or(&Value::Null);
                if let Some(override_map) = override_map {
                    if let Some(override_entry) = override_map.get(&key) {
                        out.insert(key.clone(), deep_merge(base_entry, Some(override_entry)));
                        continue;
                    }
                }
                out.insert(key.clone(), base_entry.clone());
            }
            Value::Object(out)
        }
        _ => override_value.cloned().unwrap_or_else(|| base_value.clone()),
    }
}

fn root_dir(repo_root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Some(Value::String(raw)) = payload.get("root_dir") {
        let candidate = PathBuf::from(raw.trim());
        if candidate.is_absolute() { return candidate; }
        if !raw.trim().is_empty() { return repo_root.join(candidate); }
    }
    repo_root.to_path_buf()
}

fn resolve_policy_path(repo_root: &Path, payload: &Map<String, Value>) -> String {
    let raw = payload.get("policy_path").or_else(|| payload.get("raw_path")).and_then(Value::as_str).unwrap_or("").trim();
    if raw.is_empty() { return String::new(); }
    let candidate = PathBuf::from(raw);
    let resolved = if candidate.is_absolute() { candidate } else { root_dir(repo_root, payload).join(candidate) };
    resolved.to_string_lossy().to_string()
}

fn resolve_path(repo_root: &Path, raw: &str, fallback_rel: &str) -> String {
    let chosen = if raw.trim().is_empty() { fallback_rel } else { raw.trim() };
    let candidate = PathBuf::from(chosen);
    let resolved = if candidate.is_absolute() { candidate } else { root_dir(repo_root, &Map::new()).join(candidate) };
    resolved.to_string_lossy().to_string()
}

fn load_policy_runtime(repo_root: &Path, payload: &Map<String, Value>) -> Value {
    let defaults = payload.get("defaults").cloned().filter(is_plain_object).unwrap_or_else(|| json!({}));
    let policy_path = resolve_policy_path(repo_root, payload);
    let raw = if policy_path.is_empty() { json!({}) } else { lane_utils::read_json(Path::new(&policy_path)).unwrap_or_else(|| json!({})) };
    let merged = deep_merge(&defaults, Some(&raw));
    json!({
        "policy": merged,
        "raw": raw,
        "defaults": defaults,
        "merged": merged,
        "policy_path": policy_path,
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv.first().map(|value| value.to_ascii_lowercase()).unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") { usage(); return 0; }
    let payload = match payload_json(&argv[1..]) { Ok(payload) => payload, Err(err) => { print_json_line(&cli_error("policy_runtime_kernel_error", &err)); return 1; } };
    let input = payload_obj(&payload);
    let result = match command.as_str() {
        "deep-merge" => cli_receipt("policy_runtime_kernel_deep_merge", json!({ "ok": true, "value": deep_merge(input.get("base").unwrap_or(&json!({})), input.get("override")) })),
        "resolve-policy-path" => cli_receipt("policy_runtime_kernel_resolve_policy_path", json!({ "ok": true, "policy_path": resolve_policy_path(root, input) })),
        "load-policy-runtime" => cli_receipt("policy_runtime_kernel_load_policy_runtime", json!({ "ok": true, "runtime": load_policy_runtime(root, input) })),
        "resolve-policy-value-path" => cli_receipt("policy_runtime_kernel_resolve_policy_value_path", json!({ "ok": true, "path": resolve_path(root, input.get("raw").and_then(Value::as_str).unwrap_or(""), input.get("fallback_rel").and_then(Value::as_str).unwrap_or("")) })),
        _ => cli_error("policy_runtime_kernel_error", &format!("unknown_command:{command}")),
    };
    let exit = if result.get("ok").and_then(Value::as_bool).unwrap_or(false) { 0 } else { 1 };
    print_json_line(&result);
    exit
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deep_merge_replaces_arrays_and_overrides_scalars() {
        let merged = deep_merge(&json!({"a":1,"rows":[1],"obj":{"x":1}}), Some(&json!({"rows":[2],"obj":{"y":2}})));
        assert_eq!(merged["rows"], json!([2]));
        assert_eq!(merged["obj"], json!({"x":1,"y":2}));
    }
}
