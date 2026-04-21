// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_STATE_REL: &str = "client/runtime/local/state/memory/session_isolation.json";

fn usage() {
    println!("memory-session-isolation-kernel commands:");
    println!("  protheus-ops memory-session-isolation-kernel load-state [--payload-base64=<base64_json>]");
    println!("  protheus-ops memory-session-isolation-kernel save-state [--payload-base64=<base64_json>]");
    println!(
        "  protheus-ops memory-session-isolation-kernel validate [--payload-base64=<base64_json>]"
    );
    println!("  protheus-ops memory-session-isolation-kernel failure-result [--payload-base64=<base64_json>]");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("memory_session_isolation_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("memory_session_isolation_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("memory_session_isolation_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("memory_session_isolation_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_object<'a>(value: Option<&'a Value>) -> Option<&'a Map<String, Value>> {
    value.and_then(Value::as_object)
}

fn as_array<'a>(value: Option<&'a Value>) -> &'a Vec<Value> {
    value.and_then(Value::as_array).unwrap_or_else(|| {
        static EMPTY: OnceLock<Vec<Value>> = OnceLock::new();
        EMPTY.get_or_init(Vec::new)
    })
}

fn as_str(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn to_bool(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(n)) => n.as_i64().map(|row| row != 0).unwrap_or(fallback),
        Some(Value::String(v)) => lane_utils::parse_bool(Some(v.as_str()), fallback),
        _ => fallback,
    }
}

fn workspace_root(root: &Path) -> PathBuf {
    if let Some(raw) = std::env::var_os("INFRING_WORKSPACE") {
        let path = PathBuf::from(raw);
        if path.is_absolute() {
            return path;
        }
    }
    root.to_path_buf()
}

fn resolve_path(root: &Path, raw: Option<&Value>, fallback_rel: &str) -> PathBuf {
    let workspace = workspace_root(root);
    let trimmed = as_str(raw);
    if trimmed.is_empty() {
        return workspace.join(fallback_rel);
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        workspace.join(candidate)
    }
}

fn state_path_from_map(root: &Path, map: &Map<String, Value>) -> PathBuf {
    resolve_path(
        root,
        map.get("statePath").or_else(|| map.get("state_path")),
        DEFAULT_STATE_REL,
    )
}

fn default_state_value() -> Value {
    json!({
        "schema_version": "1.0",
        "resources": {}
    })
}

fn load_state_value(path: &Path) -> Value {
    let parsed = fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(default_state_value);
    let mut state = parsed.as_object().cloned().unwrap_or_default();
    if !state.contains_key("schema_version") {
        state.insert(
            "schema_version".to_string(),
            Value::String("1.0".to_string()),
        );
    }
    let resources_ok = state.get("resources").and_then(Value::as_object).is_some();
    if !resources_ok {
        state.insert("resources".to_string(), json!({}));
    }
    Value::Object(state)
}

fn save_state_value(path: &Path, state: &Value) -> Result<Value, String> {
    let mut normalized = state.as_object().cloned().unwrap_or_default();
    if !normalized.contains_key("schema_version") {
        normalized.insert(
            "schema_version".to_string(),
            Value::String("1.0".to_string()),
        );
    }
    let resources_ok = normalized
        .get("resources")
        .and_then(Value::as_object)
        .is_some();
    if !resources_ok {
        normalized.insert("resources".to_string(), json!({}));
    }
    let saved = Value::Object(normalized);
    lane_utils::write_json(path, &saved)?;
    Ok(saved)
}

fn parse_cli_args(raw_args: &[String]) -> (Vec<String>, HashMap<String, String>) {
    let mut positional = Vec::new();
    let mut flags = HashMap::new();
    for token in raw_args {
        if !token.starts_with("--") {
            positional.push(token.clone());
            continue;
        }
        match token.split_once('=') {
            Some((key, value)) => {
                flags.insert(key.trim_start_matches("--").to_string(), value.to_string());
            }
            None => {
                flags.insert(
                    token.trim_start_matches("--").to_string(),
                    "true".to_string(),
                );
            }
        }
    }
    (positional, flags)
}

fn session_id_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._:-]{2,127}$").unwrap())
}

fn parsed_args_value(raw_args: &[String]) -> Value {
    let (positional, flags) = parse_cli_args(raw_args);
    json!({
        "positional": positional,
        "flags": flags
    })
}

fn find_session_id(flags: &HashMap<String, String>, options: &Map<String, Value>) -> String {
    let option_value = options
        .get("sessionId")
        .or_else(|| options.get("session_id"));
    let from_options = as_str(option_value);
    if !from_options.is_empty() {
        return from_options;
    }
    [
        "session-id",
        "session_id",
        "session",
        "session-key",
        "session_key",
    ]
    .iter()
    .filter_map(|key| flags.get(*key))
    .map(|value| value.trim().to_string())
    .find(|value| !value.is_empty())
    .unwrap_or_default()
}

fn collect_resource_keys(flags: &HashMap<String, String>) -> Vec<String> {
    let names = [
        "resource-id",
        "resource_id",
        "item-id",
        "item_id",
        "node-id",
        "node_id",
        "uid",
        "memory-id",
        "memory_id",
        "task-id",
        "task_id",
    ];
    let mut out = Vec::new();
    for name in names {
        let value = flags.get(name).map(|row| row.trim()).unwrap_or("");
        if value.is_empty() {
            continue;
        }
        out.push(format!("{name}:{value}"));
    }
    out.sort();
    out.dedup();
    out
}
