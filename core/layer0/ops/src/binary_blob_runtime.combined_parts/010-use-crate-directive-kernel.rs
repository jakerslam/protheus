// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::directive_kernel;
use crate::v8_kernel::{
    canonical_json_string, keyed_digest_hex, parse_bool, parse_f64, print_json, read_json,
    scoped_state_root, sha256_file, sha256_hex_str, write_json, write_receipt,
};
use crate::{clean, now_iso, parse_args};
use memmap2::MmapOptions;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "BINARY_BLOB_RUNTIME_STATE_ROOT";
const STATE_SCOPE: &str = "binary_blob_runtime";
const BLOB_SIGNING_ENV: &str = "BINARY_BLOB_VAULT_SIGNING_KEY";
#[path = "../binary_blob_runtime_run.rs"]
mod binary_blob_runtime_run;

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn state_file(root: &Path, rel: &str) -> PathBuf {
    state_root(root).join(rel)
}

fn active_path(root: &Path) -> PathBuf {
    state_file(root, "active_blobs.json")
}

fn blobs_dir(root: &Path) -> PathBuf {
    state_file(root, "blobs")
}

fn snapshots_dir(root: &Path) -> PathBuf {
    state_file(root, "snapshots")
}

fn mutation_history_path(root: &Path) -> PathBuf {
    state_file(root, "mutation_history.jsonl")
}

fn prime_blob_vault_path(root: &Path) -> PathBuf {
    state_file(root, "prime_blob_vault.json")
}

fn default_prime_blob_vault() -> Value {
    json!({
        "version": "1.0",
        "entries": [],
        "chain_head": "genesis",
        "created_at": now_iso()
    })
}

fn load_prime_blob_vault(root: &Path) -> Value {
    let path = prime_blob_vault_path(root);
    let raw = read_json(&path).unwrap_or_else(default_prime_blob_vault);
    let normalized = normalize_prime_blob_vault(&raw);
    if normalized != raw {
        let _ = write_json(&path, &normalized);
    }
    normalized
}

fn store_prime_blob_vault(root: &Path, vault: &Value) -> Result<(), String> {
    write_json(&prime_blob_vault_path(root), vault)
}

fn blob_vault_secret() -> Option<String> {
    blob_vault_signing_keys().into_iter().next()
}

fn blob_vault_signing_keys() -> Vec<String> {
    let mut keys = Vec::new();
    for key in [BLOB_SIGNING_ENV, "DIRECTIVE_KERNEL_SIGNING_KEY"] {
        if let Ok(value) = std::env::var(key) {
            let cleaned = clean(value, 1024);
            if !cleaned.is_empty() && !keys.iter().any(|row| row == &cleaned) {
                keys.push(cleaned);
            }
        }
    }
    keys
}

fn blob_signature_payload(entry: &Value) -> Value {
    json!({
        "entry_id": entry.get("entry_id").cloned().unwrap_or(Value::Null),
        "module": entry.get("module").cloned().unwrap_or(Value::Null),
        "blob_id": entry.get("blob_id").cloned().unwrap_or(Value::Null),
        "source_hash": entry.get("source_hash").cloned().unwrap_or(Value::Null),
        "blob_hash": entry.get("blob_hash").cloned().unwrap_or(Value::Null),
        "policy_hash": entry.get("policy_hash").cloned().unwrap_or(Value::Null),
        "mode": entry.get("mode").cloned().unwrap_or(Value::Null),
        "shadow_pointer": entry.get("shadow_pointer").cloned().unwrap_or(Value::Null),
        "rollback_pointer": entry.get("rollback_pointer").cloned().unwrap_or(Value::Null),
        "prev_hash": entry.get("prev_hash").cloned().unwrap_or(Value::Null),
        "ts": entry.get("ts").cloned().unwrap_or(Value::Null)
    })
}

