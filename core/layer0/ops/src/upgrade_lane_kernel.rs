// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_MEMORY_DIR: &str = "memory/ops/lanes";
const DEFAULT_ADAPTIVE_INDEX_PATH: &str = "adaptive/ops/lanes/index.json";
const DEFAULT_EVENTS_PATH: &str = "local/state/ops/lanes/events.jsonl";
const DEFAULT_LATEST_PATH: &str = "local/state/ops/lanes/latest.json";
const DEFAULT_RECEIPTS_PATH: &str = "local/state/ops/lanes/receipts.jsonl";

fn usage() {
    println!("upgrade-lane-kernel commands:");
    println!("  protheus-ops upgrade-lane-kernel status --payload-base64=<json>");
    println!("  protheus-ops upgrade-lane-kernel record --payload-base64=<json>");
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
            .map_err(|err| format!("upgrade_lane_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("upgrade_lane_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("upgrade_lane_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("upgrade_lane_kernel_payload_decode_failed:{err}"));
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

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    let mut out = as_str(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn normalize_token(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    for ch in raw.to_ascii_lowercase().chars() {
        if out.len() >= max_len {
            break;
        }
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '/' | '-') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn to_bool(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(n)) => n.as_i64().map(|row| row != 0).unwrap_or(fallback),
        Some(Value::String(v)) => lane_utils::parse_bool(Some(v.as_str()), fallback),
        _ => fallback,
    }
}

fn clamp_int(value: Option<&Value>, lo: i64, hi: i64, fallback: i64) -> i64 {
    match value {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(fallback).clamp(lo, hi),
        Some(Value::String(v)) => v.trim().parse::<i64>().unwrap_or(fallback).clamp(lo, hi),
        _ => fallback,
    }
}

fn workspace_root(root: &Path) -> PathBuf {
    if let Some(raw) = std::env::var_os("OPENCLAW_WORKSPACE") {
        let path = PathBuf::from(raw);
        if path.is_absolute() {
            return path;
        }
    }
    root.to_path_buf()
}

fn resolve_path(root: &Path, raw: &str) -> PathBuf {
    let workspace = workspace_root(root);
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return workspace;
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        workspace.join(trimmed)
    }
}

fn rel(root: &Path, file_path: &Path) -> String {
    file_path
        .strip_prefix(workspace_root(root))
        .ok()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| file_path.to_string_lossy().replace('\\', "/"))
}

fn parse_json_loose(value: Option<&Value>) -> Value {
    match value {
        Some(Value::String(text)) => {
            serde_json::from_str::<Value>(text).unwrap_or_else(|_| json!({}))
        }
        Some(v) => v.clone(),
        None => json!({}),
    }
}

#[derive(Clone, Debug)]
struct NormalizedPolicy {
    enabled: bool,
    strict_default: bool,
    owner_id: String,
    event_stream_enabled: bool,
    event_stream_publish: bool,
    event_stream_stream: String,
    memory_dir: PathBuf,
    adaptive_index_path: PathBuf,
    events_path: PathBuf,
    latest_path: PathBuf,
    receipts_path: PathBuf,
    policy_path: PathBuf,
}

fn normalized_policy_value(root: &Path, payload: &Map<String, Value>) -> NormalizedPolicy {
    let opts = as_object(payload.get("opts")).cloned().unwrap_or_default();
    let policy_path = resolve_path(root, &as_str(payload.get("policy_path")));
    let raw_policy = fs::read_to_string(&policy_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}));
    let raw_policy_obj = payload_obj(&raw_policy);
    let payload_paths = as_object(payload.get("paths")).cloned().unwrap_or_default();
    let base_paths = as_object(opts.get("paths")).cloned().unwrap_or_default();
    let raw_paths = as_object(raw_policy_obj.get("paths"))
        .cloned()
        .unwrap_or_default();
    let default_paths = [
        ("memory_dir", DEFAULT_MEMORY_DIR),
        ("adaptive_index_path", DEFAULT_ADAPTIVE_INDEX_PATH),
        ("events_path", DEFAULT_EVENTS_PATH),
        ("latest_path", DEFAULT_LATEST_PATH),
        ("receipts_path", DEFAULT_RECEIPTS_PATH),
    ];
    let resolve_named_path = |key: &str, fallback: &str| -> PathBuf {
        let selected = raw_paths
            .get(key)
            .or_else(|| payload_paths.get(key))
            .or_else(|| base_paths.get(key))
            .cloned()
            .unwrap_or_else(|| Value::String(fallback.to_string()));
        resolve_path(root, &as_str(Some(&selected)))
    };

    let event_stream_obj = as_object(raw_policy_obj.get("event_stream"))
        .cloned()
        .unwrap_or_default();
    NormalizedPolicy {
        enabled: raw_policy_obj
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        strict_default: raw_policy_obj
            .get("strict_default")
            .map(|v| to_bool(Some(v), true))
            .unwrap_or(true),
        owner_id: normalize_token(
            &clean_text(
                raw_policy_obj
                    .get("owner_id")
                    .or_else(|| opts.get("owner_id")),
                120,
            ),
            120,
        ),
        event_stream_enabled: event_stream_obj
            .get("enabled")
            .map(|v| to_bool(Some(v), true))
            .unwrap_or(true),
        event_stream_publish: event_stream_obj
            .get("publish")
            .map(|v| to_bool(Some(v), true))
            .unwrap_or(true),
        event_stream_stream: clean_text(
            event_stream_obj
                .get("stream")
                .or_else(|| opts.get("stream")),
            180,
        ),
        memory_dir: resolve_named_path(default_paths[0].0, default_paths[0].1),
        adaptive_index_path: resolve_named_path(default_paths[1].0, default_paths[1].1),
        events_path: resolve_named_path(default_paths[2].0, default_paths[2].1),
        latest_path: resolve_named_path(default_paths[3].0, default_paths[3].1),
        receipts_path: resolve_named_path(default_paths[4].0, default_paths[4].1),
        policy_path,
    }
}

