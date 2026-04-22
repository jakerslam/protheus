// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
use crate::now_iso;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use parking_lot::Mutex;
use protheus_nexus_core_v1::spine_core::{
    run_background_hands_scheduler, run_evidence_run_plan, run_rsi_idle_hands_scheduler,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Seek;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::UNIX_EPOCH;
use sysinfo::Disks;

#[derive(Debug, Clone)]
struct CliArgs {
    command: String,
    mode: String,
    date: String,
    max_eyes: Option<i64>,
}

#[derive(Debug, Clone)]
struct StepResult {
    ok: bool,
    code: i32,
    payload: Option<Value>,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone)]
struct LedgerWriter {
    root: PathBuf,
    date: String,
    run_id: String,
    seq: u64,
    last_type: Option<String>,
}

#[derive(Debug, Clone)]
struct MechSuitPolicy {
    enabled: bool,
    heartbeat_hours: i64,
    manual_triggers_allowed: bool,
    quiet_non_critical: bool,
    silent_subprocess_output: bool,
    push_attention_queue: bool,
    attention_queue_path: String,
    attention_receipts_path: String,
    attention_latest_path: String,
    attention_max_queue_depth: i64,
    attention_ttl_hours: i64,
    attention_dedupe_window_hours: i64,
    attention_backpressure_drop_below: String,
    attention_escalate_levels: Vec<String>,
    ambient_stance: bool,
    dopamine_threshold_breach_only: bool,
    status_path: PathBuf,
    history_path: PathBuf,
    policy_path: PathBuf,
}

#[derive(Debug, Clone)]
struct SleepCleanupPolicy {
    enabled: bool,
    min_interval_minutes: i64,
    archive_root: PathBuf,
    archive_max_age_hours: i64,
    archive_keep_latest: usize,
    target_root: PathBuf,
    target_max_age_hours: i64,
    detached_worktree_max_age_hours: i64,
    disk_free_floor_percent: f64,
    hard_free_floor_percent: f64,
    pressure_target_free_percent: f64,
    pressure_jsonl_cap_bytes: u64,
    pressure_log_cap_bytes: u64,
    pressure_max_candidates: usize,
    pressure_min_age_hours: i64,
    state_path: PathBuf,
    history_path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
enum PressureAction {
    TrimTail { max_bytes: u64 },
    RemoveFile,
}

#[derive(Debug, Clone)]
struct PressureCandidate {
    path: PathBuf,
    size_bytes: u64,
    last_touch_ms: i64,
    action: PressureAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SleepCleanupMode {
    Normal,
    Purge,
}

fn stable_hash(seed: &str, len: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let hex = hex::encode(hasher.finalize());
    hex[..len.min(hex.len())].to_string()
}

fn receipt_hash(v: &Value) -> String {
    crate::deterministic_receipt_hash(v)
}

fn value_f64(value: Option<&Value>, fallback: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(fallback)
}

fn parse_clearance_level(raw: Option<String>, fallback: i64) -> i64 {
    raw.and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(fallback)
        .clamp(1, 5)
}

fn derive_duality_clearance(base: i64, debt_after: f64, harmony: f64, hard_block: bool) -> (i64, String) {
    if hard_block {
        return (1, "duality_toll_hard_block".to_string());
    }
    if debt_after >= 0.75 {
        return ((base - 1).max(1), "duality_toll_pressure".to_string());
    }
    if debt_after <= 0.2 && harmony >= 0.85 {
        return ((base + 1).min(5), "duality_harmony_boost".to_string());
    }
    (base, "duality_clearance_hold".to_string())
}
