// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::{clean, deterministic_receipt_hash, now_iso};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const DEFAULT_RECEIPT_HISTORY_MAX_BYTES: u64 = 2 * 1024 * 1024;
const DEFAULT_RECEIPT_BINARY_MAX_BYTES: u64 = 2 * 1024 * 1024;
const RETENTION_MAX_BYTES_CAP: u64 = 1024 * 1024 * 1024;
const RETENTION_TAIL_SLACK_BYTES: u64 = 8 * 1024;

pub fn scoped_state_root(root: &Path, env_key: &str, scope: &str) -> PathBuf {
    if let Ok(v) = std::env::var(env_key) {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    crate::core_state_root(root).join("ops").join(scope)
}

pub fn state_root_from_env_or(root: &Path, env_key: &str, default_rel: &[&str]) -> PathBuf {
    if let Ok(v) = std::env::var(env_key) {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    default_rel
        .iter()
        .fold(root.to_path_buf(), |path, segment| path.join(segment))
}

pub fn latest_path(root: &Path, env_key: &str, scope: &str) -> PathBuf {
    scoped_state_root(root, env_key, scope).join("latest.json")
}

pub fn history_path(root: &Path, env_key: &str, scope: &str) -> PathBuf {
    scoped_state_root(root, env_key, scope).join("history.jsonl")
}

pub fn read_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

pub fn read_jsonl(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let payload = serde_json::to_string_pretty(value)
        .map_err(|err| format!("encode_json_failed:{}:{err}", path.display()))?;
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    fs::write(&tmp, format!("{payload}\n"))
        .map_err(|err| format!("write_tmp_failed:{}:{err}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|err| {
        format!(
            "rename_tmp_failed:{}:{}:{err}",
            tmp.display(),
            path.display()
        )
    })
}

pub fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    append_jsonl_with_limits(
        path,
        value,
        receipt_history_max_bytes(),
        receipt_binary_queue_enabled(),
        receipt_binary_queue_max_bytes(),
    )
}

pub fn append_jsonl_without_binary_queue(path: &Path, value: &Value) -> Result<(), String> {
    append_jsonl_with_limits(path, value, receipt_history_max_bytes(), false, 0)
}

pub fn append_jsonl_with_limits(
    path: &Path,
    value: &Value,
    history_max_bytes: u64,
    binary_queue_enabled: bool,
    binary_max_bytes: u64,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let line = serde_json::to_string(value)
        .map_err(|err| format!("encode_jsonl_failed:{}:{err}", path.display()))?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("open_jsonl_failed:{}:{err}", path.display()))?;
    writeln!(file, "{line}")
        .map_err(|err| format!("append_jsonl_failed:{}:{err}", path.display()))?;

    let queue_path = if binary_queue_enabled {
        let queue = receipt_binary_queue_path(path);
        append_binary_queue(&queue, value)?;
        Some(queue)
    } else {
        None
    };

    let history_trimmed = enforce_jsonl_tail_limit(path, history_max_bytes)?;
    if let Some(queue) = queue_path {
        enforce_binary_queue_limit(path, &queue, binary_max_bytes, history_trimmed)?;
    }
    Ok(())
}

fn receipt_binary_queue_enabled() -> bool {
    match std::env::var("PROTHEUS_RECEIPT_BINARY_QUEUE") {
        Ok(raw) => !matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "off" | "no"
        ),
        Err(_) => true,
    }
}

fn parse_retention_max_bytes_env(name: &str, fallback: u64) -> u64 {
    match std::env::var(name) {
        Ok(raw) => match raw.trim().parse::<u64>() {
            Ok(0) => u64::MAX,
            Ok(v) => v.min(RETENTION_MAX_BYTES_CAP),
            Err(_) => fallback,
        },
        Err(_) => fallback,
    }
}

fn receipt_history_max_bytes() -> u64 {
    parse_retention_max_bytes_env(
        "PROTHEUS_RECEIPT_HISTORY_MAX_BYTES",
        DEFAULT_RECEIPT_HISTORY_MAX_BYTES,
    )
}

fn receipt_binary_queue_max_bytes() -> u64 {
    parse_retention_max_bytes_env(
        "PROTHEUS_RECEIPT_BINARY_QUEUE_MAX_BYTES",
        DEFAULT_RECEIPT_BINARY_MAX_BYTES,
    )
}

pub fn receipt_binary_queue_path(history_jsonl_path: &Path) -> PathBuf {
    let parent = history_jsonl_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let stem = history_jsonl_path
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("history");
    parent.join(format!("{stem}.bin"))
}