fn stable_hash(raw: &str, len: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    hash[..hash.len().min(len)].to_string()
}

fn persist_adaptive_index(policy: &NormalizedPolicy, row: &Value) -> Result<(), String> {
    let prev = lane_utils::read_json(&policy.adaptive_index_path).unwrap_or_else(|| json!({}));
    let mut next = prev.as_object().cloned().unwrap_or_default();
    next.entry("schema_id".to_string())
        .or_insert_with(|| Value::String("upgrade_lane_runtime_adaptive_index".to_string()));
    next.entry("schema_version".to_string())
        .or_insert_with(|| Value::String("1.0".to_string()));
    next.insert(
        "updated_at".to_string(),
        row.get("ts")
            .cloned()
            .unwrap_or_else(|| Value::String(now_iso())),
    );
    next.insert(
        "latest".to_string(),
        json!({
            "lane_id": row.get("lane_id").cloned().unwrap_or(Value::Null),
            "event": row.get("event").cloned().unwrap_or(Value::Null),
            "action": row.get("action").cloned().unwrap_or(Value::Null),
            "ok": row.get("ok").cloned().unwrap_or(Value::Bool(false)),
            "receipt_hash": row.get("receipt_hash").cloned().unwrap_or(Value::Null)
        }),
    );
    lane_utils::write_json(&policy.adaptive_index_path, &Value::Object(next))
}

fn persist_row(policy: &NormalizedPolicy, row: &Value) -> Result<(), String> {
    lane_utils::write_json(&policy.latest_path, row)?;
    lane_utils::append_jsonl(&policy.receipts_path, row)?;
    lane_utils::append_jsonl(&policy.events_path, row)?;
    lane_utils::ensure_parent(&policy.memory_dir.join(".keep"))?;
    persist_adaptive_index(policy, row)
}

fn status_value(root: &Path, payload: &Map<String, Value>) -> Value {
    let policy = normalized_policy_value(root, payload);
    let lane_id = clean_text(payload.get("lane_id"), 120);
    let lane_type = clean_text(payload.get("lane_type"), 120);
    if !policy.enabled {
        return json!({
            "ok": false,
            "lane_id": lane_id,
            "type": format!("{lane_type}_disabled"),
            "action": "status",
            "ts": now_iso(),
            "error": "lane_disabled",
            "policy_path": rel(root, &policy.policy_path)
        });
    }
    json!({
        "ok": true,
        "lane_id": lane_id,
        "type": format!("{lane_type}_status"),
        "action": "status",
        "ts": now_iso(),
        "latest": lane_utils::read_json(&policy.latest_path).unwrap_or_else(|| json!({})),
        "policy_path": rel(root, &policy.policy_path),
        "artifacts": {
            "memory_dir": rel(root, &policy.memory_dir),
            "adaptive_index_path": rel(root, &policy.adaptive_index_path),
            "events_path": rel(root, &policy.events_path),
            "latest_path": rel(root, &policy.latest_path),
            "receipts_path": rel(root, &policy.receipts_path)
        }
    })
}

