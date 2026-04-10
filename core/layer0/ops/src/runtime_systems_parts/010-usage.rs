// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::runtime_systems (authoritative)
use crate::contract_lane_utils as lane_utils;
use crate::runtime_system_contracts::{
    actionable_profiles, looks_like_contract_id, profile_for, RuntimeSystemContractProfile,
};
use crate::{client_state_root, deterministic_receipt_hash, now_iso};
use llm_runtime::{
    choose_best_model, normalize_model_scores, ModelMetadata, ModelRuntimeKind, ModelSpecialty,
    RoutingRequest, WorkloadClass,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const LANE_ID: &str = "runtime_systems";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops runtime-systems <status|verify|run|build|manifest|roi-sweep|bootstrap|package|settle> [--system-id=<id>|--lane-id=<id>] [flags]");
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn receipt_hash(value: &Value) -> String {
    deterministic_receipt_hash(value)
}

fn profile_json(profile: RuntimeSystemContractProfile) -> Value {
    json!({
        "id": profile.id,
        "family": profile.family,
        "objective": profile.objective,
        "strict_conduit_only": profile.strict_conduit_only,
        "strict_fail_closed": profile.strict_fail_closed
    })
}

fn mutation_receipt_claim(system_id: &str, command: &str, apply: bool, strict: bool) -> Value {
    json!({
        "id": "runtime_system_mutation_receipted",
        "claim": "runtime_system_operations_emit_deterministic_receipts_and_state",
        "evidence": {
            "system_id": system_id,
            "command": command,
            "apply": apply,
            "strict": strict
        }
    })
}

fn parse_json(raw: Option<&str>) -> Result<Value, String> {
    let text = raw.ok_or_else(|| "missing_json_payload".to_string())?;
    serde_json::from_str::<Value>(text).map_err(|err| format!("invalid_json_payload:{err}"))
}

fn systems_dir(root: &Path) -> PathBuf {
    client_state_root(root).join("runtime_systems")
}

fn latest_path(root: &Path, system_id: &str) -> PathBuf {
    systems_dir(root).join(system_id).join("latest.json")
}

fn history_path(root: &Path, system_id: &str) -> PathBuf {
    systems_dir(root).join(system_id).join("history.jsonl")
}

fn contract_state_path(root: &Path, family: &str) -> PathBuf {
    systems_dir(root)
        .join("_contracts")
        .join(family)
        .join("state.json")
}

fn payload_number(payload: &Value, key: &str) -> Option<f64> {
    payload
        .get(key)
        .and_then(Value::as_f64)
        .or_else(|| payload.get(key).and_then(Value::as_i64).map(|v| v as f64))
        .or_else(|| payload.get(key).and_then(Value::as_u64).map(|v| v as f64))
}

fn payload_non_empty_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn payload_f64(payload: &Value, key: &str, fallback: f64) -> f64 {
    payload_number(payload, key).unwrap_or(fallback)
}

fn payload_bool(payload: &Value, key: &str, fallback: bool) -> bool {
    payload
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

fn payload_string(payload: &Value, key: &str, fallback: &str) -> String {
    payload_non_empty_string(payload, key)
        .unwrap_or_else(|| fallback.to_string())
}

fn payload_string_array(payload: &Value, key: &str, fallback: &[&str]) -> Vec<String> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| fallback.iter().map(|v| (*v).to_string()).collect())
}

fn payload_u64(payload: &Value, key: &str, fallback: u64) -> u64 {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .or_else(|| {
            payload
                .get(key)
                .and_then(Value::as_i64)
                .map(|v| v.max(0) as u64)
        })
        .unwrap_or(fallback)
}

fn payload_array(payload: &Value, key: &str) -> Vec<Value> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

fn missing_required_tokens(actual: &[String], required: &[&str]) -> Vec<String> {
    let set: BTreeSet<String> = actual.iter().map(|v| v.to_ascii_lowercase()).collect();
    required
        .iter()
        .filter_map(|token| {
            let canonical = token.to_ascii_lowercase();
            if set.contains(&canonical) {
                None
            } else {
                Some((*token).to_string())
            }
        })
        .collect()
}