pub fn append_binary_queue(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let encoded = serde_json::to_vec(value)
        .map_err(|err| format!("encode_binary_receipt_failed:{}:{err}", path.display()))?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("open_binary_receipt_failed:{}:{err}", path.display()))?;
    let len = (encoded.len() as u32).to_le_bytes();
    file.write_all(&len)
        .and_then(|_| file.write_all(&encoded))
        .map_err(|err| format!("append_binary_receipt_failed:{}:{err}", path.display()))
}

fn enforce_jsonl_tail_limit(path: &Path, max_bytes: u64) -> Result<bool, String> {
    if max_bytes == u64::MAX {
        return Ok(false);
    }
    let current = fs::metadata(path)
        .map(|meta| meta.len())
        .map_err(|err| format!("jsonl_metadata_failed:{}:{err}", path.display()))?;
    if current <= max_bytes {
        return Ok(false);
    }

    let mut file = fs::File::open(path)
        .map_err(|err| format!("open_jsonl_failed:{}:{err}", path.display()))?;
    let read_len = current.min(max_bytes.saturating_add(RETENTION_TAIL_SLACK_BYTES));
    if current > read_len {
        file.seek(SeekFrom::End(-(read_len as i64)))
            .map_err(|err| format!("seek_jsonl_failed:{}:{err}", path.display()))?;
    }
    let mut buffer = Vec::<u8>::new();
    file.read_to_end(&mut buffer)
        .map_err(|err| format!("read_jsonl_failed:{}:{err}", path.display()))?;

    let mut start = 0usize;
    if current > read_len {
        if let Some(pos) = buffer.iter().position(|byte| *byte == b'\n') {
            start = pos.saturating_add(1);
        }
    }
    let retained = if start < buffer.len() {
        &buffer[start..]
    } else {
        &[][..]
    };

    atomic_write_bytes(path, retained)?;
    Ok(true)
}

fn enforce_binary_queue_limit(
    history_jsonl_path: &Path,
    queue_path: &Path,
    max_bytes: u64,
    force_rebuild: bool,
) -> Result<(), String> {
    let queue_too_large = if max_bytes == u64::MAX {
        false
    } else {
        fs::metadata(queue_path)
            .map(|meta| meta.len() > max_bytes)
            .unwrap_or(false)
    };
    if !force_rebuild && !queue_too_large {
        return Ok(());
    }
    rebuild_binary_queue_from_jsonl(history_jsonl_path, queue_path, max_bytes)
}

fn rebuild_binary_queue_from_jsonl(
    history_jsonl_path: &Path,
    queue_path: &Path,
    max_bytes: u64,
) -> Result<(), String> {
    let rows = read_jsonl(history_jsonl_path);
    if rows.is_empty() {
        if queue_path.exists() {
            fs::remove_file(queue_path).map_err(|err| {
                format!("remove_binary_queue_failed:{}:{err}", queue_path.display())
            })?;
        }
        return Ok(());
    }

    let mut frames = Vec::<Vec<u8>>::with_capacity(rows.len());
    let mut total = 0u64;
    for row in rows {
        let encoded = serde_json::to_vec(&row).map_err(|err| {
            format!(
                "encode_binary_receipt_failed:{}:{err}",
                queue_path.display()
            )
        })?;
        let mut frame = Vec::<u8>::with_capacity(4 + encoded.len());
        frame.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
        frame.extend_from_slice(&encoded);
        total = total.saturating_add(frame.len() as u64);
        frames.push(frame);
    }

    let mut keep_from = 0usize;
    if max_bytes != u64::MAX && total > max_bytes {
        let mut running = 0u64;
        keep_from = frames.len().saturating_sub(1);
        for idx in (0..frames.len()).rev() {
            let frame_len = frames[idx].len() as u64;
            if running == 0 || running.saturating_add(frame_len) <= max_bytes {
                running = running.saturating_add(frame_len);
                keep_from = idx;
            } else {
                break;
            }
        }
    }

    let mut payload = Vec::<u8>::new();
    for frame in frames.into_iter().skip(keep_from) {
        payload.extend_from_slice(&frame);
    }
    atomic_write_bytes(queue_path, &payload)
}

fn atomic_write_bytes(path: &Path, payload: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    fs::write(&tmp, payload).map_err(|err| format!("write_tmp_failed:{}:{err}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|err| {
        format!(
            "rename_tmp_failed:{}:{}:{err}",
            tmp.display(),
            path.display()
        )
    })
}

pub fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

pub trait ReceiptJsonExt {
    fn with_receipt_hash(self) -> Value;
    fn set_receipt_hash(&mut self);
}

