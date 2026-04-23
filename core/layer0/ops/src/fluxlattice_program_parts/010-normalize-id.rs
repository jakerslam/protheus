// SPDX-License-Identifier: Apache-2.0
use crate::{clean, now_iso, parse_args};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const IDS: [&str; 16] = [
    "V4-ETH-001",
    "V4-ETH-002",
    "V4-ETH-003",
    "V4-ETH-004",
    "V4-ETH-005",
    "V4-SEC-014",
    "V4-SEC-015",
    "V4-SEC-016",
    "V4-PKG-001",
    "V4-PKG-002",
    "V4-PKG-003",
    "V4-LENS-006",
    "V4-PKG-004",
    "V4-PKG-005",
    "V4-PKG-006",
    "V4-PKG-007",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramItem {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paths {
    pub state_path: PathBuf,
    pub latest_path: PathBuf,
    pub receipts_path: PathBuf,
    pub history_path: PathBuf,
    pub security_panel_path: PathBuf,
    pub flux_events_path: PathBuf,
    pub migration_profiles_path: PathBuf,
    pub lens_mode_policy_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub version: String,
    pub enabled: bool,
    pub strict_default: bool,
    pub items: Vec<ProgramItem>,
    pub paths: Paths,
    pub policy_path: PathBuf,
}

fn normalize_id(v: &str) -> String {
    let id = clean(v.replace('`', ""), 80).to_ascii_uppercase();
    if IDS.iter().any(|x| *x == id) {
        id
    } else {
        String::new()
    }
}

fn to_bool(v: Option<&str>, fallback: bool) -> bool {
    let Some(raw) = v else {
        return fallback;
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn read_json(path: &Path) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create_dir_failed:{}:{e}", parent.display()))?;
    }
    Ok(())
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    let mut payload =
        serde_json::to_string_pretty(value).map_err(|e| format!("encode_json_failed:{e}"))?;
    payload.push('\n');
    fs::write(&tmp, payload).map_err(|e| format!("write_tmp_failed:{}:{e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename_tmp_failed:{}:{e}", path.display()))
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut payload = serde_json::to_string(row).map_err(|e| format!("encode_row_failed:{e}"))?;
    payload.push('\n');
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, payload.as_bytes()))
        .map_err(|e| format!("append_jsonl_failed:{}:{e}", path.display()))
}

fn resolve_path(root: &Path, raw: Option<&Value>, fallback_rel: &str) -> PathBuf {
    let fallback = root.join(fallback_rel);
    let Some(raw) = raw.and_then(Value::as_str) else {
        return fallback;
    };
    let text = clean(raw, 400).replace('\\', "/");
    let text = text.trim().to_string();
    if text.is_empty() {
        return fallback;
    }
    let pb = PathBuf::from(&text);
    if pb.is_absolute() {
        if pb.starts_with(root) {
            pb
        } else {
            fallback
        }
    } else {
        let safe_rel = pb.components().all(|component| {
            matches!(component, Component::Normal(_) | Component::CurDir)
        });
        if safe_rel {
            root.join(pb)
        } else {
            fallback
        }
    }
}

fn rel_path(root: &Path, abs: &Path) -> String {
    abs.strip_prefix(root)
        .unwrap_or(abs)
        .to_string_lossy()
        .replace('\\', "/")
}

fn stable_hash(input: &str, len: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let hex = hex::encode(hasher.finalize());
    hex[..len.min(hex.len())].to_string()
}

fn parse_json_output(raw: &str) -> Value {
    let text = raw.trim();
    if text.is_empty() {
        return Value::Null;
    }

    if let Ok(v) = serde_json::from_str::<Value>(text) {
        return v;
    }

    for line in text.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            return v;
        }
    }

    if let Some(idx) = text.find('{') {
        if let Ok(v) = serde_json::from_str::<Value>(&text[idx..]) {
            return v;
        }
    }

    Value::Null
}

fn run_node_json(root: &Path, script_rel: &str, args: &[String]) -> Value {
    let abs = root.join(script_rel);
    let out = Command::new("node")
        .arg(abs)
        .args(args)
        .current_dir(root)
        .output();

    let Ok(out) = out else {
        return json!({
            "ok": false,
            "status": 1,
            "stdout": "",
            "stderr": "spawn_failed",
            "payload": Value::Null
        });
    };

    let status = out.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = clean(String::from_utf8_lossy(&out.stderr), 1200);
    let payload = parse_json_output(&stdout);

    json!({
        "ok": status == 0,
        "status": status,
        "stdout": clean(stdout, 30_000),
        "stderr": stderr,
        "payload": payload
    })
}

fn run_cargo_flux(root: &Path, args: &[String]) -> Value {
    let out = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg("core/layer0/fluxlattice/Cargo.toml")
        .arg("--bin")
        .arg("fluxlattice")
        .arg("--")
        .args(args)
        .current_dir(root)
        .output();

    let Ok(out) = out else {
        return json!({
            "ok": false,
            "status": 1,
            "stdout": "",
            "stderr": "spawn_failed",
            "payload": Value::Null
        });
    };

    let status = out.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = clean(String::from_utf8_lossy(&out.stderr), 1200);
    let payload = parse_json_output(&stdout);

    json!({
        "ok": status == 0,
        "status": status,
        "stdout": clean(stdout, 30_000),
        "stderr": stderr,
        "payload": payload
    })
}

