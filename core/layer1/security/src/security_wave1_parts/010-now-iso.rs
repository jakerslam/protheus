// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/security (authoritative)

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn clean_text(v: impl ToString, max_len: usize) -> String {
    v.to_string()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn normalize_token(v: impl ToString, max_len: usize) -> String {
    clean_text(v, max_len)
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '/' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn normalize_rel_path(v: impl ToString) -> String {
    v.to_string()
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn runtime_root(repo_root: &Path) -> PathBuf {
    repo_root.join("client").join("runtime")
}

fn runtime_config_path(repo_root: &Path, file_name: &str) -> PathBuf {
    runtime_root(repo_root).join("config").join(file_name)
}

fn runtime_state_root(repo_root: &Path) -> PathBuf {
    runtime_root(repo_root).join("local").join("state")
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    Ok(())
}

fn read_json_or(path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("encode_json_failed:{}:{err}", path.display()))?;
    fs::write(&tmp, payload).map_err(|err| format!("write_tmp_failed:{}:{err}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|err| {
        format!(
            "rename_tmp_failed:{}:{}:{err}",
            tmp.display(),
            path.display()
        )
    })
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let encoded = serde_json::to_string(row)
        .map_err(|err| format!("encode_jsonl_failed:{}:{err}", path.display()))?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("open_jsonl_failed:{}:{err}", path.display()))?;
    writeln!(file, "{encoded}")
        .map_err(|err| format!("append_jsonl_failed:{}:{err}", path.display()))
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    let raw = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
}

fn stable_json_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(v) => {
            if *v {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
        Value::Array(rows) => format!(
            "[{}]",
            rows.iter()
                .map(stable_json_string)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Object(map) => {
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            let mut out = String::from("{");
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string()));
                out.push(':');
                out.push_str(&stable_json_string(map.get(*key).unwrap_or(&Value::Null)));
            }
            out.push('}');
            out
        }
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
fn parse_json_from_stdout(raw: &str) -> Option<Value> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
        return Some(parsed);
    }
    for line in trimmed.lines().rev() {
        let candidate = line.trim();
        if !candidate.starts_with('{') {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<Value>(candidate) {
            return Some(parsed);
        }
    }
    None
}

#[derive(Debug, Clone, Default)]
struct CliArgs {
    positional: Vec<String>,
    flags: HashMap<String, String>,
}

fn parse_cli_args(argv: &[String]) -> CliArgs {
    let mut out = CliArgs::default();
    let mut i = 0usize;
    while i < argv.len() {
        let token = argv[i].trim().to_string();
        if !token.starts_with("--") {
            out.positional.push(token);
            i += 1;
            continue;
        }
        if let Some((k, v)) = token.split_once('=') {
            out.flags
                .insert(k.trim_start_matches("--").to_string(), v.to_string());
            i += 1;
            continue;
        }
        let key = token.trim_start_matches("--").to_string();
        if let Some(next) = argv.get(i + 1) {
            if !next.starts_with("--") {
                out.flags.insert(key, next.clone());
                i += 2;
                continue;
            }
        }
        out.flags.insert(key, "true".to_string());
        i += 1;
    }
    out
}

fn bool_from_str(v: Option<&str>, fallback: bool) -> bool {
    match v {
        Some(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => fallback,
        },
        None => fallback,
    }
}

fn bool_state(v: Option<&str>) -> Option<bool> {
    v.and_then(|raw| match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "enable" | "enabled" => Some(true),
        "0" | "false" | "no" | "off" | "disable" | "disabled" => Some(false),
        _ => None,
    })
}

fn number_i64(v: Option<&Value>, fallback: i64, lo: i64, hi: i64) -> i64 {
    let parsed = v.and_then(Value::as_i64).unwrap_or(fallback);
    parsed.clamp(lo, hi)
}

fn number_f64(v: Option<&Value>, fallback: f64, lo: f64, hi: f64) -> f64 {
    let parsed = v.and_then(Value::as_f64).unwrap_or(fallback);
    parsed.clamp(lo, hi)
}

// -------------------------------------------------------------------------------------------------
// Capability Switchboard
// -------------------------------------------------------------------------------------------------

fn capability_switchboard_paths(repo_root: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf, PathBuf) {
    let policy_path = std::env::var("CAPABILITY_SWITCHBOARD_POLICY_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| runtime_config_path(repo_root, "capability_switchboard_policy.json"));
    let state_path = std::env::var("CAPABILITY_SWITCHBOARD_STATE_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            runtime_state_root(repo_root)
                .join("security")
                .join("capability_switchboard_state.json")
        });
    let audit_path = std::env::var("CAPABILITY_SWITCHBOARD_AUDIT_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            runtime_state_root(repo_root)
                .join("security")
                .join("capability_switchboard_audit.jsonl")
        });
    let policy_root_script = std::env::var("CAPABILITY_SWITCHBOARD_POLICY_ROOT_SCRIPT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            runtime_root(repo_root)
                .join("systems")
                .join("security")
                .join("policy_rootd.js")
        });
    let chain_path = std::env::var("CAPABILITY_SWITCHBOARD_CHAIN_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            runtime_state_root(repo_root)
                .join("security")
                .join("capability_switchboard_chain.jsonl")
        });
    (
        policy_path,
        state_path,
        audit_path,
        policy_root_script,
        chain_path,
    )
}