impl ReceiptJsonExt for Value {
    fn with_receipt_hash(mut self) -> Value {
        self.set_receipt_hash();
        self
    }

    fn set_receipt_hash(&mut self) {
        self["receipt_hash"] = Value::String(deterministic_receipt_hash(self));
    }
}

pub fn parse_bool(raw: Option<&String>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

pub fn parse_bool_str(raw: Option<&str>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

pub fn parse_f64(raw: Option<&String>, fallback: f64) -> f64 {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
}

pub fn parse_f64_str(raw: Option<&str>, fallback: f64) -> f64 {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
}

pub fn parse_u64(raw: Option<&String>, fallback: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
}

pub fn parse_u64_str(raw: Option<&str>, fallback: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
}

pub fn parse_i64(raw: Option<&String>, fallback: i64) -> i64 {
    raw.and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(fallback)
}

pub fn parse_i64_clamped(raw: Option<&String>, fallback: i64, lo: i64, hi: i64) -> i64 {
    parse_i64(raw, fallback).clamp(lo, hi)
}

pub fn parse_json_or_empty(raw: Option<&String>) -> Value {
    raw.and_then(|s| serde_json::from_str::<Value>(s).ok())
        .unwrap_or_else(|| json!({}))
}

pub fn date_or_today(raw: Option<&String>) -> String {
    let candidate = raw.map(|v| v.trim().to_string()).unwrap_or_default();
    if !candidate.is_empty() && chrono::NaiveDate::parse_from_str(&candidate, "%Y-%m-%d").is_ok() {
        return candidate;
    }
    now_iso().chars().take(10).collect()
}

pub fn parse_i64_str(raw: Option<&str>, fallback: i64) -> i64 {
    raw.and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(fallback)
}

pub fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let needle = format!("--{key}");
    for idx in 0..argv.len() {
        let token = &argv[idx];
        if token == &needle {
            return argv.get(idx + 1).cloned();
        }
        let prefix = format!("{needle}=");
        if let Some(value) = token.strip_prefix(&prefix) {
            return Some(value.to_string());
        }
    }
    None
}

pub fn load_json_or(root: &Path, rel: &str, fallback: Value) -> Value {
    read_json(&root.join(rel)).unwrap_or(fallback)
}

pub fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let mut out = serde_json::Map::new();
            for key in keys {
                if let Some(v) = map.get(&key) {
                    out.insert(key, canonicalize_json(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(rows) => Value::Array(rows.iter().map(canonicalize_json).collect()),
        _ => value.clone(),
    }
}

pub fn canonical_json_string(value: &Value) -> String {
    serde_json::to_string(&canonicalize_json(value)).unwrap_or_else(|_| "null".to_string())
}

pub fn conduit_bypass_requested(flags: &HashMap<String, String>) -> bool {
    parse_bool(flags.get("bypass"), false)
        || parse_bool(flags.get("direct"), false)
        || parse_bool(flags.get("unsafe-client-route"), false)
        || parse_bool(flags.get("client-bypass"), false)
}

pub fn conduit_claim_rows(
    action: &str,
    bypass_requested: bool,
    claim: &str,
    claim_ids: &[&str],
) -> Vec<Value> {
    claim_ids
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "claim": clean(claim, 240),
                "evidence": {
                    "action": clean(action, 120),
                    "bypass_requested": bypass_requested
                }
            })
        })
        .collect()
}

pub fn build_conduit_enforcement(
    root: &Path,
    env_key: &str,
    scope: &str,
    strict: bool,
    action: &str,
    receipt_type: &str,
    required_path: &str,
    bypass_requested: bool,
    claim_evidence: Vec<Value>,
) -> Value {
    let ok = !bypass_requested;
    let mut out = json!({
        "ok": if strict { ok } else { true },
        "type": clean(receipt_type, 120),
        "action": clean(action, 120),
        "required_path": clean(required_path, 240),
        "bypass_requested": bypass_requested,
        "errors": if ok { Value::Array(Vec::new()) } else { json!(["conduit_bypass_rejected"]) },
        "claim_evidence": claim_evidence
    });
    out.set_receipt_hash();
    let _ = append_jsonl(
        &scoped_state_root(root, env_key, scope)
            .join("conduit")
            .join("history.jsonl"),
        &out,
    );
    out
}

pub fn build_plane_conduit_enforcement(
    root: &Path,
    env_key: &str,
    scope: &str,
    strict: bool,
    action: &str,
    receipt_type: &str,
    required_path: &str,
    bypass_requested: bool,
    claim: &str,
    claim_ids: &[&str],
) -> Value {
    build_conduit_enforcement(
        root,
        env_key,
        scope,
        strict,
        action,
        receipt_type,
        required_path,
        bypass_requested,
        conduit_claim_rows(action, bypass_requested, claim, claim_ids),
    )
}

