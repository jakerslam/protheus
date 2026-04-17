// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::Utc;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_REL_PATH: &str = "sensory/eyes/focus_triggers.json";
const MUTATION_LOG_REL: &str = "client/runtime/local/state/security/adaptive_mutations.jsonl";
const POINTERS_REL: &str = "client/runtime/local/state/memory/adaptive_pointers.jsonl";
const POINTER_INDEX_REL: &str = "client/runtime/local/state/memory/adaptive_pointer_index.json";

fn usage() {
    println!("focus-trigger-store-kernel commands:");
    println!("  protheus-ops focus-trigger-store-kernel paths [--payload-base64=<json>]");
    println!("  protheus-ops focus-trigger-store-kernel default-state");
    println!("  protheus-ops focus-trigger-store-kernel normalize-state [--payload-base64=<json>]");
    println!("  protheus-ops focus-trigger-store-kernel read-state [--payload-base64=<json>]");
    println!("  protheus-ops focus-trigger-store-kernel ensure-state [--payload-base64=<json>]");
    println!("  protheus-ops focus-trigger-store-kernel set-state --payload-base64=<json>");
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
    lane_utils::payload_json(argv, "focus_trigger_store_kernel")
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    lane_utils::payload_obj(value)
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

fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::String(v)) => matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        _ => fallback,
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

fn has_disallowed_path_tokens(raw: &str) -> bool {
    raw.chars()
        .any(|ch| ch.is_ascii_control() || matches!(ch, '\n' | '\r' | '\t' | '\0'))
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
    let body_len = length.saturating_sub(out.len()).max(8);
    out.push_str(&hex[..body_len.min(hex.len())]);
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
    let workspace = workspace_root(root);
    let candidate = workspace.join("client").join("runtime");
    if candidate.exists() {
        candidate
    } else {
        workspace
    }
}

fn default_abs_path(root: &Path) -> PathBuf {
    runtime_root(root).join("adaptive").join(DEFAULT_REL_PATH)
}

fn mutation_log_path(root: &Path) -> PathBuf {
    workspace_root(root).join(MUTATION_LOG_REL)
}

fn adaptive_pointers_path(root: &Path) -> PathBuf {
    workspace_root(root).join(POINTERS_REL)
}

fn adaptive_pointer_index_path(root: &Path) -> PathBuf {
    workspace_root(root).join(POINTER_INDEX_REL)
}

fn store_abs_path(root: &Path, payload: &Map<String, Value>) -> Result<PathBuf, String> {
    let canonical = default_abs_path(root);
    let raw = clean_text(
        payload
            .get("file_path")
            .or_else(|| payload.get("path"))
            .or_else(|| payload.get("store_path")),
        520,
    );
    if raw.is_empty() {
        return Ok(canonical);
    }
    if has_disallowed_path_tokens(&raw) {
        return Err("focus_trigger_store: path contains disallowed control characters".to_string());
    }
    let requested = PathBuf::from(raw);
    if requested
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("focus_trigger_store: path override with parent traversal denied".to_string());
    }
    let resolved = if requested.is_absolute() {
        requested
    } else {
        workspace_root(root).join(requested)
    };
    let resolved_canon = fs::canonicalize(&resolved).unwrap_or(resolved.clone());
    let canonical_canon = fs::canonicalize(&canonical).unwrap_or(canonical.clone());
    if resolved_canon != canonical_canon {
        return Err(format!(
            "focus_trigger_store: path override denied (requested={})",
            resolved_canon.display()
        ));
    }
    Ok(canonical)
}

fn write_json_atomic(file_path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("focus_trigger_store_kernel_create_dir_failed:{err}"))?;
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
                .map_err(|err| format!("focus_trigger_store_kernel_encode_failed:{err}"))?
        ),
    )
    .map_err(|err| format!("focus_trigger_store_kernel_write_failed:{err}"))?;
    fs::rename(&tmp_path, file_path)
        .map_err(|err| format!("focus_trigger_store_kernel_rename_failed:{err}"))?;
    Ok(())
}

fn read_json_value(file_path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(file_path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn append_jsonl(file_path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("focus_trigger_store_kernel_create_dir_failed:{err}"))?;
    }
    let mut handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .map_err(|err| format!("focus_trigger_store_kernel_append_open_failed:{err}"))?;
    handle
        .write_all(
            format!(
                "{}\n",
                serde_json::to_string(row).unwrap_or_else(|_| "null".to_string())
            )
            .as_bytes(),
        )
        .map_err(|err| format!("focus_trigger_store_kernel_append_failed:{err}"))?;
    Ok(())
}

fn normalize_terms_array(raw: Option<&Value>, max_count: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    if let Some(Value::Array(rows)) = raw {
        for row in rows {
            let term = normalize_key(&as_str(Some(row)), 120);
            if term.is_empty() || seen.contains(&term) {
                continue;
            }
            seen.insert(term.clone());
            out.push(term);
            if out.len() >= max_count {
                break;
            }
        }
    }
    out
}

fn normalize_term_weights(raw: Option<&Value>, include_terms: &[String], max_weight: i64) -> Value {
    let allowed: BTreeSet<String> = include_terms.iter().cloned().collect();
    let mut out = Map::new();
    if let Some(Value::Object(rows)) = raw {
        for (key, value) in rows {
            let term = normalize_key(key, 120);
            if term.is_empty() || !allowed.contains(&term) {
                continue;
            }
            out.insert(
                term,
                Value::Number(serde_json::Number::from(clamp_i64(
                    Some(value),
                    1,
                    max_weight,
                    1,
                ))),
            );
        }
    }
    for term in allowed {
        out.entry(term)
            .or_insert_with(|| Value::Number(serde_json::Number::from(1)));
    }
    Value::Object(out)
}