fn legacy_blob_signature_payload(entry: &Value) -> Value {
    json!({
        "entry_id": entry.get("entry_id").cloned().unwrap_or(Value::Null),
        "module": entry.get("module").cloned().unwrap_or(Value::Null),
        "blob_id": entry.get("blob_id").cloned().unwrap_or(Value::Null),
        "source_hash": entry.get("source_hash").cloned().unwrap_or(Value::Null),
        "blob_hash": entry.get("blob_hash").cloned().unwrap_or(Value::Null),
        "policy_hash": entry.get("policy_hash").cloned().unwrap_or(Value::Null),
        "mode": entry.get("mode").cloned().unwrap_or(Value::Null),
        "ts": entry.get("ts").cloned().unwrap_or(Value::Null)
    })
}

fn blob_signature_payload_variants(entry: &Value) -> Vec<Value> {
    let current = blob_signature_payload(entry);
    let legacy = legacy_blob_signature_payload(entry);
    let mut canonical = canonical_blob_entry_for_hash(entry);
    if let Some(obj) = canonical.as_object_mut() {
        obj.remove("signature");
    }
    let mut variants = vec![current.clone()];
    if legacy != current {
        variants.push(legacy.clone());
    }
    if canonical != current && canonical != legacy {
        variants.push(canonical);
    }
    variants
}

fn sign_blob_entry(entry: &Value) -> String {
    let payload = blob_signature_payload(entry);
    let payload_canonical = canonical_json_string(&payload);
    let key = blob_vault_secret().unwrap_or_default();
    if key.is_empty() {
        format!("unsigned:{}", sha256_hex_str(&payload_canonical))
    } else {
        format!("sig:{}", keyed_digest_hex(&key, &payload))
    }
}

fn verify_blob_entry_signature(entry: &Value) -> bool {
    let sig = entry
        .get("signature")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if sig.is_empty() {
        return false;
    }
    let payload_variants = blob_signature_payload_variants(entry);
    if let Some(raw) = sig.strip_prefix("unsigned:") {
        return payload_variants.iter().any(|payload| {
            raw.eq_ignore_ascii_case(&sha256_hex_str(&canonical_json_string(payload)))
        });
    }
    if let Some(raw) = sig.strip_prefix("sig:") {
        let keys = blob_vault_signing_keys();
        if keys.is_empty() {
            return false;
        }
        return keys.iter().any(|key| {
            payload_variants
                .iter()
                .any(|payload| raw.eq_ignore_ascii_case(&keyed_digest_hex(key, payload)))
        });
    }
    false
}

fn canonical_blob_entry_for_hash(entry: &Value) -> Value {
    let mut canonical = entry.clone();
    if let Some(obj) = canonical.as_object_mut() {
        obj.remove("entry_hash");
    }
    canonical
}

fn recompute_blob_entry_hash(entry: &Value) -> String {
    sha256_hex_str(&canonical_json_string(&canonical_blob_entry_for_hash(
        entry,
    )))
}

fn normalize_prime_blob_vault(vault: &Value) -> Value {
    let mut normalized = if vault.is_object() {
        vault.clone()
    } else {
        default_prime_blob_vault()
    };
    if !normalized
        .get("entries")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        normalized["entries"] = Value::Array(Vec::new());
    }
    if normalized.get("version").and_then(Value::as_str).is_none() {
        normalized["version"] = Value::String("1.0".to_string());
    }
    if normalized
        .get("created_at")
        .and_then(Value::as_str)
        .is_none()
    {
        normalized["created_at"] = Value::String(now_iso());
    }
    let chain_head_missing = normalized
        .get("chain_head")
        .and_then(Value::as_str)
        .map(|v| v.trim().is_empty())
        .unwrap_or(true);
    if chain_head_missing {
        let derived = normalized
            .get("entries")
            .and_then(Value::as_array)
            .and_then(|rows| rows.last())
            .and_then(|row| row.get("entry_hash"))
            .and_then(Value::as_str)
            .filter(|v| !v.trim().is_empty())
            .unwrap_or("genesis")
            .to_string();
        normalized["chain_head"] = Value::String(derived);
    }
    normalized
}
