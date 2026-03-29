// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_iso};
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_POLICY_REL: &str = "client/runtime/config/autotest_doctor_policy.json";

#[derive(Debug, Clone)]
struct CliArgs {
    positional: Vec<String>,
    flags: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct RuntimePaths {
    policy_path: PathBuf,
    state_dir: PathBuf,
    runs_dir: PathBuf,
    latest_path: PathBuf,
    history_path: PathBuf,
    events_path: PathBuf,
    state_path: PathBuf,
    autotest_runs_dir: PathBuf,
    autotest_latest_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GatingPolicy {
    min_consecutive_failures: u32,
    max_actions_per_run: u32,
    cooldown_sec_per_signature: i64,
    max_repairs_per_signature_per_day: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KillSwitchPolicy {
    enabled: bool,
    window_hours: i64,
    max_unknown_signatures_per_window: u32,
    max_suspicious_signatures_per_window: u32,
    max_repairs_per_window: u32,
    max_rollbacks_per_window: u32,
    max_same_signature_repairs_per_window: u32,
    auto_reset_hours: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SleepWindow {
    enabled: bool,
    start_hour: u32,
    end_hour: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Policy {
    version: String,
    enabled: bool,
    shadow_mode: bool,
    sleep_window_local: SleepWindow,
    gating: GatingPolicy,
    kill_switch: KillSwitchPolicy,
    recipes: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SignatureState {
    consecutive_failures: u32,
    total_failures: u32,
    total_repairs: u32,
    total_rollbacks: u32,
    last_fail_ts: Option<String>,
    last_repair_ts: Option<String>,
    last_recipe_id: Option<String>,
    last_outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct KillSwitchState {
    engaged: bool,
    reason: Option<String>,
    engaged_at: Option<String>,
    auto_release_at: Option<String>,
    last_trip_meta: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DoctorState {
    updated_at: Option<String>,
    signatures: HashMap<String, SignatureState>,
    history: Vec<Value>,
    kill_switch: KillSwitchState,
}

#[derive(Debug, Clone)]
struct TrustedTestPath {
    path: Option<String>,
    trusted: bool,
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureSignature {
    signature_id: String,
    kind: String,
    test_id: Option<String>,
    command: Option<String>,
    test_path: Option<String>,
    trusted_test_command: bool,
    untrusted_reason: Option<String>,
    exit_code: Option<i64>,
    guard_ok: bool,
    guard_reason: Option<String>,
    stderr_excerpt: Option<String>,
    stdout_excerpt: Option<String>,
    guard_files: Vec<String>,
    flaky: bool,
}

fn parse_cli(argv: &[String]) -> CliArgs {
    let mut positional = Vec::new();
    let mut flags = HashMap::new();
    let mut i = 0usize;
    while i < argv.len() {
        let tok = argv[i].trim().to_string();
        if !tok.starts_with("--") {
            positional.push(argv[i].clone());
            i += 1;
            continue;
        }
        if let Some((k, v)) = tok.split_once('=') {
            flags.insert(k.trim_start_matches("--").to_string(), v.to_string());
            i += 1;
            continue;
        }
        let key = tok.trim_start_matches("--").to_string();
        if let Some(next) = argv.get(i + 1) {
            if !next.starts_with("--") {
                flags.insert(key, next.clone());
                i += 2;
                continue;
            }
        }
        flags.insert(key, "true".to_string());
        i += 1;
    }
    CliArgs { positional, flags }
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

fn clamp_i64(v: Option<&str>, lo: i64, hi: i64, fallback: i64) -> i64 {
    let Some(raw) = v else {
        return fallback;
    };
    let Ok(mut n) = raw.trim().parse::<i64>() else {
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

fn normalize_token(v: &str, max_len: usize) -> String {
    let mut out = String::new();
    for ch in v.trim().to_ascii_lowercase().chars().take(max_len) {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '/' | '-') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out.split('_')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn clean_text(v: &str, max_len: usize) -> String {
    let compact = v
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    if compact.len() <= max_len {
        compact
    } else {
        compact[..max_len].to_string()
    }
}

fn stable_hash(seed: &str, len: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let hex = hex::encode(hasher.finalize());
    hex[..len.min(hex.len())].to_string()
}

fn stable_id(prefix: &str, seed: &str) -> String {
    format!("{prefix}_{}", stable_hash(seed, 16))
}

fn receipt_hash(v: &Value) -> String {
    deterministic_receipt_hash(v)
}

fn read_json(path: &Path) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect()
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("create_dir_failed:{}:{e}", path.display()))
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    let mut payload =
        serde_json::to_string_pretty(value).map_err(|e| format!("encode_json:{e}"))?;
    payload.push('\n');
    fs::write(&tmp, payload).map_err(|e| format!("write_tmp_failed:{}:{e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename_tmp_failed:{}:{e}", path.display()))
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let mut payload = serde_json::to_string(row).map_err(|e| format!("encode_row:{e}"))?;
    payload.push('\n');
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, payload.as_bytes()))
        .map_err(|e| format!("append_jsonl_failed:{}:{e}", path.display()))
}

fn rel_path(root: &Path, abs: &Path) -> String {
    abs.strip_prefix(root)
        .unwrap_or(abs)
        .to_string_lossy()
        .replace('\\', "/")
}

fn default_policy() -> Policy {
    Policy {
        version: "1.0".to_string(),
        enabled: true,
        shadow_mode: true,
        sleep_window_local: SleepWindow {
            enabled: true,
            start_hour: 0,
            end_hour: 7,
        },
        gating: GatingPolicy {
            min_consecutive_failures: 2,
            max_actions_per_run: 2,
            cooldown_sec_per_signature: 1800,
            max_repairs_per_signature_per_day: 3,
        },
        kill_switch: KillSwitchPolicy {
            enabled: true,
            window_hours: 24,
            max_unknown_signatures_per_window: 4,
            max_suspicious_signatures_per_window: 2,
            max_repairs_per_window: 12,
            max_rollbacks_per_window: 3,
            max_same_signature_repairs_per_window: 4,
            auto_reset_hours: 12,
        },
        recipes: HashMap::from([
            (
                "guard_blocked".to_string(),
                vec![
                    "inspect_guard_context".to_string(),
                    "verify_allowlist_scope".to_string(),
                ],
            ),
            (
                "timeout".to_string(),
                vec![
                    "increase_timeout_budget".to_string(),
                    "retest_once".to_string(),
                ],
            ),
            (
                "exit_nonzero".to_string(),
                vec![
                    "capture_failure_context".to_string(),
                    "retest_once".to_string(),
                ],
            ),
            (
                "assertion_failed".to_string(),
                vec![
                    "collect_assertion_diff".to_string(),
                    "retest_once".to_string(),
                ],
            ),
            (
                "flaky".to_string(),
                vec![
                    "mark_flaky_quarantine".to_string(),
                    "retest_once".to_string(),
                ],
            ),
        ]),
    }
}

fn runtime_paths(root: &Path, policy_path: &Path) -> RuntimePaths {
    let state_dir = std::env::var("AUTOTEST_DOCTOR_STATE_DIR")
        .ok()
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("local/state/ops/autotest_doctor"));

    RuntimePaths {
        policy_path: std::env::var("AUTOTEST_DOCTOR_POLICY_PATH")
            .ok()
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| if p.is_absolute() { p } else { root.join(p) })
            .unwrap_or_else(|| policy_path.to_path_buf()),
        state_dir: state_dir.clone(),
        runs_dir: state_dir.join("runs"),
        latest_path: state_dir.join("latest.json"),
        history_path: state_dir.join("history.jsonl"),
        events_path: state_dir.join("events.jsonl"),
        state_path: state_dir.join("state.json"),
        autotest_runs_dir: std::env::var("AUTOTEST_DOCTOR_AUTOTEST_RUNS_DIR")
            .ok()
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| if p.is_absolute() { p } else { root.join(p) })
            .unwrap_or_else(|| root.join("local/state/ops/autotest/runs")),
        autotest_latest_path: std::env::var("AUTOTEST_DOCTOR_AUTOTEST_LATEST_PATH")
            .ok()
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| if p.is_absolute() { p } else { root.join(p) })
            .unwrap_or_else(|| root.join("local/state/ops/autotest/latest.json")),
    }
}
