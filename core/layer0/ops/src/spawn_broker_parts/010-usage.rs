// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops::spawn_broker (authoritative)
use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct PoolPolicy {
    min_cells: i64,
    max_cells: i64,
    reserve_cpu_threads: f64,
    reserve_ram_gb: f64,
    estimated_cpu_threads_per_cell: f64,
    estimated_ram_gb_per_cell: f64,
    max_cells_by_hardware: BTreeMap<String, i64>,
}

#[derive(Debug, Clone)]
struct QuotaPolicy {
    default_max_cells: i64,
    modules: BTreeMap<String, i64>,
}

#[derive(Debug, Clone)]
struct LeasePolicy {
    enabled: bool,
    default_ttl_sec: i64,
    max_ttl_sec: i64,
}

#[derive(Debug, Clone)]
struct SpawnPolicy {
    version: String,
    pool: PoolPolicy,
    quotas: QuotaPolicy,
    leases: LeasePolicy,
}

#[derive(Debug, Clone)]
struct Allocation {
    cells: i64,
    ts: String,
    reason: String,
    lease_expires_at: Option<String>,
}

#[derive(Debug, Clone)]
struct BrokerState {
    version: i64,
    ts: String,
    allocations: BTreeMap<String, Allocation>,
}

#[derive(Debug, Clone)]
struct RouterPlan {
    ok: bool,
    payload: Value,
    error: Option<String>,
    transport: Option<String>,
}

#[derive(Debug, Clone)]
struct HardwareBounds {
    hardware_class: Option<String>,
    cpu_threads: Option<f64>,
    ram_gb: Option<f64>,
    cap_by_class: i64,
    cap_by_cpu: i64,
    cap_by_ram: i64,
    global_max_cells: i64,
}

#[derive(Debug, Clone)]
struct Limits {
    module: String,
    global_max_cells: i64,
    module_quota_max_cells: i64,
    module_current_cells: i64,
    allocated_other_cells: i64,
    allocated_total_cells: i64,
    free_global_cells: i64,
    max_cells: i64,
}

#[derive(Debug, Clone)]
struct AutopauseState {
    active: bool,
    source: Option<String>,
    reason: Option<String>,
    until: Option<String>,
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops spawn-broker status [--module=<name>] [--profile=<id>]");
    println!("  protheus-ops spawn-broker request --module=<name> --requested_cells=<n> [--profile=<id>] [--reason=<text>] [--apply=1|0] [--lease_sec=<n>]");
    println!("  protheus-ops spawn-broker release --module=<name> [--reason=<text>]");
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

fn now_ms() -> i64 {
    let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };
    i64::try_from(dur.as_millis()).unwrap_or(0)
}

fn clamp_i64(v: i64, lo: i64, hi: i64) -> i64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

fn clamp_f64(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    lane_utils::parse_flag(argv, key, true)
}

fn parse_bool(raw: Option<&str>, fallback: bool) -> bool {
    lane_utils::parse_bool(raw, fallback)
}

fn parse_i64(raw: Option<&str>, fallback: i64) -> i64 {
    raw.and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(fallback)
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|e| format!("mkdir_failed:{}:{e}", parent.display()))
}

fn read_json(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&text).ok()
}

fn write_json_atomic(path: &Path, payload: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let tmp = path.with_extension(format!("tmp-{}-{}", std::process::id(), now_ms().max(0)));
    let encoded = serde_json::to_string_pretty(payload).unwrap_or_else(|_| "{}".to_string()) + "\n";
    fs::write(&tmp, encoded).map_err(|e| format!("write_tmp_failed:{}:{e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| {
        format!(
            "rename_tmp_failed:{}=>{}:{e}",
            tmp.display(),
            path.display()
        )
    })
}

fn append_jsonl(path: &Path, payload: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, payload)
}

fn root_client_runtime(root: &Path) -> PathBuf {
    root.join("client").join("runtime")
}

fn policy_path(root: &Path) -> PathBuf {
    if let Ok(v) = std::env::var("SPAWN_POLICY_PATH") {
        let s = v.trim();
        if !s.is_empty() {
            let p = PathBuf::from(s);
            if p.is_absolute() {
                return p;
            }
            return root.join(p);
        }
    }
    root_client_runtime(root)
        .join("config")
        .join("spawn_policy.json")
}

fn state_dir(root: &Path) -> PathBuf {
    if let Ok(v) = std::env::var("SPAWN_STATE_DIR") {
        let s = v.trim();
        if !s.is_empty() {
            let p = PathBuf::from(s);
            if p.is_absolute() {
                return p;
            }
            return root.join(p);
        }
    }
    root_client_runtime(root)
        .join("local")
        .join("state")
        .join("spawn")
}

fn state_path(root: &Path) -> PathBuf {
    state_dir(root).join("allocations.json")
}

fn events_path(root: &Path) -> PathBuf {
    state_dir(root).join("events.jsonl")
}

fn router_script_path(root: &Path) -> PathBuf {
    if let Ok(v) = std::env::var("SPAWN_ROUTER_SCRIPT") {
        let s = v.trim();
        if !s.is_empty() {
            let p = PathBuf::from(s);
            if p.is_absolute() {
                return p;
            }
            return root.join(p);
        }
    }
    root_client_runtime(root)
        .join("systems")
        .join("routing")
        .join("model_router.js")
}

fn autopause_path(root: &Path) -> PathBuf {
    if let Ok(v) = std::env::var("SPAWN_TOKEN_BUDGET_AUTOPAUSE_PATH") {
        let s = v.trim();
        if !s.is_empty() {
            let p = PathBuf::from(s);
            if p.is_absolute() {
                return p;
            }
            return root.join(p);
        }
    }
    root_client_runtime(root)
        .join("local")
        .join("state")
        .join("autonomy")
        .join("budget_autopause.json")
}

