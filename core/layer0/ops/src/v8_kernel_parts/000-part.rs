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

fn env_nonempty_path(env_key: &str) -> Option<PathBuf> {
    std::env::var(env_key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

pub fn scoped_state_root(root: &Path, env_key: &str, scope: &str) -> PathBuf {
    if let Some(path) = env_nonempty_path(env_key) {
        return path;
    }
    crate::core_state_root(root).join("ops").join(scope)
}

pub fn state_root_from_env_or(root: &Path, env_key: &str, default_rel: &[&str]) -> PathBuf {
    if let Some(path) = env_nonempty_path(env_key) {
        return path;
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
        let mut unhashed = self.clone();
        if let Some(obj) = unhashed.as_object_mut() {
            obj.remove("receipt_hash");
        }
        self["receipt_hash"] = Value::String(deterministic_receipt_hash(&unhashed));
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
