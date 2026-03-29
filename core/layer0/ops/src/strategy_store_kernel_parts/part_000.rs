// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::Utc;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_REL_PATH: &str = "strategy/registry.json";
const POINTER_INDEX_REL: &str = "client/runtime/local/state/memory/adaptive_pointer_index.json";
const POINTERS_REL: &str = "client/runtime/local/state/memory/adaptive_pointers.jsonl";
const GENERATION_MODES: &[&str] = &[
    "normal",
    "narrative",
    "creative",
    "hyper-creative",
    "deep-thinker",
];
const EXECUTION_MODES: &[&str] = &["score_only", "canary_execute", "execute"];

fn usage() {
    println!("strategy-store-kernel commands:");
    println!("  protheus-ops strategy-store-kernel paths [--payload-base64=<json>]");
    println!("  protheus-ops strategy-store-kernel default-state");
    println!("  protheus-ops strategy-store-kernel default-draft [--payload-base64=<json>]");
    println!("  protheus-ops strategy-store-kernel normalize-mode [--payload-base64=<json>]");
    println!(
        "  protheus-ops strategy-store-kernel normalize-execution-mode [--payload-base64=<json>]"
    );
    println!("  protheus-ops strategy-store-kernel normalize-profile --payload-base64=<json>");
    println!("  protheus-ops strategy-store-kernel validate-profile --payload-base64=<json>");
    println!("  protheus-ops strategy-store-kernel normalize-queue-item --payload-base64=<json>");
    println!("  protheus-ops strategy-store-kernel recommend-mode [--payload-base64=<json>]");
    println!("  protheus-ops strategy-store-kernel read-state [--payload-base64=<json>]");
    println!("  protheus-ops strategy-store-kernel ensure-state [--payload-base64=<json>]");
    println!("  protheus-ops strategy-store-kernel set-state --payload-base64=<json>");
    println!("  protheus-ops strategy-store-kernel upsert-profile --payload-base64=<json>");
    println!("  protheus-ops strategy-store-kernel intake-signal --payload-base64=<json>");
    println!("  protheus-ops strategy-store-kernel materialize-from-queue --payload-base64=<json>");
    println!("  protheus-ops strategy-store-kernel touch-profile-usage --payload-base64=<json>");
    println!(
        "  protheus-ops strategy-store-kernel evaluate-gc-candidates [--payload-base64=<json>]"
    );
    println!("  protheus-ops strategy-store-kernel gc-profiles [--payload-base64=<json>]");
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
            .map_err(|err| format!("strategy_store_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("strategy_store_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("strategy_store_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("strategy_store_kernel_payload_decode_failed:{err}"));
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

fn as_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(v)) => v.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn clamp_number(value: Option<&Value>, lo: f64, hi: f64, fallback: f64) -> f64 {
    let raw = as_f64(value).unwrap_or(fallback);
    if !raw.is_finite() {
        return fallback;
    }
    raw.clamp(lo, hi)
}

fn clamp_i64(value: Option<&Value>, lo: i64, hi: i64, fallback: i64) -> i64 {
    let raw = as_f64(value).unwrap_or(fallback as f64);
    if !raw.is_finite() {
        return fallback;
    }
    raw.floor().clamp(lo as f64, hi as f64) as i64
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

fn hash16(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::new();
    for byte in digest.iter().take(8) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn stable_uid(seed: &str, prefix: &str, length: usize) -> String {
    let body = {
        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        let digest = hasher.finalize();
        let mut hex = String::new();
        for byte in digest {
            hex.push_str(&format!("{byte:02x}"));
        }
        hex
    };
    let mut out = normalize_tag(prefix).replace('-', "");
    let body_len = length.saturating_sub(out.len()).max(8);
    out.push_str(&body[..body_len.min(body.len())]);
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

fn parse_ts_ms(raw: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn json_string(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

fn workspace_root(root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("PROTHEUS_WORKSPACE_ROOT") {
        let raw = raw.trim();
        if !raw.is_empty() {
            return PathBuf::from(raw);
        }
    }
    root.to_path_buf()
}

fn runtime_root(root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("PROTHEUS_RUNTIME_ROOT") {
        let raw = raw.trim();
        if !raw.is_empty() {
            return PathBuf::from(raw);
        }
    }
    workspace_root(root).join("client").join("runtime")
}

fn default_abs_path(root: &Path) -> PathBuf {
    runtime_root(root).join("adaptive").join(DEFAULT_REL_PATH)
}

fn store_abs_path(root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("STRATEGY_STORE_PATH") {
        let raw = raw.trim();
        if !raw.is_empty() {
            let candidate = PathBuf::from(raw);
            if candidate.is_absolute() {
                return candidate;
            }
            return workspace_root(root).join(candidate);
        }
    }
    default_abs_path(root)
}

fn mutation_log_path(root: &Path) -> PathBuf {
    runtime_root(root).join("local/state/security/adaptive_mutations.jsonl")
}

fn pointer_index_path(root: &Path) -> PathBuf {
    workspace_root(root).join(POINTER_INDEX_REL)
}

fn pointers_path(root: &Path) -> PathBuf {
    workspace_root(root).join(POINTERS_REL)
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "strategy_store_kernel_create_dir_failed:{}:{err}",
                parent.display()
            )
        })?;
    }
    Ok(())
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let temp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let payload = serde_json::to_string_pretty(value)
        .map_err(|err| format!("strategy_store_kernel_encode_json_failed:{err}"))?;
    let mut file = fs::File::create(&temp).map_err(|err| {
        format!(
            "strategy_store_kernel_create_tmp_failed:{}:{err}",
            temp.display()
        )
    })?;
    file.write_all(payload.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|err| {
            format!(
                "strategy_store_kernel_write_tmp_failed:{}:{err}",
                temp.display()
            )
        })?;
    fs::rename(&temp, path).map_err(|err| {
        format!(
            "strategy_store_kernel_rename_tmp_failed:{}:{err}",
            path.display()
        )
    })
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| {
            format!(
                "strategy_store_kernel_open_jsonl_failed:{}:{err}",
                path.display()
            )
        })?;
    let encoded = serde_json::to_string(row)
        .map_err(|err| format!("strategy_store_kernel_encode_jsonl_failed:{err}"))?;
    file.write_all(encoded.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|err| {
            format!(
                "strategy_store_kernel_append_jsonl_failed:{}:{err}",
                path.display()
            )
        })
}

fn read_json(path: &Path) -> Value {
    let Ok(raw) = fs::read_to_string(path) else {
        return Value::Null;
    };
    serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null)
}

fn actor_from_meta(meta: Option<&Map<String, Value>>) -> String {
    let raw = meta
        .and_then(|m| m.get("actor"))
        .map(|v| clean_text(Some(v), 80))
        .unwrap_or_default();
    if !raw.is_empty() {
        return raw;
    }
    std::env::var("USER")
        .ok()
        .map(|v| v.chars().take(80).collect::<String>())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn source_from_meta(meta: Option<&Map<String, Value>>) -> String {
    meta.and_then(|m| m.get("source"))
        .map(|v| clean_text(Some(v), 120))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "core/layer1/memory_runtime/adaptive/strategy_store.ts".to_string())
}

fn reason_from_meta(meta: Option<&Map<String, Value>>, fallback: &str) -> String {
    meta.and_then(|m| m.get("reason"))
        .map(|v| clean_text(Some(v), 160))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn append_mutation_log(
    root: &Path,
    op: &str,
    rel_path: &str,
    value: Option<&Value>,
    meta: Option<&Map<String, Value>>,
    reason_fallback: &str,
) -> Result<(), String> {
    let row = json!({
        "ts": now_iso(),
        "op": op,
        "rel_path": rel_path,
        "actor": actor_from_meta(meta),
        "source": source_from_meta(meta),
        "reason": reason_from_meta(meta, reason_fallback),
        "value_hash": value.map(json_string).map(|v| hash16(&v)).unwrap_or_default(),
    });
    append_jsonl(&mutation_log_path(root), &row)
}

fn pointer_index_load(root: &Path) -> Value {
    let raw = read_json(&pointer_index_path(root));
    if raw.is_object() {
        raw
    } else {
        json!({"version": "1.0", "pointers": {}})
    }
}

fn pointer_index_save(root: &Path, index: &Value) -> Result<(), String> {
    let pointers = index.get("pointers").cloned().unwrap_or_else(|| json!({}));
    write_json_atomic(
        &pointer_index_path(root),
        &json!({
            "version": "1.0",
            "updated_ts": now_iso(),
            "pointers": pointers,
        }),
    )
}