pub fn attach_conduit(mut payload: Value, conduit: Option<&Value>) -> Value {
    if let Some(gate) = conduit {
        payload["conduit_enforcement"] = gate.clone();
        let mut claims = payload
            .get("claim_evidence")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if let Some(rows) = gate.get("claim_evidence").and_then(Value::as_array) {
            claims.extend(rows.iter().cloned());
        }
        if !claims.is_empty() {
            payload["claim_evidence"] = Value::Array(claims);
        }
    }
    payload.set_receipt_hash();
    payload
}

pub fn sha256_hex_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn sha256_hex_str(value: &str) -> String {
    sha256_hex_bytes(value.as_bytes())
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("read_file_failed:{}:{err}", path.display()))?;
    Ok(sha256_hex_bytes(&bytes))
}

pub fn keyed_digest_hex(secret: &str, payload: &Value) -> String {
    let rendered = serde_json::to_string(payload).unwrap_or_default();
    sha256_hex_str(&format!("{}:{}", clean(secret, 4096), rendered))
}

pub fn next_chain_hash(prev_hash: Option<&str>, payload: &Value) -> String {
    let prev = prev_hash.unwrap_or("genesis");
    let rendered = serde_json::to_string(payload).unwrap_or_default();
    sha256_hex_str(&format!("{prev}|{rendered}"))
}

pub fn deterministic_merkle_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return sha256_hex_str("merkle:empty");
    }
    let mut level = leaves
        .iter()
        .map(|leaf| sha256_hex_str(&format!("leaf:{leaf}")))
        .collect::<Vec<_>>();
    while level.len() > 1 {
        let mut next = Vec::new();
        let mut i = 0usize;
        while i < level.len() {
            let left = &level[i];
            let right = if i + 1 < level.len() {
                &level[i + 1]
            } else {
                &level[i]
            };
            next.push(sha256_hex_str(&format!("node:{left}:{right}")));
            i += 2;
        }
        level = next;
    }
    level[0].clone()
}

pub fn merkle_proof(leaves: &[String], index: usize) -> Vec<Value> {
    if leaves.is_empty() || index >= leaves.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut idx = index;
    let mut level = leaves
        .iter()
        .map(|leaf| sha256_hex_str(&format!("leaf:{leaf}")))
        .collect::<Vec<_>>();

    while level.len() > 1 {
        let sibling_idx = if idx % 2 == 0 {
            idx + 1
        } else {
            idx.saturating_sub(1)
        };
        let sibling = if sibling_idx < level.len() {
            level[sibling_idx].clone()
        } else {
            level[idx].clone()
        };
        out.push(json!({
            "level_size": level.len(),
            "index": idx,
            "sibling_index": sibling_idx.min(level.len().saturating_sub(1)),
            "sibling_hash": sibling
        }));

        let mut next = Vec::new();
        let mut i = 0usize;
        while i < level.len() {
            let left = &level[i];
            let right = if i + 1 < level.len() {
                &level[i + 1]
            } else {
                &level[i]
            };
            next.push(sha256_hex_str(&format!("node:{left}:{right}")));
            i += 2;
        }
        idx /= 2;
        level = next;
    }

    out
}

pub fn write_receipt(
    root: &Path,
    env_key: &str,
    scope: &str,
    mut payload: Value,
) -> Result<Value, String> {
    let latest = latest_path(root, env_key, scope);
    let history = history_path(root, env_key, scope);
    payload["ts"] = Value::String(now_iso());
    payload.set_receipt_hash();
    write_json(&latest, &payload)?;
    append_jsonl(&history, &payload)?;
    Ok(payload)
}

pub fn emit_plane_receipt(
    root: &Path,
    env_key: &str,
    scope: &str,
    error_type: &str,
    payload: Value,
) -> i32 {
    match write_receipt(root, env_key, scope, payload) {
        Ok(out) => {
            print_json(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_json(&json!({
                "ok": false,
                "type": clean(error_type, 120),
                "error": clean(err, 240)
            }));
            1
        }
    }
}

pub fn emit_attached_plane_receipt(
    root: &Path,
    env_key: &str,
    scope: &str,
    strict: bool,
    payload: Value,
    conduit: Option<&Value>,
) -> i32 {
    let out = attach_conduit(payload, conduit);
    let _ = write_json(&latest_path(root, env_key, scope), &out);
    let _ = append_jsonl(&history_path(root, env_key, scope), &out);
    print_json(&out);
    if strict && !out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        1
    } else if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    }
}