fn default_policy() -> SpawnPolicy {
    let mut class_caps = BTreeMap::new();
    class_caps.insert("tiny".to_string(), 1);
    class_caps.insert("small".to_string(), 2);
    class_caps.insert("medium".to_string(), 3);
    class_caps.insert("large".to_string(), 4);
    class_caps.insert("xlarge".to_string(), 6);
    SpawnPolicy {
        version: "1.0".to_string(),
        pool: PoolPolicy {
            min_cells: 0,
            max_cells: 6,
            reserve_cpu_threads: 2.0,
            reserve_ram_gb: 4.0,
            estimated_cpu_threads_per_cell: 1.0,
            estimated_ram_gb_per_cell: 1.2,
            max_cells_by_hardware: class_caps,
        },
        quotas: QuotaPolicy {
            default_max_cells: 2,
            modules: BTreeMap::new(),
        },
        leases: LeasePolicy {
            enabled: true,
            default_ttl_sec: 300,
            max_ttl_sec: 3600,
        },
    }
}

fn load_policy(root: &Path) -> SpawnPolicy {
    let mut out = default_policy();
    let path = policy_path(root);
    let Some(raw) = read_json(&path) else {
        return out;
    };

    if let Some(version) = raw.get("version").and_then(Value::as_str) {
        out.version = version.trim().to_string();
    }
    if let Some(pool) = raw.get("pool").and_then(Value::as_object) {
        out.pool.min_cells = clamp_i64(
            pool.get("min_cells")
                .and_then(Value::as_i64)
                .unwrap_or(out.pool.min_cells),
            0,
            4096,
        );
        out.pool.max_cells = clamp_i64(
            pool.get("max_cells")
                .and_then(Value::as_i64)
                .unwrap_or(out.pool.max_cells),
            out.pool.min_cells,
            8192,
        );
        out.pool.reserve_cpu_threads = clamp_f64(
            pool.get("reserve_cpu_threads")
                .and_then(Value::as_f64)
                .unwrap_or(out.pool.reserve_cpu_threads),
            0.0,
            4096.0,
        );
        out.pool.reserve_ram_gb = clamp_f64(
            pool.get("reserve_ram_gb")
                .and_then(Value::as_f64)
                .unwrap_or(out.pool.reserve_ram_gb),
            0.0,
            4096.0,
        );
        out.pool.estimated_cpu_threads_per_cell = clamp_f64(
            pool.get("estimated_cpu_threads_per_cell")
                .and_then(Value::as_f64)
                .unwrap_or(out.pool.estimated_cpu_threads_per_cell),
            0.1,
            512.0,
        );
        out.pool.estimated_ram_gb_per_cell = clamp_f64(
            pool.get("estimated_ram_gb_per_cell")
                .and_then(Value::as_f64)
                .unwrap_or(out.pool.estimated_ram_gb_per_cell),
            0.1,
            512.0,
        );
        if let Some(map) = pool.get("max_cells_by_hardware").and_then(Value::as_object) {
            let mut next = BTreeMap::new();
            for (k, v) in map {
                let n = v.as_i64().unwrap_or(out.pool.max_cells);
                next.insert(
                    k.trim().to_ascii_lowercase(),
                    clamp_i64(n, 0, out.pool.max_cells),
                );
            }
            if !next.is_empty() {
                out.pool.max_cells_by_hardware = next;
            }
        }
    }
    if let Some(quotas) = raw.get("quotas").and_then(Value::as_object) {
        out.quotas.default_max_cells = clamp_i64(
            quotas
                .get("default_max_cells")
                .and_then(Value::as_i64)
                .unwrap_or(out.quotas.default_max_cells),
            0,
            out.pool.max_cells,
        );
        if let Some(modules) = quotas.get("modules").and_then(Value::as_object) {
            let mut next = BTreeMap::new();
            for (name, row) in modules {
                let max_cells = row
                    .as_object()
                    .and_then(|obj| obj.get("max_cells"))
                    .and_then(Value::as_i64)
                    .unwrap_or(out.quotas.default_max_cells);
                next.insert(
                    name.trim().to_ascii_lowercase(),
                    clamp_i64(max_cells, 0, out.pool.max_cells),
                );
            }
            out.quotas.modules = next;
        }
    }
    if let Some(leases) = raw.get("leases").and_then(Value::as_object) {
        out.leases.enabled = leases
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(out.leases.enabled);
        out.leases.default_ttl_sec = clamp_i64(
            leases
                .get("default_ttl_sec")
                .and_then(Value::as_i64)
                .unwrap_or(out.leases.default_ttl_sec),
            5,
            172800,
        );
        out.leases.max_ttl_sec = clamp_i64(
            leases
                .get("max_ttl_sec")
                .and_then(Value::as_i64)
                .unwrap_or(out.leases.max_ttl_sec),
            out.leases.default_ttl_sec,
            172800,
        );
    }
    out
}

fn default_state() -> BrokerState {
    BrokerState {
        version: 1,
        ts: now_iso(),
        allocations: BTreeMap::new(),
    }
}

fn parse_allocation(raw: &Value) -> Option<Allocation> {
    let obj = raw.as_object()?;
    Some(Allocation {
        cells: clamp_i64(
            obj.get("cells").and_then(Value::as_i64).unwrap_or(0),
            0,
            4096,
        ),
        ts: obj
            .get("ts")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string(),
        reason: obj
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string(),
        lease_expires_at: obj
            .get("lease_expires_at")
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    })
}