pub fn default_policy(root: &Path) -> Policy {
    Policy {
        version: "1.0".to_string(),
        enabled: true,
        strict_default: true,
        items: IDS
            .iter()
            .map(|id| ProgramItem {
                id: (*id).to_string(),
                title: (*id).to_string(),
            })
            .collect(),
        paths: Paths {
            state_path: root.join("local/state/ops/fluxlattice_program/state.json"),
            latest_path: root.join("local/state/ops/fluxlattice_program/latest.json"),
            receipts_path: root.join("local/state/ops/fluxlattice_program/receipts.jsonl"),
            history_path: root.join("local/state/ops/fluxlattice_program/history.jsonl"),
            security_panel_path: root.join("local/state/ops/infring_top/security_panel.json"),
            flux_events_path: root.join("local/state/ops/fluxlattice_program/flux_events.jsonl"),
            migration_profiles_path: root
                .join("client/runtime/config/fluxlattice_migration_profiles.json"),
            lens_mode_policy_path: root.join("client/runtime/config/lens_mode_policy.json"),
        },
        policy_path: root.join("client/runtime/config/fluxlattice_program_policy.json"),
    }
}

pub fn load_policy(root: &Path, policy_path: &Path) -> Policy {
    let base = default_policy(root);
    let raw = read_json(policy_path);

    let mut out = base.clone();
    if let Some(v) = raw.get("version").and_then(Value::as_str) {
        let c = clean(v, 24);
        if !c.is_empty() {
            out.version = c;
        }
    }
    out.enabled = raw
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(base.enabled);
    out.strict_default = raw
        .get("strict_default")
        .and_then(Value::as_bool)
        .unwrap_or(base.strict_default);

    out.items = raw
        .get("items")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let id = normalize_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
                    if id.is_empty() {
                        return None;
                    }
                    let title = clean(row.get("title").and_then(Value::as_str).unwrap_or(&id), 260);
                    Some(ProgramItem {
                        id: id.clone(),
                        title: if title.is_empty() { id } else { title },
                    })
                })
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| base.items.clone());

    let paths = raw.get("paths").cloned().unwrap_or(Value::Null);
    out.paths = Paths {
        state_path: resolve_path(
            root,
            paths.get("state_path"),
            "local/state/ops/fluxlattice_program/state.json",
        ),
        latest_path: resolve_path(
            root,
            paths.get("latest_path"),
            "local/state/ops/fluxlattice_program/latest.json",
        ),
        receipts_path: resolve_path(
            root,
            paths.get("receipts_path"),
            "local/state/ops/fluxlattice_program/receipts.jsonl",
        ),
        history_path: resolve_path(
            root,
            paths.get("history_path"),
            "local/state/ops/fluxlattice_program/history.jsonl",
        ),
        security_panel_path: resolve_path(
            root,
            paths.get("security_panel_path"),
            "local/state/ops/infring_top/security_panel.json",
        ),
        flux_events_path: resolve_path(
            root,
            paths.get("flux_events_path"),
            "local/state/ops/fluxlattice_program/flux_events.jsonl",
        ),
        migration_profiles_path: resolve_path(
            root,
            paths.get("migration_profiles_path"),
            "client/runtime/config/fluxlattice_migration_profiles.json",
        ),
        lens_mode_policy_path: resolve_path(
            root,
            paths.get("lens_mode_policy_path"),
            "client/runtime/config/lens_mode_policy.json",
        ),
    };

    out.policy_path = if policy_path.is_absolute() {
        policy_path.to_path_buf()
    } else {
        root.join(policy_path)
    };

    out
}

fn default_state() -> Value {
    json!({
        "schema_id": "fluxlattice_program_state",
        "schema_version": "1.0",
        "updated_at": now_iso(),
        "flux": {
          "morphology": "coalesced",
          "shadow_active": false,
          "dissolved_modules": [],
          "weave_mode": "deterministic"
        },
        "covenant": {
          "state": "unknown",
          "last_decision": Value::Null,
          "receipt_chain_hash": Value::Null
        },
        "tamper": {
          "anomalies": false,
          "last_revocation_at": Value::Null
        },
        "lens": {
          "mode": "hidden",
          "private_store": "client/runtime/local/private-lenses/"
        }
    })
}

fn load_state(policy: &Policy) -> Value {
    let raw = read_json(&policy.paths.state_path);
    if !raw.is_object() {
        return default_state();
    }

    let mut merged = default_state().as_object().cloned().unwrap_or_default();
    for (k, v) in raw.as_object().cloned().unwrap_or_default() {
        merged.insert(k, v);
    }

    if !merged.get("flux").map(Value::is_object).unwrap_or(false) {
        merged.insert("flux".to_string(), default_state()["flux"].clone());
    }
    if !merged
        .get("covenant")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        merged.insert("covenant".to_string(), default_state()["covenant"].clone());
    }
    if !merged.get("tamper").map(Value::is_object).unwrap_or(false) {
        merged.insert("tamper".to_string(), default_state()["tamper"].clone());
    }
    if !merged.get("lens").map(Value::is_object).unwrap_or(false) {
        merged.insert("lens".to_string(), default_state()["lens"].clone());
    }

    Value::Object(merged)
}

fn save_state(policy: &Policy, state: &Value, apply: bool) -> Result<(), String> {
    if !apply {
        return Ok(());
    }

    let mut payload = state.clone();
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("updated_at".to_string(), Value::String(now_iso()));
    }
    write_json_atomic(&policy.paths.state_path, &payload)
}

fn write_receipt(policy: &Policy, payload: &Value, apply: bool) -> Result<(), String> {
    if !apply {
        return Ok(());
    }

    write_json_atomic(&policy.paths.latest_path, payload)?;
    append_jsonl(&policy.paths.receipts_path, payload)?;
    append_jsonl(&policy.paths.history_path, payload)
}

fn append_flux_event(policy: &Policy, row: &Value, apply: bool) -> Result<(), String> {
    if !apply {
        return Ok(());
    }
    append_jsonl(&policy.paths.flux_events_path, row)
}
