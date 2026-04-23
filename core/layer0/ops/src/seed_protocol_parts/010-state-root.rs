// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::directive_kernel;
use crate::v8_kernel::{
    deterministic_merkle_root, parse_bool, parse_f64, parse_u64, print_json, read_json,
    scoped_state_root, sha256_hex_str, write_json, write_receipt,
};
use crate::{clean, now_iso, parse_args};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "SEED_PROTOCOL_STATE_ROOT";
const STATE_SCOPE: &str = "seed_protocol";
const PACKETS_DIR: &str = "packets";
const PROVIDER_FAMILY_CONTRACT_TARGETS: &[&str] =
    &["anthropic", "fal", "google", "minimax", "moonshot"];

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn state_path(root: &Path) -> PathBuf {
    state_root(root).join("seed_state.json")
}

fn packets_dir(root: &Path) -> PathBuf {
    state_root(root).join(PACKETS_DIR)
}

fn default_state() -> Value {
    json!({
        "version": "1.0",
        "active_profile": Value::Null,
        "packet_count": 0u64,
        "replication_count": 0u64,
        "migration_count": 0u64,
        "compliance_checks": 0u64,
        "compliance_denies": 0u64,
        "selection_rounds": 0u64,
        "archive_count": 0u64,
        "archive_merkle_root": sha256_hex_str("seed_archive_empty"),
        "defense_event_count": 0u64,
        "provider_family_contract": {
            "active_provider_family": "moonshot",
            "provider_family_contract_targets": PROVIDER_FAMILY_CONTRACT_TARGETS,
            "provider_runtime_contract": true,
            "provider_auth_contract": true,
            "provider_registry_contract": true,
            "provider_discovery_contract": true
        },
        "quarantine": {},
        "packets": [],
        "replications": [],
        "migrations": [],
        "selection_history": [],
        "archives": [],
        "defense_events": [],
        "created_at": now_iso()
    })
}

fn load_state(root: &Path) -> Value {
    read_json(&state_path(root)).unwrap_or_else(default_state)
}

fn store_state(root: &Path, state: &Value) -> Result<(), String> {
    write_json(&state_path(root), state)
}

fn state_obj_mut(state: &mut Value) -> &mut Map<String, Value> {
    if !state.is_object() {
        *state = default_state();
    }
    state.as_object_mut().expect("state_object")
}

fn obj_mut<'a>(obj: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    if !obj.get(key).map(Value::is_object).unwrap_or(false) {
        obj.insert(key.to_string(), Value::Object(Map::new()));
    }
    obj.get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object")
}

fn arr_mut<'a>(obj: &'a mut Map<String, Value>, key: &str) -> &'a mut Vec<Value> {
    if !obj.get(key).map(Value::is_array).unwrap_or(false) {
        obj.insert(key.to_string(), Value::Array(Vec::new()));
    }
    obj.get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("array")
}

fn push_bounded(rows: &mut Vec<Value>, value: Value, max_rows: usize) {
    rows.push(value);
    if rows.len() > max_rows {
        let drop_n = rows.len() - max_rows;
        rows.drain(0..drop_n);
    }
}

fn set_counter(obj: &mut Map<String, Value>, key: &str, value: u64) {
    obj.insert(key.to_string(), Value::from(value));
}

fn inc_counter(obj: &mut Map<String, Value>, key: &str, delta: u64) -> u64 {
    let next = obj.get(key).and_then(Value::as_u64).unwrap_or(0) + delta;
    set_counter(obj, key, next);
    next
}

fn core_state_root(root: &Path) -> PathBuf {
    crate::core_state_root(root).join("ops")
}

fn read_blob_index(root: &Path) -> Value {
    read_json(
        &core_state_root(root)
            .join("binary_blob_runtime")
            .join("active_blobs.json"),
    )
    .unwrap_or_else(|| Value::Object(Map::new()))
}

fn read_organism_state(root: &Path) -> Value {
    read_json(
        &core_state_root(root)
            .join("organism_layer")
            .join("organism_state.json"),
    )
    .unwrap_or_else(|| Value::Object(Map::new()))
}

fn read_network_ledger(root: &Path) -> Value {
    read_json(
        &core_state_root(root)
            .join("network_protocol")
            .join("ledger.json"),
    )
    .unwrap_or_else(|| Value::Object(Map::new()))
}

fn gate_allowed(root: &Path, action: &str) -> bool {
    directive_kernel::action_allowed(root, action)
        || directive_kernel::action_allowed(root, "seed:*")
        || directive_kernel::action_allowed(root, "seed")
}

fn normalize_provider_family(raw: Option<&str>) -> String {
    match raw.unwrap_or("").trim().to_ascii_lowercase().as_str() {
        "claude" | "anthropic" => "anthropic".to_string(),
        "fal_ai" | "fal" => "fal".to_string(),
        "gemini" | "google" => "google".to_string(),
        "minimax" => "minimax".to_string(),
        "kimi" | "moonshot" => "moonshot".to_string(),
        other => other.to_string(),
    }
}

fn profile_claim_id(prefix: &str, profile: &str) -> String {
    let norm = profile.trim().to_ascii_lowercase();
    if norm == "viral" {
        format!("V9-VIRAL-001.{prefix}")
    } else {
        format!("V9-IMMORTAL-001.{prefix}")
    }
}

