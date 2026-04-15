// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

fn usage() {
    println!("mutation-provenance-kernel commands:");
    println!("  protheus-ops mutation-provenance-kernel load-policy [--payload-base64=<json>]");
    println!("  protheus-ops mutation-provenance-kernel normalize-meta --payload-base64=<json>");
    println!("  protheus-ops mutation-provenance-kernel enforce --payload-base64=<json>");
    println!("  protheus-ops mutation-provenance-kernel record-audit --payload-base64=<json>");
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
            .map_err(|err| format!("mutation_provenance_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("mutation_provenance_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("mutation_provenance_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("mutation_provenance_kernel_payload_decode_failed:{err}"));
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

fn workspace_root(root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("PROTHEUS_WORKSPACE_ROOT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    root.to_path_buf()
}

fn client_root(root: &Path) -> PathBuf {
    workspace_root(root).join("client")
}

fn runtime_root(root: &Path) -> PathBuf {
    workspace_root(root).join("client").join("runtime")
}

fn normalize_path_string(raw: &str) -> String {
    raw.replace('\\', "/")
}

fn strip_private_prefix(raw: &str) -> &str {
    raw.strip_prefix("/private").unwrap_or(raw)
}

fn strip_prefix_loose<'a>(candidate: &'a str, prefix: &str) -> Option<&'a str> {
    let candidate_norm = strip_private_prefix(candidate);
    let prefix_norm = strip_private_prefix(prefix).trim_end_matches('/');
    if prefix_norm.is_empty() {
        return None;
    }
    if candidate_norm == prefix_norm {
        return Some("");
    }
    let with_sep = format!("{prefix_norm}/");
    candidate_norm.strip_prefix(&with_sep)
}

fn default_policy_path(root: &Path) -> PathBuf {
    let runtime_candidate = runtime_root(root)
        .join("config")
        .join("mutation_provenance_policy.json");
    if runtime_candidate.exists() {
        runtime_candidate
    } else {
        client_root(root)
            .join("config")
            .join("mutation_provenance_policy.json")
    }
}

fn resolve_policy_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let explicit = clean_text(payload.get("policy_path"), 520);
    if !explicit.is_empty() {
        return PathBuf::from(explicit);
    }
    if let Ok(raw) = std::env::var("MUTATION_PROVENANCE_POLICY_PATH") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    default_policy_path(root)
}

fn read_json_safe(file_path: &Path, fallback: Value) -> Value {
    fs::read_to_string(file_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(fallback)
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("mutation_provenance_kernel_create_dir_failed:{err}"))?;
    }
    Ok(())
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("mutation_provenance_kernel_append_open_failed:{err}"))?;
    file.write_all(
        format!(
            "{}\n",
            serde_json::to_string(row).unwrap_or_else(|_| "null".to_string())
        )
        .as_bytes(),
    )
    .map_err(|err| format!("mutation_provenance_kernel_append_failed:{err}"))
}

fn normalize_source(root: &Path, raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        let candidate_norm = normalize_path_string(&candidate.to_string_lossy());
        let explicit_workspace = root.to_path_buf();
        let explicit_client = explicit_workspace.join("client");
        let explicit_runtime = explicit_client.join("runtime");
        for base in [
            explicit_runtime,
            explicit_client,
            explicit_workspace,
            runtime_root(root),
            client_root(root),
            workspace_root(root),
        ] {
            if let Ok(rel) = candidate.strip_prefix(&base) {
                return normalize_path_string(&rel.to_string_lossy());
            }
            let base_norm = normalize_path_string(&base.to_string_lossy());
            if let Some(rel) = strip_prefix_loose(&candidate_norm, &base_norm) {
                return normalize_path_string(rel);
            }
        }
    }
    normalize_path_string(trimmed)
}

pub(crate) fn normalize_meta_value(
    root: &Path,
    meta: Option<&Map<String, Value>>,
    fallback_source: &str,
    default_reason: &str,
) -> Value {
    let src = meta.cloned().unwrap_or_default();
    let source_input = {
        let value = as_str(src.get("source"));
        if !value.is_empty() {
            value
        } else {
            fallback_source.trim().to_string()
        }
    };
    let actor = {
        let value = clean_text(src.get("actor"), 80);
        if value.is_empty() {
            std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())
        } else {
            value
        }
    };
    let reason = {
        let value = clean_text(src.get("reason"), 160);
        if value.is_empty() {
            default_reason.trim().to_string()
        } else {
            value
        }
    };
    let source = if source_input.is_empty() {
        String::new()
    } else {
        normalize_source(root, &source_input)
    };

    let mut out = Map::new();
    for (key, value) in src {
        if matches!(key.as_str(), "source" | "actor" | "reason") {
            continue;
        }
        out.insert(key, value);
    }
    out.insert("source".to_string(), Value::String(source));
    out.insert("actor".to_string(), Value::String(actor));
    out.insert("reason".to_string(), Value::String(reason));
    Value::Object(out)
}

