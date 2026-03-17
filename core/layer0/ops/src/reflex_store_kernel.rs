// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::Utc;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::adaptive_layer_store_kernel;
use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_REL_PATH: &str = "reflex/registry.json";
const SOURCE_PATH: &str = "core/layer1/memory_runtime/adaptive/reflex_store.ts";

fn usage() {
    println!("reflex-store-kernel commands:");
    println!("  protheus-ops reflex-store-kernel default-state");
    println!("  protheus-ops reflex-store-kernel normalize-state [--payload-base64=<json>]");
    println!("  protheus-ops reflex-store-kernel read-state [--payload-base64=<json>]");
    println!("  protheus-ops reflex-store-kernel ensure-state [--payload-base64=<json>]");
    println!("  protheus-ops reflex-store-kernel set-state --payload-base64=<json>");
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
            .map_err(|err| format!("reflex_store_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("reflex_store_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("reflex_store_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("reflex_store_kernel_payload_decode_failed:{err}"));
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

fn as_array<'a>(value: Option<&'a Value>) -> &'a Vec<Value> {
    value.and_then(Value::as_array).unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
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

fn clamp_int(value: Option<&Value>, lo: i64, hi: i64, fallback: i64) -> i64 {
    let raw = match value {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(fallback),
        Some(Value::String(v)) => v.trim().parse::<i64>().unwrap_or(fallback),
        _ => fallback,
    };
    raw.clamp(lo, hi)
}

fn normalize_key(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    let mut prev_us = false;
    for ch in raw.chars() {
        let lower = ch.to_ascii_lowercase();
        let keep = matches!(lower, 'a'..='z' | '0'..='9' | ':' | '_' | '-');
        if keep {
            out.push(lower);
            prev_us = false;
        } else if !prev_us {
            out.push('_');
            prev_us = true;
        }
        if out.len() >= max_len {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

fn normalize_tag(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in raw.chars() {
        let lower = ch.to_ascii_lowercase();
        let keep = matches!(lower, 'a'..='z' | '0'..='9' | '_' | '-');
        if keep {
            out.push(lower);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
        if out.len() >= 32 {
            break;
        }
    }
    out.trim_matches('-').to_string()
}

fn is_alnum(raw: &str) -> bool {
    !raw.is_empty() && raw.chars().all(|ch| ch.is_ascii_alphanumeric())
}

fn stable_uid(seed: &str, prefix: &str, length: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::new();
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    let mut out = normalize_tag(prefix).replace('-', "");
    let body_len = length.saturating_sub(out.len()).max(8).min(hex.len());
    out.push_str(&hex[..body_len]);
    out.truncate(length.max(8).min(48));
    out
}

fn random_uid(prefix: &str, length: usize) -> String {
    stable_uid(
        &format!(
            "{}:{}:{}",
            prefix,
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ),
        prefix,
        length,
    )
}

fn now_ts() -> String {
    now_iso()
}

pub(crate) fn default_reflex_state() -> Value {
    json!({
        "version": "1.0",
        "policy": {
            "min_cells": 1,
            "max_cells": 2,
            "scale_up_queue_depth": 3,
            "scale_down_idle_seconds": 60,
            "routine_ttl_days": 14,
        },
        "routines": [],
        "metrics": {
            "total_created": 0,
            "total_updated": 0,
            "total_gc_deleted": 0,
            "last_gc_ts": Value::Null,
        }
    })
}

fn normalize_routine_uid(
    item: &Map<String, Value>,
    taken: &mut std::collections::BTreeSet<String>,
) -> String {
    let candidate = as_str(item.get("uid"));
    if is_alnum(&candidate) && !taken.contains(&candidate) {
        return candidate;
    }
    let key = normalize_key(&as_str(item.get("key")), 80);
    let seeded = if key.is_empty() {
        String::new()
    } else {
        stable_uid(&format!("adaptive_reflex|{key}|v1"), "r", 24)
    };
    if !seeded.is_empty() && !taken.contains(&seeded) {
        return seeded;
    }
    let mut uid = random_uid("r", 24);
    let mut attempts = 0;
    while taken.contains(&uid) && attempts < 8 {
        uid = random_uid("r", 24);
        attempts += 1;
    }
    uid
}

fn normalize_routine(
    raw: Option<&Map<String, Value>>,
    taken: &mut std::collections::BTreeSet<String>,
    now: &str,
) -> Option<Value> {
    let src = raw.cloned().unwrap_or_default();
    let key = {
        let base = normalize_key(&as_str(src.get("key")), 80);
        if base.is_empty() {
            normalize_key(&clean_text(src.get("name"), 80), 80)
        } else {
            base
        }
    };
    if key.is_empty() {
        return None;
    }
    let status = if as_str(src.get("status")).to_ascii_lowercase() == "disabled" {
        "disabled"
    } else {
        "active"
    };
    let uid = normalize_routine_uid(&src, taken);
    taken.insert(uid.clone());
    let name = clean_text(src.get("name"), 120);
    let last_run_ts = as_str(src.get("last_run_ts"));
    let created_ts = as_str(src.get("created_ts"));
    Some(json!({
        "uid": uid,
        "key": key,
        "name": if name.is_empty() { Value::String(key.clone()) } else { Value::String(name) },
        "trigger": clean_text(src.get("trigger"), 200),
        "action": clean_text(src.get("action"), 240),
        "status": status,
        "priority": clamp_int(src.get("priority"), 1, 100, 50),
        "last_run_ts": if last_run_ts.is_empty() { Value::Null } else { Value::String(last_run_ts) },
        "created_ts": if created_ts.is_empty() { Value::String(now.to_string()) } else { Value::String(created_ts) },
        "updated_ts": now,
    }))
}

pub(crate) fn normalize_reflex_state(raw: Option<&Value>, fallback: Option<&Value>) -> Value {
    let src = raw
        .or(fallback)
        .cloned()
        .unwrap_or_else(default_reflex_state);
    let policy = as_object(src.get("policy")).cloned().unwrap_or_default();
    let metrics = as_object(src.get("metrics")).cloned().unwrap_or_default();
    let now = now_ts();
    let mut taken = std::collections::BTreeSet::new();
    let mut routines = as_array(src.get("routines"))
        .iter()
        .filter_map(|row| normalize_routine(row.as_object(), &mut taken, &now))
        .collect::<Vec<_>>();
    routines.sort_by(|a, b| as_str(a.get("key")).cmp(&as_str(b.get("key"))));

    let min_cells = clamp_int(policy.get("min_cells"), 1, 128, 1);
    let mut max_cells = clamp_int(policy.get("max_cells"), 1, 256, 2);
    if max_cells < min_cells {
        max_cells = min_cells;
    }
    let version = as_str(src.get("version"));
    let last_gc_ts = as_str(metrics.get("last_gc_ts"));

    json!({
        "version": if version.is_empty() { Value::String("1.0".to_string()) } else { Value::String(version) },
        "policy": {
            "min_cells": min_cells,
            "max_cells": max_cells,
            "scale_up_queue_depth": clamp_int(policy.get("scale_up_queue_depth"), 1, 1000, 3),
            "scale_down_idle_seconds": clamp_int(policy.get("scale_down_idle_seconds"), 10, 3600, 60),
            "routine_ttl_days": clamp_int(policy.get("routine_ttl_days"), 1, 365, 14),
        },
        "routines": routines,
        "metrics": {
            "total_created": clamp_int(metrics.get("total_created"), 0, 100_000_000, 0),
            "total_updated": clamp_int(metrics.get("total_updated"), 0, 100_000_000, 0),
            "total_gc_deleted": clamp_int(metrics.get("total_gc_deleted"), 0, 100_000_000, 0),
            "last_gc_ts": if last_gc_ts.is_empty() { Value::Null } else { Value::String(last_gc_ts) }
        }
    })
}

fn store_target_path(root: &Path, payload: &Map<String, Value>) -> Result<String, String> {
    let raw = as_str(payload.get("file_path"));
    if raw.is_empty() {
        return Ok(DEFAULT_REL_PATH.to_string());
    }
    let (canonical_abs, _) =
        adaptive_layer_store_kernel::resolve_adaptive_path(root, &Map::new(), DEFAULT_REL_PATH)?;
    let requested = PathBuf::from(raw.trim());
    if requested.is_absolute() {
        if requested == canonical_abs {
            return Ok(DEFAULT_REL_PATH.to_string());
        }
        return Err(format!(
            "reflex_store: path override denied (requested={})",
            requested.display()
        ));
    }
    let normalized = raw
        .trim()
        .replace('\\', "/")
        .trim_start_matches("adaptive/")
        .to_string();
    if normalized == DEFAULT_REL_PATH {
        return Ok(DEFAULT_REL_PATH.to_string());
    }
    Err(format!(
        "reflex_store: path override denied (requested={})",
        raw.trim()
    ))
}

fn meta_with_defaults(payload: &Map<String, Value>, default_reason: &str) -> Value {
    let mut meta = as_object(payload.get("meta")).cloned().unwrap_or_default();
    if as_str(meta.get("source")).is_empty() {
        meta.insert("source".to_string(), Value::String(SOURCE_PATH.to_string()));
    }
    if as_str(meta.get("reason")).is_empty() {
        meta.insert(
            "reason".to_string(),
            Value::String(default_reason.to_string()),
        );
    }
    Value::Object(meta)
}

fn read_state_value(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let target = store_target_path(root, payload)?;
    let fallback = payload
        .get("fallback")
        .cloned()
        .unwrap_or_else(default_reflex_state);
    let out = adaptive_layer_store_kernel::read_json_command(
        root,
        &json!({
            "target_path": target,
            "fallback": fallback,
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    )?;
    Ok(normalize_reflex_state(out.get("value"), Some(&fallback)))
}

fn ensure_state_value(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let target = store_target_path(root, payload)?;
    let default_state = default_reflex_state();
    let out = adaptive_layer_store_kernel::ensure_json_command(
        root,
        &json!({
            "target_path": target,
            "default_value": default_state,
            "meta": meta_with_defaults(payload, "ensure_reflex_state"),
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    )?;
    Ok(normalize_reflex_state(
        out.get("value"),
        Some(&default_state),
    ))
}

fn set_state_value(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let target = store_target_path(root, payload)?;
    let default_state = default_reflex_state();
    let next_state = normalize_reflex_state(payload.get("state"), Some(&default_state));
    let out = adaptive_layer_store_kernel::set_json_command(
        root,
        &json!({
            "target_path": target,
            "value": next_state,
            "meta": meta_with_defaults(payload, "set_reflex_state"),
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    )?;
    Ok(normalize_reflex_state(
        out.get("value"),
        Some(&default_state),
    ))
}

fn run_command(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "default-state" => Ok(json!({ "ok": true, "state": default_reflex_state() })),
        "normalize-state" => Ok(json!({
            "ok": true,
            "state": normalize_reflex_state(payload.get("state"), payload.get("fallback")),
        })),
        "read-state" => Ok(json!({ "ok": true, "state": read_state_value(root, payload)? })),
        "ensure-state" => Ok(json!({ "ok": true, "state": ensure_state_value(root, payload)? })),
        "set-state" => Ok(json!({ "ok": true, "state": set_state_value(root, payload)? })),
        _ => Err("reflex_store_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
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
            print_json_line(&cli_error("reflex_store_kernel", &err));
            return 1;
        }
    };
    match run_command(root, command, payload_obj(&payload)) {
        Ok(out) => {
            print_json_line(&cli_receipt("reflex_store_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("reflex_store_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_reflex_state_assigns_uid_and_defaults() {
        let state = normalize_reflex_state(
            Some(&json!({
                "routines": [
                    { "key": "Queue Spike", "trigger": "queue_depth>5", "action": "scale_up" }
                ]
            })),
            None,
        );
        let routine = state
            .get("routines")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(Value::as_object)
            .expect("routine");
        assert_eq!(
            routine.get("key").and_then(Value::as_str),
            Some("queue_spike")
        );
        assert!(
            routine
                .get("uid")
                .and_then(Value::as_str)
                .unwrap_or("")
                .len()
                >= 8
        );
    }
}
