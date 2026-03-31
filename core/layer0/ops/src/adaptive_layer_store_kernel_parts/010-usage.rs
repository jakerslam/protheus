// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::Utc;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const WRITE_LOCK_TIMEOUT_MS: u64 = 8_000;
const WRITE_LOCK_RETRY_MS: u64 = 15;
const WRITE_LOCK_STALE_MS: u64 = 30_000;
const MISSING_HASH_SENTINEL: &str = "__missing__";

struct WriteLock {
    #[allow(dead_code)]
    file: fs::File,
    lock_path: PathBuf,
    waited_ms: u64,
}

fn usage() {
    println!("adaptive-layer-store-kernel commands:");
    println!("  protheus-ops adaptive-layer-store-kernel paths [--payload-base64=<json>]");
    println!("  protheus-ops adaptive-layer-store-kernel is-within-root --payload-base64=<json>");
    println!("  protheus-ops adaptive-layer-store-kernel resolve-path --payload-base64=<json>");
    println!("  protheus-ops adaptive-layer-store-kernel read-json --payload-base64=<json>");
    println!("  protheus-ops adaptive-layer-store-kernel ensure-json --payload-base64=<json>");
    println!("  protheus-ops adaptive-layer-store-kernel set-json --payload-base64=<json>");
    println!("  protheus-ops adaptive-layer-store-kernel delete-path --payload-base64=<json>");
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
            .map_err(|err| format!("adaptive_layer_store_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("adaptive_layer_store_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("adaptive_layer_store_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("adaptive_layer_store_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
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

fn normalize_path_string(raw: &str) -> String {
    raw.replace('\\', "/")
}

fn normalize_adaptive_rel(raw: &str) -> String {
    normalize_path_string(raw)
        .trim_start_matches("./")
        .trim_start_matches('/')
        .trim_start_matches("adaptive/")
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn relative_within(root_path: &Path, target_path: &Path) -> Option<String> {
    let rel = target_path
        .strip_prefix(root_path)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    if rel == ".." || rel.starts_with("../") {
        None
    } else {
        Some(rel)
    }
}

fn workspace_root(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let explicit = clean_text(payload.get("workspace_root"), 520);
    if !explicit.is_empty() {
        return PathBuf::from(explicit);
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
    let explicit = clean_text(payload.get("runtime_root"), 520);
    if !explicit.is_empty() {
        return PathBuf::from(explicit);
    }
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

fn adaptive_root(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    runtime_root(root, payload).join("adaptive")
}

fn adaptive_runtime_root(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    runtime_root(root, payload).join("local").join("adaptive")
}

fn mutation_log_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    runtime_root(root, payload)
        .join("local")
        .join("state")
        .join("security")
        .join("adaptive_mutations.jsonl")
}

fn adaptive_pointers_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    runtime_root(root, payload)
        .join("local")
        .join("state")
        .join("memory")
        .join("adaptive_pointers.jsonl")
}

fn adaptive_pointer_index_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    runtime_root(root, payload)
        .join("local")
        .join("state")
        .join("memory")
        .join("adaptive_pointer_index.json")
}

fn runtime_adaptive_rel_allowed(rel: &str) -> bool {
    matches!(
        rel,
        "sensory/eyes/catalog.json" | "sensory/eyes/focus_triggers.json"
    )
}

pub(crate) fn resolve_adaptive_path(
    root: &Path,
    payload: &Map<String, Value>,
    target_path: &str,
) -> Result<(PathBuf, String), String> {
    let raw = target_path.trim();
    if raw.is_empty() {
        return Err("adaptive_store: target must be file path under adaptive/".to_string());
    }
    let adaptive_root = adaptive_root(root, payload);
    let adaptive_runtime_root = adaptive_runtime_root(root, payload);
    let target = PathBuf::from(raw);

    if target.is_absolute() {
        let abs_path = target.canonicalize().unwrap_or(target.clone());
        if let Some(source_rel) = relative_within(&adaptive_root, &abs_path) {
            if source_rel.is_empty() {
                return Err("adaptive_store: target must be file path under adaptive/".to_string());
            }
            let rel = normalize_adaptive_rel(&source_rel);
            if runtime_adaptive_rel_allowed(&rel) {
                return Ok((adaptive_runtime_root.join(&rel), rel));
            }
            return Ok((abs_path, rel));
        }
        if let Some(runtime_rel) = relative_within(&adaptive_runtime_root, &abs_path) {
            if runtime_rel.is_empty() {
                return Err(format!(
                    "adaptive_store: target outside adaptive roots: {}",
                    abs_path.display()
                ));
            }
            let rel = normalize_adaptive_rel(&runtime_rel);
            if !runtime_adaptive_rel_allowed(&rel) {
                return Err(format!(
                    "adaptive_store: runtime path not allowed: {}",
                    abs_path.display()
                ));
            }
            return Ok((abs_path, rel));
        }
        return Err(format!(
            "adaptive_store: target outside adaptive roots: {}",
            abs_path.display()
        ));
    }

    let rel = normalize_adaptive_rel(raw);
    if rel.is_empty() {
        return Err("adaptive_store: target must be file path under adaptive/".to_string());
    }
    if runtime_adaptive_rel_allowed(&rel) {
        return Ok((adaptive_runtime_root.join(&rel), rel));
    }
    let abs = adaptive_root.join(&rel);
    let source_rel = relative_within(&adaptive_root, &abs).ok_or_else(|| {
        format!(
            "adaptive_store: target outside adaptive root: {}",
            abs.display()
        )
    })?;
    if source_rel.is_empty() {
        return Err(format!(
            "adaptive_store: target outside adaptive root: {}",
            abs.display()
        ));
    }
    Ok((abs, normalize_adaptive_rel(&source_rel)))
}

fn write_json_atomic(file_path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("adaptive_layer_store_kernel_create_dir_failed:{err}"))?;
    }
    let tmp_path = file_path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    fs::write(
        &tmp_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value)
                .map_err(|err| format!("adaptive_layer_store_kernel_encode_failed:{err}"))?
        ),
    )
    .map_err(|err| format!("adaptive_layer_store_kernel_write_failed:{err}"))?;
    fs::rename(&tmp_path, file_path)
        .map_err(|err| format!("adaptive_layer_store_kernel_rename_failed:{err}"))?;
    Ok(())
}

