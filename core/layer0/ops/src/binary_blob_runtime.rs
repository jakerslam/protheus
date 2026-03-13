// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::directive_kernel;
use crate::v8_kernel::{
    parse_bool, parse_f64, print_json, read_json, scoped_state_root, sha256_file, sha256_hex_str,
    write_json, write_receipt,
};
use crate::{clean, now_iso, parse_args};
use memmap2::MmapOptions;
use serde_json::{json, Map, Value};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "BINARY_BLOB_RUNTIME_STATE_ROOT";
const STATE_SCOPE: &str = "binary_blob_runtime";
#[path = "binary_blob_runtime_run.rs"]
mod binary_blob_runtime_run;

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn active_path(root: &Path) -> PathBuf {
    state_root(root).join("active_blobs.json")
}

fn blobs_dir(root: &Path) -> PathBuf {
    state_root(root).join("blobs")
}

fn snapshots_dir(root: &Path) -> PathBuf {
    state_root(root).join("snapshots")
}

fn mutation_history_path(root: &Path) -> PathBuf {
    state_root(root).join("mutation_history.jsonl")
}

fn normalize_module(raw: Option<&String>) -> String {
    clean(raw.cloned().unwrap_or_else(|| "all".to_string()), 96)
        .to_ascii_lowercase()
        .replace(' ', "_")
}

fn module_source_path(root: &Path, module: &str, explicit: Option<&String>) -> PathBuf {
    if let Some(p) = explicit {
        let c = PathBuf::from(clean(p, 512));
        if c.is_absolute() {
            return c;
        }
        return root.join(c);
    }
    root.join("core")
        .join("layer0")
        .join("ops")
        .join("src")
        .join(format!("{module}.rs"))
}

fn sha256_file_mmap(path: &Path) -> Result<String, String> {
    let file =
        fs::File::open(path).map_err(|err| format!("blob_open_failed:{}:{err}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|err| format!("blob_metadata_failed:{}:{err}", path.display()))?;
    if metadata.len() == 0 {
        return Ok(sha256_hex_str(""));
    }
    if metadata.len() > usize::MAX as u64 {
        return Err("blob_too_large_for_mmap".to_string());
    }
    let map = unsafe { MmapOptions::new().map(&file) }
        .map_err(|err| format!("blob_mmap_failed:{}:{err}", path.display()))?;
    Ok(crate::v8_kernel::sha256_hex_bytes(&map))
}

fn read_first_bytes(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let mut file =
        fs::File::open(path).map_err(|err| format!("blob_open_failed:{}:{err}", path.display()))?;
    let mut buf = vec![0u8; limit];
    let read = file
        .read(&mut buf)
        .map_err(|err| format!("blob_read_failed:{}:{err}", path.display()))?;
    buf.truncate(read);
    Ok(buf)
}

fn load_active_map(root: &Path) -> Map<String, Value> {
    read_json(&active_path(root))
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn write_active_map(root: &Path, map: &Map<String, Value>) -> Result<(), String> {
    write_json(&active_path(root), &Value::Object(map.clone()))
}

fn write_mutation_event(root: &Path, event: &Value) {
    if let Some(parent) = mutation_history_path(root).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let line = serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string());
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(mutation_history_path(root))
        .and_then(|mut file| std::io::Write::write_all(&mut file, format!("{line}\n").as_bytes()));
}

fn parse_module_list(flags: &std::collections::HashMap<String, String>) -> Vec<String> {
    let csv = flags
        .get("modules")
        .cloned()
        .unwrap_or_else(|| "conduit,directive_kernel,network_protocol,intelligence_nexus,organism_layer,rsi_ignition".to_string());
    csv.split(',')
        .map(|v| clean(v, 96).to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
}

fn settle_one(root: &Path, parsed: &crate::ParsedArgs, module: &str) -> Result<Value, String> {
    let mode = clean(
        parsed
            .flags
            .get("mode")
            .cloned()
            .unwrap_or_else(|| "modular".to_string()),
        24,
    );
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let shadow_swap = parse_bool(parsed.flags.get("shadow-swap"), true);
    let source_path = module_source_path(root, module, parsed.flags.get("module-path"));

    if !source_path.exists() {
        return Err(format!("module_source_missing:{}", source_path.display()));
    }

    let source_hash = sha256_file(&source_path)?;
    let policy_hash = directive_kernel::directive_vault_hash(root);
    let blob_id = sha256_hex_str(&format!("{}:{}:{}", module, source_hash, policy_hash));

    let blob_path = blobs_dir(root).join(module).join(format!("{blob_id}.blob"));
    let snapshot_path = snapshots_dir(root)
        .join(module)
        .join(format!("{blob_id}.json"));
    let source_bytes = fs::read(&source_path)
        .map_err(|err| format!("module_source_read_failed:{}:{err}", source_path.display()))?;
    let blob_hash = crate::v8_kernel::sha256_hex_bytes(&source_bytes);
    if apply {
        if let Some(parent) = blob_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("blob_dir_create_failed:{}:{err}", parent.display()))?;
        }
        if let Some(parent) = snapshot_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("snapshot_dir_create_failed:{}:{err}", parent.display()))?;
        }
        fs::write(&blob_path, source_bytes)
            .map_err(|err| format!("blob_write_failed:{}:{err}", blob_path.display()))?;
    }

    let mut active = load_active_map(root);
    let previous = active.get(module).cloned().unwrap_or(Value::Null);
    let shadow_pointer = format!("shadow://{}:{}", module, &blob_id[..16]);
    let rollback_pointer = format!(
        "rollback://{}:{}",
        module,
        &sha256_hex_str(&now_iso())[..16]
    );

    let snapshot = json!({
        "module": module,
        "blob_id": blob_id,
        "source_path": source_path.display().to_string(),
        "source_hash": source_hash,
        "blob_path": blob_path.display().to_string(),
        "blob_hash": blob_hash,
        "policy_hash": policy_hash,
        "mode": mode,
        "shadow_swap": shadow_swap,
        "shadow_pointer": shadow_pointer,
        "rollback_pointer": rollback_pointer,
        "previous": previous,
        "ts": now_iso()
    });

    if apply {
        write_json(&snapshot_path, &snapshot)?;
        active.insert(
            module.to_string(),
            json!({
                "blob_id": snapshot.get("blob_id").cloned().unwrap_or(Value::Null),
                "snapshot_path": snapshot_path.display().to_string(),
                "blob_path": blob_path.display().to_string(),
                "policy_hash": snapshot.get("policy_hash").cloned().unwrap_or(Value::Null),
                "source_hash": snapshot.get("source_hash").cloned().unwrap_or(Value::Null),
                "blob_hash": snapshot.get("blob_hash").cloned().unwrap_or(Value::Null),
                "previous": snapshot.get("previous").cloned().unwrap_or(Value::Null),
                "shadow_pointer": shadow_pointer,
                "rollback_pointer": rollback_pointer,
                "active_at": now_iso()
            }),
        );
        write_active_map(root, &active)?;
    }

    Ok(json!({
        "module": module,
        "snapshot": snapshot,
        "snapshot_path": snapshot_path.display().to_string(),
        "blob_path": blob_path.display().to_string(),
        "applied": apply
    }))
}