pub fn plane_status(root: &Path, env_key: &str, scope: &str, status_type: &str) -> Value {
    json!({
        "ok": true,
        "type": clean(status_type, 120),
        "lane": "core/layer0/ops",
        "latest_path": latest_path(root, env_key, scope).display().to_string(),
        "latest": read_json(&latest_path(root, env_key, scope))
    })
}

pub fn split_csv_clean(raw: &str, max_len: usize) -> Vec<String> {
    raw.split(',')
        .map(|row| clean(row, max_len))
        .filter(|row| !row.is_empty())
        .collect()
}

pub fn parse_csv_flag(flags: &HashMap<String, String>, key: &str, max_len: usize) -> Vec<String> {
    flags
        .get(key)
        .map(|v| split_csv_clean(v, max_len))
        .unwrap_or_default()
}

pub fn parse_csv_or_file(
    root: &Path,
    flags: &HashMap<String, String>,
    csv_key: &str,
    file_key: &str,
    max_len: usize,
) -> Vec<String> {
    let mut values = parse_csv_flag(flags, csv_key, max_len);
    let Some(rel_or_abs) = flags.get(file_key) else {
        return values;
    };
    let path = if Path::new(rel_or_abs).is_absolute() {
        PathBuf::from(rel_or_abs)
    } else {
        root.join(rel_or_abs)
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return values;
    };
    if raw.trim_start().starts_with('[') {
        if let Ok(parsed_json) = serde_json::from_str::<Value>(&raw) {
            if let Some(rows) = parsed_json.as_array() {
                for row in rows {
                    if let Some(text) = row.as_str() {
                        let cleaned = clean(text, max_len);
                        if !cleaned.is_empty() {
                            values.push(cleaned);
                        }
                    }
                }
            }
        }
        return values;
    }
    values.extend(split_csv_clean(&raw.replace('\n', ","), max_len));
    values
}

pub fn parse_csv_or_file_unique(
    root: &Path,
    flags: &HashMap<String, String>,
    csv_key: &str,
    file_key: &str,
    max_len: usize,
) -> Vec<String> {
    let mut values = parse_csv_or_file(root, flags, csv_key, file_key, max_len);
    values.sort();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn decode_binary_rows(path: &Path) -> Vec<Value> {
        let Ok(bytes) = fs::read(path) else {
            return Vec::new();
        };
        let mut out = Vec::<Value>::new();
        let mut idx = 0usize;
        while idx + 4 <= bytes.len() {
            let len =
                u32::from_le_bytes([bytes[idx], bytes[idx + 1], bytes[idx + 2], bytes[idx + 3]])
                    as usize;
            idx += 4;
            if idx + len > bytes.len() {
                break;
            }
            if let Ok(value) = serde_json::from_slice::<Value>(&bytes[idx..idx + len]) {
                out.push(value);
            }
            idx += len;
        }
        out
    }

    #[test]
    fn append_jsonl_with_limits_caps_history_and_binary_queue() {
        let dir = tempdir().expect("tempdir");
        let history_path = dir.path().join("history.jsonl");
        for idx in 0..120 {
            let payload = json!({
                "idx": idx,
                "text": "x".repeat(64)
            });
            append_jsonl_with_limits(&history_path, &payload, 1024, true, 1024).expect("append");
        }

        let history_size = fs::metadata(&history_path).expect("history metadata").len();
        assert!(history_size <= 1024 + RETENTION_TAIL_SLACK_BYTES);

        let history_rows = read_jsonl(&history_path);
        assert!(!history_rows.is_empty());
        assert_eq!(
            history_rows
                .last()
                .and_then(|row| row.get("idx"))
                .and_then(Value::as_i64),
            Some(119)
        );

        let queue_path = receipt_binary_queue_path(&history_path);
        let queue_size = fs::metadata(&queue_path).expect("queue metadata").len();
        assert!(queue_size <= 1024);
        let queue_rows = decode_binary_rows(&queue_path);
        assert!(!queue_rows.is_empty());
        assert_eq!(
            queue_rows
                .last()
                .and_then(|row| row.get("idx"))
                .and_then(Value::as_i64),
            Some(119)
        );
    }

    #[test]
    fn append_jsonl_without_binary_queue_skips_binary_file() {
        let dir = tempdir().expect("tempdir");
        let history_path = dir.path().join("history.jsonl");
        append_jsonl_without_binary_queue(&history_path, &json!({"ok": true})).expect("append");

        let queue_path = receipt_binary_queue_path(&history_path);
        assert!(!queue_path.exists());
    }
}
