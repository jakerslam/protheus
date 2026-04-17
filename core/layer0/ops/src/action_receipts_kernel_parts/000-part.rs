// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

type HmacSha256 = Hmac<Sha256>;

fn usage() {
    println!("action-receipts-kernel commands:");
    println!("  infring-ops action-receipts-kernel now-iso");
    println!("  infring-ops action-receipts-kernel append-jsonl --payload-base64=<json>");
    println!("  infring-ops action-receipts-kernel with-receipt-contract --payload-base64=<json>");
    println!("  infring-ops action-receipts-kernel write-contract-receipt --payload-base64=<json>");
    println!(
        "  infring-ops action-receipts-kernel replay-task-lineage --task-id=<id> [--trace-id=<id>] [--limit=<n>] [--scan-root=<path>] [--sources=<csv_paths>]"
    );
    println!(
        "  infring-ops action-receipts-kernel query-task-lineage --task-id=<id> [--trace-id=<id>] [--limit=<n>] [--scan-root=<path>] [--sources=<csv_paths>]"
    );
}

fn with_receipt_hash(mut value: Value) -> Value {
    value["receipt_hash"] = Value::String(deterministic_receipt_hash(&value));
    value
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    with_receipt_hash(json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload,
    }))
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    with_receipt_hash(json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true,
    }))
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
            .map_err(|err| format!("action_receipts_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("action_receipts_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("action_receipts_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("action_receipts_kernel_payload_decode_failed:{err}"));
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

fn resolve_file_path(root: &Path, raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return root.join("local").join("state").join("receipts.jsonl");
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("action_receipts_kernel_create_dir_failed:{err}"))?;
    }
    Ok(())
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("action_receipts_kernel_append_open_failed:{err}"))?;
    file.write_all(
        format!(
            "{}\n",
            serde_json::to_string(row).unwrap_or_else(|_| "null".to_string())
        )
        .as_bytes(),
    )
    .map_err(|err| format!("action_receipts_kernel_append_failed:{err}"))
}

fn chain_state_path(file_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.chain.json", file_path.to_string_lossy()))
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn optional_hmac(hash: &str) -> Result<Option<String>, String> {
    let key = std::env::var("RECEIPT_CHAIN_HMAC_KEY").unwrap_or_default();
    let key = key.trim();
    if key.is_empty() {
        return Ok(None);
    }
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .map_err(|err| format!("action_receipts_kernel_hmac_init_failed:{err}"))?;
    mac.update(hash.as_bytes());
    Ok(Some(hex::encode(mac.finalize().into_bytes())))
}

fn read_chain_state(file_path: &Path) -> (u64, Option<String>) {
    let state_path = chain_state_path(file_path);
    let parsed = fs::read_to_string(state_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}));
    let seq = parsed.get("seq").and_then(Value::as_u64).unwrap_or(0);
    let hash = parsed
        .get("hash")
        .and_then(Value::as_str)
        .map(|row| row.to_string());
    (seq, hash)
}

fn write_chain_state(file_path: &Path, seq: u64, hash: Option<&str>) -> Result<(), String> {
    let state_path = chain_state_path(file_path);
    ensure_parent(&state_path)?;
    let tmp_path = PathBuf::from(format!(
        "{}.tmp-{}",
        state_path.to_string_lossy(),
        std::process::id()
    ));
    fs::write(
        &tmp_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&json!({
                "seq": seq,
                "hash": hash,
                "ts": now_iso(),
            }))
            .map_err(|err| format!("action_receipts_kernel_encode_failed:{err}"))?
        ),
    )
    .map_err(|err| format!("action_receipts_kernel_write_failed:{err}"))?;
    fs::rename(&tmp_path, &state_path)
        .map_err(|err| format!("action_receipts_kernel_rename_failed:{err}"))
}

fn with_receipt_contract_value(record: &Value, attempted: bool, verified: bool) -> Value {
    let src = as_object(Some(record)).cloned().unwrap_or_default();
    let mut receipt_contract = as_object(src.get("receipt_contract"))
        .cloned()
        .unwrap_or_default();
    receipt_contract.insert("version".to_string(), Value::String("1.0".to_string()));
    receipt_contract.insert("attempted".to_string(), Value::Bool(attempted));
    receipt_contract.insert("verified".to_string(), Value::Bool(verified));
    receipt_contract.insert("recorded".to_string(), Value::Bool(true));
    let mut out = src;
    out.insert(
        "receipt_contract".to_string(),
        Value::Object(receipt_contract),
    );
    Value::Object(out)
}

