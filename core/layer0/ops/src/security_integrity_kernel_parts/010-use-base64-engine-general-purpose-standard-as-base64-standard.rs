// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use regex::Regex;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

const DEFAULT_POLICY_REL: &str = "config/security_integrity_policy.json";
const DEFAULT_LOG_REL: &str = "local/state/security/integrity_violations.jsonl";

#[derive(Clone, Debug)]
struct IntegrityPolicy {
    version: String,
    target_roots: Vec<String>,
    target_extensions: Vec<String>,
    protected_files: Vec<String>,
    exclude_paths: Vec<String>,
    hashes: BTreeMap<String, String>,
    sealed_at: Option<String>,
    sealed_by: Option<String>,
    last_approval_note: Option<String>,
}

trait StringFallback {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl StringFallback for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.trim().is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

fn usage() {
    println!("security-integrity-kernel commands:");
    println!(
        "  protheus-ops security-integrity-kernel <load-policy|collect-present-files|verify|seal|append-event> [--payload-base64=<base64_json>]"
    );
}

fn receipt_envelope(kind: &str, ok: bool) -> Value {
    let ts = now_iso();
    json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
    })
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
            .map_err(|err| format!("security_integrity_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("security_integrity_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("security_integrity_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("security_integrity_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    let mut out = as_string(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn as_string_vec(value: Option<&Value>) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(Value::Array(items)) = value {
        for item in items {
            let raw = as_string(Some(item)).replace('\\', "/");
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !out.iter().any(|existing| existing == trimmed) {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

fn workspace_root(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let explicit = clean_text(payload.get("workspace_root"), 520);
    if !explicit.is_empty() {
        return PathBuf::from(explicit);
    }
    let explicit_root = clean_text(payload.get("root"), 520);
    if !explicit_root.is_empty() {
        return PathBuf::from(explicit_root);
    }
    if let Ok(raw) = std::env::var("PROTHEUS_WORKSPACE_ROOT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    root.to_path_buf()
}

fn runtime_root(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Ok(raw) = std::env::var("PROTHEUS_RUNTIME_ROOT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let workspace = workspace_root(root, payload);
    let candidate = workspace.join("client").join("runtime");
    if candidate.exists() {
        candidate
    } else {
        workspace
    }
}

fn rel_from_runtime(runtime_root: &Path, candidate: &str) -> String {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let path = PathBuf::from(trimmed);
    let rel = if path.is_absolute() {
        match path.strip_prefix(runtime_root) {
            Ok(v) => v.to_string_lossy().replace('\\', "/"),
            Err(_) => trimmed.replace('\\', "/"),
        }
    } else {
        trimmed.replace('\\', "/")
    };
    rel.trim_start_matches("./").to_string()
}

fn resolve_path(runtime_root: &Path, explicit: &str, fallback_rel: &str) -> PathBuf {
    let trimmed = explicit.trim();
    if trimmed.is_empty() {
        return runtime_root.join(fallback_rel);
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        runtime_root.join(trimmed)
    }
}

fn read_json_or_default(file_path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(file_path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json_atomic(file_path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("security_integrity_kernel_create_dir_failed:{err}"))?;
    }
    let tmp_path = file_path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    fs::write(
        &tmp_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value)
                .map_err(|err| format!("security_integrity_kernel_encode_failed:{err}"))?
        ),
    )
    .map_err(|err| format!("security_integrity_kernel_write_failed:{err}"))?;
    fs::rename(&tmp_path, file_path)
        .map_err(|err| format!("security_integrity_kernel_rename_failed:{err}"))?;
    Ok(())
}

fn append_jsonl(file_path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("security_integrity_kernel_create_dir_failed:{err}"))?;
    }
    let mut handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .map_err(|err| format!("security_integrity_kernel_open_failed:{err}"))?;
    handle
        .write_all(
            format!(
                "{}\n",
                serde_json::to_string(row).unwrap_or_else(|_| "null".to_string())
            )
            .as_bytes(),
        )
        .map_err(|err| format!("security_integrity_kernel_append_failed:{err}"))?;
    Ok(())
}

fn sha256_file(file_path: &Path) -> Result<String, String> {
    let bytes = fs::read(file_path)
        .map_err(|err| format!("security_integrity_kernel_read_file_failed:{err}"))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hex::encode(hasher.finalize()))
}