fn record_value(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let policy = normalized_policy_value(root, payload);
    let lane_id = clean_text(payload.get("lane_id"), 120);
    let lane_type = clean_text(payload.get("lane_type"), 120);
    let action = clean_text(payload.get("action"), 80);
    if !policy.enabled {
        return Ok(json!({
            "ok": false,
            "lane_id": lane_id,
            "type": format!("{lane_type}_disabled"),
            "action": action,
            "ts": now_iso(),
            "error": "lane_disabled",
            "policy_path": rel(root, &policy.policy_path)
        }));
    }
    let strict = payload
        .get("strict")
        .map(|v| to_bool(Some(v), policy.strict_default))
        .unwrap_or(policy.strict_default);
    let apply = payload
        .get("apply")
        .map(|v| to_bool(Some(v), true))
        .unwrap_or(true);
    let record_args = as_object(payload.get("record_args"))
        .cloned()
        .unwrap_or_default();
    let payload_value = parse_json_loose(record_args.get("payload_json"));
    let owner_raw = record_args
        .get("owner")
        .or_else(|| payload.get("owner"))
        .map(|value| clean_text(Some(value), 120))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| policy.owner_id.clone());
    let owner = normalize_token(&owner_raw, 120);
    let owner = if owner.is_empty() {
        "system".to_string()
    } else {
        owner
    };
    let event = {
        let token = normalize_token(
            &clean_text(
                record_args.get("event").or_else(|| payload.get("event")),
                160,
            ),
            160,
        );
        if token.is_empty() {
            format!("{}_{}", lane_type, action)
        } else {
            token
        }
    };
    let row_ts = now_iso();
    let row_type = clean_text(record_args.get("type").or_else(|| payload.get("type")), 120);
    let row_type = if row_type.is_empty() {
        lane_type.clone()
    } else {
        row_type
    };
    let row_action = if action.is_empty() {
        "record".to_string()
    } else {
        action.clone()
    };
    let stream_value = if policy.event_stream_enabled
        && policy.event_stream_publish
        && !policy.event_stream_stream.is_empty()
    {
        Value::String(policy.event_stream_stream.clone())
    } else {
        Value::Null
    };
    let script = clean_text(payload.get("script_rel"), 260);
    let script_value = if script.is_empty() {
        Value::Null
    } else {
        Value::String(script)
    };
    let mut out = json!({
        "ok": true,
        "lane_id": lane_id,
        "type": row_type,
        "action": row_action,
        "event": event,
        "ts": row_ts,
        "owner": owner,
        "risk_tier": clamp_int(record_args.get("risk_tier").or_else(|| record_args.get("risk-tier")), 0, 5, 2),
        "strict": strict,
        "apply": apply,
        "stream": stream_value,
        "policy_path": rel(root, &policy.policy_path),
        "script": script_value,
        "payload": payload_value
    });
    out["receipt_hash"] = Value::String(stable_hash(
        &serde_json::to_string(&json!({
            "lane_id": out.get("lane_id").cloned().unwrap_or(Value::Null),
            "event": out.get("event").cloned().unwrap_or(Value::Null),
            "ts": out.get("ts").cloned().unwrap_or(Value::Null),
            "owner": out.get("owner").cloned().unwrap_or(Value::Null),
            "payload": out.get("payload").cloned().unwrap_or(Value::Null)
        }))
        .unwrap_or_default(),
        32,
    ));
    if apply {
        persist_row(&policy, &out)?;
    }
    Ok(out)
}

fn run_command(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "status" => Ok(status_value(root, payload)),
        "record" => record_value(root, payload),
        _ => Err("upgrade_lane_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|value| value.as_str()) else {
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
            print_json_line(&cli_error("upgrade_lane_kernel", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();
    match run_command(root, command, &payload) {
        Ok(out) => {
            print_json_line(&cli_receipt("upgrade_lane_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("upgrade_lane_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_persists_latest_and_adaptive_index() {
        let root = std::env::temp_dir().join(format!("upgrade-lane-kernel-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("config")).unwrap();
        let policy_path = root.join("config").join("policy.json");
        fs::write(&policy_path, "{\"enabled\":true}\n").unwrap();
        let payload = json!({
            "lane_id": "V3-RACE-169",
            "lane_type": "core_profile_contract",
            "script_rel": "packages/protheus-core/core_profile_contract.js",
            "policy_path": policy_path.to_string_lossy().to_string(),
            "paths": {
                "adaptive_index_path": root.join("adaptive/index.json").to_string_lossy().to_string(),
                "events_path": root.join("state/events.jsonl").to_string_lossy().to_string(),
                "latest_path": root.join("state/latest.json").to_string_lossy().to_string(),
                "receipts_path": root.join("state/receipts.jsonl").to_string_lossy().to_string(),
                "memory_dir": root.join("memory").to_string_lossy().to_string()
            },
            "action": "bootstrap",
            "record_args": {
                "event": "core_profile_bootstrap",
                "payload_json": "{\"mode\":\"lite\"}"
            }
        });
        let out = record_value(&root, payload_obj(&payload)).unwrap();
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(root.join("state/latest.json").exists());
        assert!(root.join("adaptive/index.json").exists());
        let _ = fs::remove_dir_all(&root);
    }
}