fn selected_profile(parsed: &crate::ParsedArgs) -> String {
    clean(
        parsed
            .flags
            .get("profile")
            .cloned()
            .unwrap_or_else(|| "immortal".to_string()),
        32,
    )
    .to_ascii_lowercase()
}

fn activation_command(profile: &str) -> String {
    if profile == "viral" {
        "infring seed deploy viral".to_string()
    } else {
        "infring seed deploy".to_string()
    }
}

fn packet_signature(packet: &Value) -> String {
    let key = std::env::var("DIRECTIVE_KERNEL_SIGNING_KEY")
        .ok()
        .map(|v| clean(v, 1024))
        .unwrap_or_default();
    if key.is_empty() {
        return format!(
            "unsigned:{}",
            sha256_hex_str(&serde_json::to_string(packet).unwrap_or_default())
        );
    }
    format!("sig:{}", crate::v8_kernel::keyed_digest_hex(&key, packet))
}

fn persist_packet(root: &Path, packet_id: &str, packet: &Value) -> Result<PathBuf, String> {
    let path = packets_dir(root).join(format!("{packet_id}.json"));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("packet_dir_create_failed:{}:{err}", parent.display()))?;
    }
    write_json(&path, packet)?;
    Ok(path)
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
                "type": "seed_protocol_error",
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

fn default_targets(profile: &str) -> Vec<String> {
    if profile == "viral" {
        vec![
            "swarm-alpha".to_string(),
            "swarm-beta".to_string(),
            "swarm-gamma".to_string(),
            "swarm-delta".to_string(),
        ]
    } else {
        vec![
            "vault-cold-1".to_string(),
            "vault-cold-2".to_string(),
            "vault-cold-3".to_string(),
        ]
    }
}

fn parse_targets(raw: Option<&String>, profile: &str, cap: usize) -> Vec<String> {
    let mut out = raw
        .map(|v| {
            v.split(',')
                .map(|node| clean(Some(node.trim()).unwrap_or(""), 120))
                .map(|node| node.to_ascii_lowercase())
                .filter(|node| !node.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if out.is_empty() {
        out = default_targets(profile);
    }
    out.truncate(cap);
    out
}

fn command_status(root: &Path) -> i32 {
    let state = load_state(root);
    let obj = state.as_object().cloned().unwrap_or_default();
    let replication_count = obj
        .get("replication_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let migration_count = obj
        .get("migration_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let compliance_checks = obj
        .get("compliance_checks")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let compliance_denies = obj
        .get("compliance_denies")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let archive_count = obj
        .get("archive_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let defense_event_count = obj
        .get("defense_event_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let provider_family_contract = obj
        .get("provider_family_contract")
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "active_provider_family": "moonshot",
                "provider_family_contract_targets": PROVIDER_FAMILY_CONTRACT_TARGETS,
                "provider_runtime_contract": true,
                "provider_auth_contract": true,
                "provider_registry_contract": true,
                "provider_discovery_contract": true
            })
        });
    let provider_family = normalize_provider_family(
        provider_family_contract
            .get("active_provider_family")
            .and_then(Value::as_str),
    );
    let provider_family_contract_ok = !provider_family.is_empty()
        && PROVIDER_FAMILY_CONTRACT_TARGETS
            .iter()
            .any(|target| target == &provider_family.as_str());

    let compliance_rate = if compliance_checks == 0 {
        1.0
    } else {
        (compliance_checks.saturating_sub(compliance_denies)) as f64 / (compliance_checks as f64)
    };
    let survival_fitness = ((replication_count as f64 + 1.0)
        / ((migration_count + defense_event_count + 1) as f64))
        .clamp(0.0, 5.0);
    let replication_rate = (replication_count as f64) / ((archive_count + 1) as f64);

    emit(
        root,
        json!({
            "ok": true,
            "type": "seed_protocol_status",
            "lane": "core/layer0/ops",
            "state": state,
            "seed_mode_dashboard": {
                "active_profile": obj.get("active_profile").cloned().unwrap_or(Value::Null),
                "replication_rate": replication_rate,
                "survival_fitness": survival_fitness,
                "compliance_rate": compliance_rate,
                "archive_merkle_root": obj.get("archive_merkle_root").cloned().unwrap_or(Value::Null),
                "quarantined_nodes": obj.get("quarantine").and_then(Value::as_object).map(|m| m.len()).unwrap_or(0),
                "provider_family_contract_ok": provider_family_contract_ok,
                "provider_family": provider_family
            },
            "provider_family_contract": provider_family_contract,
            "latest": read_json(&latest_path(root)),
            "claim_evidence": [
                {
                    "id": "V9-VIRAL-001.6",
                    "claim": "viral_seed_dashboard_surfaces_replication_fitness_and_compliance",
                    "evidence": {"replication_rate": replication_rate, "compliance_rate": compliance_rate}
                },
                {
                    "id": "V9-IMMORTAL-001.6",
                    "claim": "millennia_dashboard_surfaces_survival_fitness_and_archive_health",
                    "evidence": {"survival_fitness": survival_fitness, "archive_count": archive_count}
                },
                {
                    "id": "V9-IMMORTAL-001.7",
                    "claim": "seed_protocol_status_surfaces_provider_family_contract_posture",
                    "evidence": {"provider_family": provider_family, "provider_family_contract_ok": provider_family_contract_ok}
                }
            ]
        }),
    )
}
