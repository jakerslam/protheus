// SPDX-License-Identifier: Apache-2.0
use crate::{clean, now_iso, parse_args};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
#[cfg(test)]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::contract_lane_utils as lane_utils;

const SCALE_IDS: [&str; 10] = [
    "V4-SCALE-001",
    "V4-SCALE-002",
    "V4-SCALE-003",
    "V4-SCALE-004",
    "V4-SCALE-005",
    "V4-SCALE-006",
    "V4-SCALE-007",
    "V4-SCALE-008",
    "V4-SCALE-009",
    "V4-SCALE-010",
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
    pub contract_dir: PathBuf,
    pub report_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budgets {
    pub max_cost_per_user_usd: f64,
    pub max_p95_latency_ms: i64,
    pub max_p99_latency_ms: i64,
    pub error_budget_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub version: String,
    pub enabled: bool,
    pub strict_default: bool,
    pub items: Vec<ProgramItem>,
    pub stage_gates: Vec<String>,
    pub paths: Paths,
    pub budgets: Budgets,
    pub policy_path: PathBuf,
}

fn normalize_id(v: &str) -> String {
    let out = clean(v.replace('`', ""), 80).to_ascii_uppercase();
    if out.len() == 12 && out.starts_with("V4-SCALE-") {
        out
    } else {
        String::new()
    }
}

fn to_bool(v: Option<&str>, fallback: bool) -> bool {
    lane_utils::parse_bool(v, fallback)
}

fn clamp_int(v: Option<i64>, lo: i64, hi: i64, fallback: i64) -> i64 {
    let Some(mut n) = v else {
        return fallback;
    };
    if n < lo {
        n = lo;
    }
    if n > hi {
        n = hi;
    }
    n
}

fn read_json(path: &Path) -> Value {
    lane_utils::read_json(path).unwrap_or(Value::Null)
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, row)
        .map_err(|e| format!("append_jsonl_failed:{}:{e}", path.display()))
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    lane_utils::write_json(path, value)
        .map_err(|e| format!("write_json_failed:{}:{e}", path.display()))
}