fn load_and_verify(root: &Path, module: &str) -> Result<Value, String> {
    let active = load_active_map(root);
    let Some(entry) = active.get(module).cloned() else {
        return Err("module_not_settled".to_string());
    };

    let snapshot_path = entry
        .get("snapshot_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| "snapshot_path_missing".to_string())?;
    if !snapshot_path.exists() {
        return Err(format!("snapshot_missing:{}", snapshot_path.display()));
    }

    let snapshot = read_json(&snapshot_path)
        .ok_or_else(|| format!("snapshot_read_failed:{}", snapshot_path.display()))?;
    let source_path = snapshot
        .get("source_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| "snapshot_source_path_missing".to_string())?;
    let expected_source_hash = snapshot
        .get("source_hash")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let source_hash = sha256_file(&source_path)?;
    let expected_policy_hash = snapshot
        .get("policy_hash")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let blob_path = snapshot
        .get("blob_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| {
            entry
                .get("blob_path")
                .and_then(Value::as_str)
                .map(PathBuf::from)
        })
        .ok_or_else(|| "snapshot_blob_path_missing".to_string())?;
    let expected_blob_hash = snapshot
        .get("blob_hash")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let current_policy_hash = directive_kernel::directive_vault_hash(root);

    if source_hash != expected_source_hash {
        return Err("source_hash_mismatch".to_string());
    }
    if !blob_path.exists() {
        return Err(format!("blob_missing:{}", blob_path.display()));
    }
    let blob_hash = sha256_file_mmap(&blob_path)?;
    if blob_hash != expected_blob_hash {
        return Err("blob_hash_mismatch".to_string());
    }
    if current_policy_hash != expected_policy_hash {
        return Err("policy_hash_mismatch".to_string());
    }

    Ok(json!({
        "module": module,
        "snapshot_path": snapshot_path.display().to_string(),
        "source_path": source_path.display().to_string(),
        "blob_path": blob_path.display().to_string(),
        "source_hash": source_hash,
        "blob_hash": blob_hash,
        "policy_hash": current_policy_hash,
        "blob_first_bytes_hex": hex::encode(read_first_bytes(&blob_path, 16)?),
        "verified": true
    }))
}

fn emit(root: &Path, payload: Value) -> i32 {
    match write_receipt(root, STATE_ENV, STATE_SCOPE, payload) {
        Ok(out) => {
            print_json(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                2
            }
        }
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "binary_blob_runtime_error",
                "lane": "core/layer0/ops",
                "error": clean(err, 240),
                "exit_code": 2
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            print_json(&out);
            2
        }
    }
}

fn verify_debug_token(root: &Path) -> Value {
    let (payload, code) = infring_layer1_security::run_soul_token_guard(
        root,
        &["verify".to_string(), "--strict=1".to_string()],
    );
    json!({"ok": code == 0 && payload.get("ok").and_then(Value::as_bool).unwrap_or(false), "payload": payload, "code": code})
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    binary_blob_runtime_run::run(root, argv)
}

#[cfg(test)]
#[path = "binary_blob_runtime_tests.rs"]
mod tests;