fn with_receipt_integrity_value(file_path: &Path, record: &Value) -> Result<Value, String> {
    let src = as_object(Some(record)).cloned().unwrap_or_default();
    let (prev_seq, prev_hash) = read_chain_state(file_path);
    let seq = prev_seq.saturating_add(1);
    let payload_hash = sha256_hex(
        &serde_json::to_string(&Value::Object(src.clone())).unwrap_or_else(|_| "{}".to_string()),
    );
    let link_hash = sha256_hex(&format!(
        "{seq}:{}:{payload_hash}",
        prev_hash.clone().unwrap_or_default()
    ));
    let hmac = optional_hmac(&link_hash)?;

    let mut receipt_contract = as_object(src.get("receipt_contract"))
        .cloned()
        .unwrap_or_default();
    receipt_contract.insert(
        "integrity".to_string(),
        json!({
            "version": "1.0",
            "seq": seq,
            "prev_hash": prev_hash,
            "payload_hash": payload_hash,
            "hash": link_hash,
            "hmac": hmac,
            "ts": now_iso(),
        }),
    );
    let mut out = src;
    out.insert(
        "receipt_contract".to_string(),
        Value::Object(receipt_contract),
    );
    let out_value = Value::Object(out);
    let current_hash = out_value
        .get("receipt_contract")
        .and_then(Value::as_object)
        .and_then(|row| row.get("integrity"))
        .and_then(Value::as_object)
        .and_then(|row| row.get("hash"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    write_chain_state(file_path, seq, Some(&current_hash))?;
    Ok(out_value)
}

fn parse_attempted(payload: &Map<String, Value>) -> bool {
    payload
        .get("attempted")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn parse_verified(payload: &Map<String, Value>) -> bool {
    payload
        .get("verified")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn write_contract_receipt_value(
    root: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let file_path = resolve_file_path(root, &as_str(payload.get("file_path")));
    let record = payload.get("record").cloned().unwrap_or_else(|| json!({}));
    let with_contract =
        with_receipt_contract_value(&record, parse_attempted(payload), parse_verified(payload));
    let with_integrity = with_receipt_integrity_value(&file_path, &with_contract)?;
    append_jsonl(&file_path, &with_integrity)?;
    Ok(json!({
        "ok": true,
        "file_path": file_path.to_string_lossy(),
        "record": with_integrity,
    }))
}

fn parse_lineage_limit(payload: &Map<String, Value>) -> usize {
    payload
        .get("limit")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .filter(|v| *v > 0)
        .unwrap_or(4000)
        .min(50_000)
}

fn parse_scan_root(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let raw = as_str(payload.get("scan_root"));
    if raw.is_empty() {
        return root.to_path_buf();
    }
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn source_paths_from_payload(root: &Path, payload: &Map<String, Value>) -> Vec<PathBuf> {
    let explicit = as_str(payload.get("sources"));
    if explicit.trim().is_empty() {
        return Vec::new();
    }
    explicit
        .split(',')
        .map(|row| resolve_file_path(root, row))
        .filter(|path| path.exists())
        .collect::<Vec<_>>()
}

fn known_lineage_paths(scan_root: &Path) -> Vec<PathBuf> {
    let mut out = vec![
        scan_root
            .join("local")
            .join("state")
            .join("runtime")
            .join("task_runtime")
            .join("verity_receipts.jsonl"),
        scan_root
            .join("local")
            .join("state")
            .join("runtime")
            .join("task_runtime")
            .join("conduit_messages.jsonl"),
        scan_root
            .join("client")
            .join("runtime")
            .join("local")
            .join("state")
            .join("ui")
            .join("infring_dashboard")
            .join("actions")
            .join("history.jsonl"),
        scan_root
            .join("local")
            .join("state")
            .join("attention")
            .join("receipts.jsonl"),
        scan_root
            .join("local")
            .join("state")
            .join("stomach")
            .join("receipts.jsonl"),
        scan_root
            .join("local")
            .join("state")
            .join("ops")
            .join("verity")
            .join("receipts.jsonl"),
    ];
    out.retain(|path| path.exists());
    out
}

fn is_replay_candidate_name(name: &str) -> bool {
    matches!(
        name,
        "history.jsonl"
            | "receipts.jsonl"
            | "verity_receipts.jsonl"
            | "conduit_messages.jsonl"
            | "protocol_step_receipts.jsonl"
            | "protocol_history.jsonl"
    )
}

fn should_skip_replay_path(path: &Path) -> bool {
    let lowered = path.to_string_lossy().to_ascii_lowercase();
    lowered.contains("/assimilation/isolated/")
        || lowered.contains("/assimilation/burned/")
        || lowered.contains("/node_modules/")
        || lowered.contains("/.git/")
        || lowered.contains("/target/")
}