fn capability_switchboard_default_policy() -> Value {
    json!({
        "version": "1.0",
        "require_dual_control": true,
        "dual_control_min_note_len": 12,
        "policy_root": {
            "required": true,
            "scope": "capability_switchboard_toggle"
        },
        "switches": {
            "autonomy": { "default_enabled": true, "security_locked": false, "require_policy_root": true, "description": "Core autonomy execution lane" },
            "reflex": { "default_enabled": true, "security_locked": false, "require_policy_root": true, "description": "Reflex execution lane" },
            "dreams": { "default_enabled": true, "security_locked": false, "require_policy_root": true, "description": "Dream/idle synthesis lane" },
            "sensory_depth": { "default_enabled": true, "security_locked": false, "require_policy_root": true, "description": "Deep sensory collection lane" },
            "routing_modes": { "default_enabled": true, "security_locked": false, "require_policy_root": true, "description": "Routing/model mode lane" },
            "external_actuation": { "default_enabled": true, "security_locked": false, "require_policy_root": true, "description": "External actuation lane" },
            "security": { "default_enabled": true, "security_locked": true, "require_policy_root": true, "description": "Security controls (non-deactivatable)" },
            "integrity": { "default_enabled": true, "security_locked": true, "require_policy_root": true, "description": "Integrity controls (non-deactivatable)" }
        }
    })
}

fn capability_switchboard_load_policy(policy_path: &Path) -> Value {
    let fallback = capability_switchboard_default_policy();
    let raw = read_json_or(policy_path, json!({}));
    let mut merged = fallback;

    if let Some(version) = raw.get("version").and_then(Value::as_str) {
        merged["version"] = Value::String(clean_text(version, 40));
    }
    if let Some(req_dual) = raw.get("require_dual_control").and_then(Value::as_bool) {
        merged["require_dual_control"] = Value::Bool(req_dual);
    }
    if raw.get("dual_control_min_note_len").is_some() {
        let n = number_i64(raw.get("dual_control_min_note_len"), 12, 8, 1024);
        merged["dual_control_min_note_len"] = Value::Number(n.into());
    }
    if let Some(raw_policy_root) = raw.get("policy_root").and_then(Value::as_object) {
        if let Some(v) = raw_policy_root.get("required").and_then(Value::as_bool) {
            merged["policy_root"]["required"] = Value::Bool(v);
        }
        if let Some(scope) = raw_policy_root.get("scope").and_then(Value::as_str) {
            merged["policy_root"]["scope"] = Value::String(clean_text(scope, 160));
        }
    }

    let mut switches_map = merged
        .get("switches")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if let Some(raw_switches) = raw.get("switches").and_then(Value::as_object) {
        for (raw_id, spec) in raw_switches {
            let id = normalize_token(raw_id, 120);
            if id.is_empty() {
                continue;
            }
            let mut row = switches_map.get(&id).cloned().unwrap_or_else(|| json!({}));
            if let Some(default_enabled) = spec.get("default_enabled").and_then(Value::as_bool) {
                row["default_enabled"] = Value::Bool(default_enabled);
            }
            if let Some(security_locked) = spec.get("security_locked").and_then(Value::as_bool) {
                row["security_locked"] = Value::Bool(security_locked);
            }
            if let Some(require_policy_root) =
                spec.get("require_policy_root").and_then(Value::as_bool)
            {
                row["require_policy_root"] = Value::Bool(require_policy_root);
            }
            if let Some(description) = spec.get("description").and_then(Value::as_str) {
                row["description"] = Value::String(clean_text(description, 200));
            }
            switches_map.insert(id, row);
        }
    }
    merged["switches"] = Value::Object(switches_map);
    merged
}

fn capability_switchboard_load_state(state_path: &Path) -> Value {
    let raw = read_json_or(state_path, json!({}));
    let switches = raw
        .get("switches")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    json!({
        "schema_id": "capability_switchboard_state",
        "schema_version": "1.0",
        "updated_at": raw.get("updated_at").cloned().unwrap_or(Value::Null),
        "switches": switches
    })
}

fn capability_switchboard_effective_switches(policy: &Value, state: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    let switches = policy
        .get("switches")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let state_switches = state
        .get("switches")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut ids = switches.keys().cloned().collect::<Vec<_>>();
    ids.sort();
    for id in ids {
        let policy_row = switches.get(&id).cloned().unwrap_or_else(|| json!({}));
        let state_row = state_switches
            .get(&id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let enabled = state_row
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| {
                policy_row
                    .get("default_enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            });
        out.push(json!({
            "id": id,
            "enabled": enabled,
            "default_enabled": policy_row.get("default_enabled").and_then(Value::as_bool).unwrap_or(true),
            "security_locked": policy_row.get("security_locked").and_then(Value::as_bool).unwrap_or(false),
            "require_policy_root": policy_row.get("require_policy_root").and_then(Value::as_bool).unwrap_or(true),
            "description": policy_row.get("description").cloned().unwrap_or(Value::Null),
            "updated_at": state_row.get("updated_at").cloned().unwrap_or(Value::Null),
            "updated_by": state_row.get("updated_by").cloned().unwrap_or(Value::Null),
            "reason": state_row.get("reason").cloned().unwrap_or(Value::Null)
        }));
    }
    out
}

fn capability_switchboard_chain_rows(chain_path: &Path) -> Vec<Value> {
    read_jsonl(chain_path)
        .into_iter()
        .filter(|row| {
            row.get("type")
                .and_then(Value::as_str)
                .map(|v| v == "capability_switchboard_chain_event")
                .unwrap_or(false)
        })
        .collect::<Vec<_>>()
}

fn capability_switchboard_chain_tip(chain_path: &Path) -> String {
    capability_switchboard_chain_rows(chain_path)
        .last()
        .and_then(|row| row.get("hash"))
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 140))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "GENESIS".to_string())
}