fn rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn resolve_path(root: &Path, raw: Option<&Value>, fallback_rel: &str) -> PathBuf {
    let fallback = root.join(fallback_rel);
    let Some(raw) = raw.and_then(Value::as_str) else {
        return fallback;
    };
    let clean_raw = clean(raw, 400);
    if clean_raw.is_empty() {
        return fallback;
    }
    let p = PathBuf::from(clean_raw);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

fn stable_hash(input: &str, len: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let hex = hex::encode(hasher.finalize());
    hex[..len.min(hex.len())].to_string()
}

pub fn default_policy(root: &Path) -> Policy {
    Policy {
        version: "1.0".to_string(),
        enabled: true,
        strict_default: true,
        items: SCALE_IDS
            .iter()
            .map(|id| ProgramItem {
                id: (*id).to_string(),
                title: (*id).to_string(),
            })
            .collect(),
        stage_gates: vec![
            "1k".to_string(),
            "10k".to_string(),
            "100k".to_string(),
            "1M".to_string(),
        ],
        paths: Paths {
            state_path: root.join("local/state/ops/scale_readiness_program/state.json"),
            latest_path: root.join("local/state/ops/scale_readiness_program/latest.json"),
            receipts_path: root.join("local/state/ops/scale_readiness_program/receipts.jsonl"),
            history_path: root.join("local/state/ops/scale_readiness_program/history.jsonl"),
            contract_dir: root.join("client/runtime/config/scale_readiness"),
            report_dir: root.join("local/state/ops/scale_readiness_program/reports"),
        },
        budgets: Budgets {
            max_cost_per_user_usd: 0.18,
            max_p95_latency_ms: 250,
            max_p99_latency_ms: 450,
            error_budget_pct: 0.01,
        },
        policy_path: root.join("client/runtime/config/scale_readiness_program_policy.json"),
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

    let items = raw
        .get("items")
        .and_then(Value::as_array)
        .map(|rows| {
            let mut seen = std::collections::HashSet::new();
            rows.iter()
                .filter_map(|row| {
                    let id = normalize_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
                    if id.is_empty() || seen.contains(&id) {
                        return None;
                    }
                    seen.insert(id.clone());
                    let title = clean(row.get("title").and_then(Value::as_str).unwrap_or(&id), 260);
                    Some(ProgramItem {
                        id: id.clone(),
                        title: if title.is_empty() { id } else { title },
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| base.items.clone());
    out.items = items;

    out.stage_gates = raw
        .get("stage_gates")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| clean(v, 20))
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| base.stage_gates.clone());

    let paths = raw.get("paths").cloned().unwrap_or(Value::Null);
    out.paths = Paths {
        state_path: resolve_path(
            root,
            paths.get("state_path"),
            "local/state/ops/scale_readiness_program/state.json",
        ),
        latest_path: resolve_path(
            root,
            paths.get("latest_path"),
            "local/state/ops/scale_readiness_program/latest.json",
        ),
        receipts_path: resolve_path(
            root,
            paths.get("receipts_path"),
            "local/state/ops/scale_readiness_program/receipts.jsonl",
        ),
        history_path: resolve_path(
            root,
            paths.get("history_path"),
            "local/state/ops/scale_readiness_program/history.jsonl",
        ),
        contract_dir: resolve_path(
            root,
            paths.get("contract_dir"),
            "client/runtime/config/scale_readiness",
        ),
        report_dir: resolve_path(
            root,
            paths.get("report_dir"),
            "local/state/ops/scale_readiness_program/reports",
        ),
    };

    let budgets = raw.get("budgets").cloned().unwrap_or(Value::Null);
    out.budgets = Budgets {
        max_cost_per_user_usd: budgets
            .get("max_cost_per_user_usd")
            .and_then(Value::as_f64)
            .unwrap_or(base.budgets.max_cost_per_user_usd),
        max_p95_latency_ms: clamp_int(
            budgets.get("max_p95_latency_ms").and_then(Value::as_i64),
            10,
            50_000,
            base.budgets.max_p95_latency_ms,
        ),
        max_p99_latency_ms: clamp_int(
            budgets.get("max_p99_latency_ms").and_then(Value::as_i64),
            10,
            50_000,
            base.budgets.max_p99_latency_ms,
        ),
        error_budget_pct: budgets
            .get("error_budget_pct")
            .and_then(Value::as_f64)
            .unwrap_or(base.budgets.error_budget_pct),
    };

    out.policy_path = if policy_path.is_absolute() {
        policy_path.to_path_buf()
    } else {
        root.join(policy_path)
    };

    out
}

fn load_state(policy: &Policy) -> Value {
    let fallback = json!({
        "schema_id": "scale_readiness_program_state",
        "schema_version": "1.0",
        "updated_at": now_iso(),
        "last_run": Value::Null,
        "lane_receipts": {},
        "current_stage": "1k",
        "autoscaling_profile": Value::Null,
        "async_pipeline_profile": Value::Null,
        "partition_profile": Value::Null,
        "cache_profile": Value::Null,
        "region_profile": Value::Null,
        "release_profile": Value::Null,
        "sre_profile": Value::Null,
        "abuse_profile": Value::Null,
        "economics_profile": Value::Null
    });
    let raw = read_json(&policy.paths.state_path);
    if !raw.is_object() {
        return fallback;
    }
    let mut merged = fallback.as_object().cloned().unwrap_or_default();
    for (k, v) in raw.as_object().cloned().unwrap_or_default() {
        merged.insert(k, v);
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

fn write_contract(
    policy: &Policy,
    name: &str,
    payload: &Value,
    apply: bool,
    root: &Path,
) -> Result<String, String> {
    let abs = policy.paths.contract_dir.join(name);
    if apply {
        write_json_atomic(&abs, payload)?;
    }
    Ok(rel_path(root, &abs))
}

fn run_json_script(root: &Path, script_rel: &str, args: &[String]) -> Value {
    let abs = root.join(script_rel);
    let out = Command::new("node")
        .arg(abs)
        .args(args)
        .current_dir(root)
        .output();

    let Ok(out) = out else {
        return json!({"ok": false, "status": 1, "payload": Value::Null, "stdout": "", "stderr": "spawn_failed"});
    };

    let status = out.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = clean(String::from_utf8_lossy(&out.stderr), 600);

    let payload = serde_json::from_str::<Value>(&stdout)
        .ok()
        .or_else(|| {
            let idx = stdout.find('{')?;
            serde_json::from_str::<Value>(&stdout[idx..]).ok()
        })
        .unwrap_or(Value::Null);

    json!({
        "ok": status == 0,
        "status": status,
        "payload": payload,
        "stdout": stdout,
        "stderr": stderr
    })
}

fn synth_load_summary(stage: &str) -> Value {
    match stage {
        "10k" => {
            json!({"dau": 10_000, "peak_concurrency": 1200, "rps": 1900, "write_ratio": 0.2, "read_ratio": 0.8})
        }
        "100k" => {
            json!({"dau": 100_000, "peak_concurrency": 12_000, "rps": 16_000, "write_ratio": 0.21, "read_ratio": 0.79})
        }
        "1M" => {
            json!({"dau": 1_000_000, "peak_concurrency": 125_000, "rps": 170_000, "write_ratio": 0.22, "read_ratio": 0.78})
        }
        _ => {
            json!({"dau": 1000, "peak_concurrency": 140, "rps": 280, "write_ratio": 0.18, "read_ratio": 0.82})
        }
    }
}

fn lane_scale(
    id: &str,
    policy: &Policy,
    state: &mut Value,
    apply: bool,
    strict: bool,
    root: &Path,