fn lock_path_for(abs_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.write.lock", abs_path.to_string_lossy()))
}

fn acquire_write_lock(abs_path: &Path) -> Result<WriteLock, String> {
    let lock_path = lock_path_for(abs_path);
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("adaptive_layer_store_kernel_lock_dir_failed:{err}"))?;
    }
    let started = std::time::Instant::now();
    loop {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                let body = json!({
                    "pid": std::process::id(),
                    "ts": now_iso(),
                    "path": abs_path.to_string_lossy(),
                });
                let _ = file.write_all(
                    format!(
                        "{}\n",
                        serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string())
                    )
                    .as_bytes(),
                );
                return Ok(WriteLock {
                    file,
                    lock_path,
                    waited_ms: started.elapsed().as_millis() as u64,
                });
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                let stale = fs::metadata(&lock_path)
                    .and_then(|meta| meta.modified())
                    .ok()
                    .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                    .map(|elapsed| elapsed.as_millis() as u64 > WRITE_LOCK_STALE_MS)
                    .unwrap_or(false);
                if stale {
                    let _ = fs::remove_file(&lock_path);
                    continue;
                }
                if started.elapsed().as_millis() as u64 >= WRITE_LOCK_TIMEOUT_MS {
                    return Err(format!(
                        "adaptive_store: write lock timeout for {}",
                        abs_path.display()
                    ));
                }
                sleep(Duration::from_millis(WRITE_LOCK_RETRY_MS));
            }
            Err(err) => {
                return Err(format!("adaptive_layer_store_kernel_lock_failed:{err}"));
            }
        }
    }
}

fn release_write_lock(lock: WriteLock) {
    let _ = fs::remove_file(lock.lock_path);
}

fn read_json_value(file_path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(file_path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn read_json_with_hash(file_path: &Path) -> (bool, Value, Option<String>) {
    if !file_path.exists() {
        return (false, Value::Null, None);
    }
    match read_json_value(file_path) {
        Some(value) => {
            let hash = canonical_hash(&value);
            (true, value, Some(hash))
        }
        None => (false, Value::Null, None),
    }
}

pub(crate) fn canonical_hash(value: &Value) -> String {
    let raw = serde_json::to_string(value).unwrap_or_else(|_| "null".to_string());
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn hash16(raw: &str) -> String {
    canonical_hash(&Value::String(raw.to_string()))[..16].to_string()
}

fn is_alnum(raw: &str) -> bool {
    !raw.is_empty() && raw.chars().all(|ch| ch.is_ascii_alphanumeric())
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

