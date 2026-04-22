// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
use crate::now_iso;
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::System;
use walkdir::WalkDir;

const DEFAULT_POLICY_REL: &str = "client/runtime/config/autotest_policy.json";

#[derive(Debug, Clone)]
struct CliArgs {
    positional: Vec<String>,
    flags: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct RuntimePaths {
    policy_path: PathBuf,
    state_dir: PathBuf,
    registry_path: PathBuf,
    status_path: PathBuf,
    events_path: PathBuf,
    latest_path: PathBuf,
    reports_dir: PathBuf,
    runs_dir: PathBuf,
    module_root: PathBuf,
    test_root: PathBuf,
    spine_runs_dir: PathBuf,
    pain_signals_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecPolicy {
    default_scope: String,
    strict: bool,
    max_tests_per_run: usize,
    run_timeout_ms: i64,
    timeout_ms_per_test: i64,
    retry_flaky_once: bool,
    flaky_quarantine_after: u32,
    flaky_quarantine_sec: i64,
    midrun_resource_guard: bool,
    resource_recheck_every_tests: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AlertsPolicy {
    emit_untested: bool,
    emit_changed_without_tests: bool,
    max_untested_in_report: usize,
    max_failed_in_report: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeGuardPolicy {
    spine_hot_window_sec: i64,
    max_rss_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Policy {
    version: String,
    enabled: bool,
    strict_default: bool,
    module_include_ext: Vec<String>,
    module_ignore_prefixes: Vec<String>,
    test_include_suffix: String,
    test_ignore_prefixes: Vec<String>,
    min_match_score: i64,
    min_token_len: usize,
    shared_token_score: i64,
    basename_contains_score: i64,
    layer_hint_score: i64,
    explicit_prefix_maps: BTreeMap<String, Vec<String>>,
    critical_commands: Vec<String>,
    execution: ExecPolicy,
    alerts: AlertsPolicy,
    runtime_guard: RuntimeGuardPolicy,
    sleep_window_start_hour: u32,
    sleep_window_end_hour: u32,
    external_health_paths: Vec<String>,
    external_health_window_hours: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SeedFields {
    owner: Option<String>,
    priority: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct GuardMeta {
    ok: bool,
    reason: Option<String>,
    files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ModuleRow {
    id: String,
    path: String,
    fingerprint: String,
    checked: bool,
    changed: bool,
    is_new: bool,
    untested: bool,
    mapped_test_ids: Vec<String>,
    mapped_test_count: usize,
    last_change_ts: Option<String>,
    last_test_ts: Option<String>,
    last_pass_ts: Option<String>,
    last_fail_ts: Option<String>,
    seed_fields: SeedFields,
    health_state: Option<String>,
    health_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TestRow {
    id: String,
    kind: String,
    path: Option<String>,
    command: String,
    critical: bool,
    last_status: String,
    last_exit_code: Option<i32>,
    last_run_ts: Option<String>,
    last_duration_ms: Option<u128>,
    last_stdout_excerpt: Option<String>,
    last_stderr_excerpt: Option<String>,
    last_guard: Option<GuardMeta>,
    last_retry_count: Option<u32>,
    last_flaky: Option<bool>,
    consecutive_flaky: Option<u32>,
    quarantined_until_ts: Option<String>,
    last_pass_ts: Option<String>,
    last_fail_ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AlertState {
    emitted_signatures: HashMap<String, String>,
    latest: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StatusState {
    version: String,
    updated_at: Option<String>,
    modules: HashMap<String, ModuleRow>,
    tests: HashMap<String, TestRow>,
    alerts: AlertState,
    last_sync: Option<String>,
    last_run: Option<String>,
    last_report: Option<String>,
}

#[derive(Debug, Clone)]
struct ModuleCandidate {
    id: String,
    path: String,
    abs_path: PathBuf,
    basename: String,
}

#[derive(Debug, Clone)]
struct TestCandidate {
    id: String,
    kind: String,
    path: String,
    command: String,
    stem: String,
}

#[derive(Debug, Clone, Default)]
struct GuardResult {
    ok: bool,
    reason: Option<String>,
    files: Vec<String>,
    stderr_excerpt: Option<String>,
    stdout_excerpt: Option<String>,
    duration_ms: u128,
}

#[derive(Debug, Clone)]
struct CommandResult {
    ok: bool,
    exit_code: i32,
    signal: Option<String>,
    timed_out: bool,
    duration_ms: u128,
    stdout_excerpt: String,
    stderr_excerpt: String,
}

#[derive(Debug, Clone)]
struct PrioritizedTest {
    id: String,
    score: i64,
    priority: String,
    test: TestRow,
}

fn parse_cli(argv: &[String]) -> CliArgs {
    let mut positional = Vec::new();
    let mut flags = HashMap::new();
    let mut i = 0usize;
    while i < argv.len() {
        let tok = argv[i].trim().to_string();
        if tok == "--" {
            positional.extend(argv.iter().skip(i + 1).cloned());
            break;
        }
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

fn short_text(v: &str, max: usize) -> String {
    let compact = v
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    if compact.len() <= max {
        compact
    } else {
        format!("{}...", &compact[..max])
    }
}

fn stable_hash(seed: &str, len: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let hex = hex::encode(hasher.finalize());
    hex[..len.min(hex.len())].to_string()
}

fn stable_id(seed: &str, prefix: &str) -> String {
    format!("{}_{}", prefix, stable_hash(seed, 14))
}

fn rel_path(root: &Path, abs: &Path) -> String {
    abs.strip_prefix(root)
        .unwrap_or(abs)
        .to_string_lossy()
        .replace('\\', "/")
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

fn receipt_hash(v: &Value) -> String {
    crate::deterministic_receipt_hash(v)
}

fn default_policy() -> Policy {
    Policy {
        version: "1.0".to_string(),
        enabled: true,
        strict_default: true,
        module_include_ext: vec![".ts".to_string()],
        module_ignore_prefixes: vec!["client/runtime/systems/ops/visualizer/".to_string()],
        test_include_suffix: ".test.ts".to_string(),
        test_ignore_prefixes: Vec::new(),
        min_match_score: 4,
        min_token_len: 4,
        shared_token_score: 2,
        basename_contains_score: 4,
        layer_hint_score: 2,
        explicit_prefix_maps: BTreeMap::from([
            (
                "client/runtime/systems/security/".to_string(),
                vec![
                    "tests/client-memory-tools/security_integrity.test.ts".to_string(),
                    "tests/client-memory-tools/guard_remote_gate.test.ts".to_string(),
                    "tests/client-memory-tools/directive_gate.test.ts".to_string(),
                ],
            ),
            (
                "client/runtime/systems/spine/".to_string(),
                vec!["tests/client-memory-tools/spine_evidence_run_plan.test.ts".to_string()],
            ),
        ]),
        critical_commands: vec![
            "node client/runtime/systems/ops/typecheck_systems.js".to_string(),
            "node client/runtime/systems/ops/ts_clone_drift_guard.js --baseline=client/runtime/config/ts_clone_drift_baseline.json"
                .to_string(),
            "node client/runtime/systems/spine/contract_check.js".to_string(),
        ],
        execution: ExecPolicy {
            default_scope: "changed".to_string(),
            strict: false,
            max_tests_per_run: 25,
            run_timeout_ms: 300_000,
            timeout_ms_per_test: 180_000,
            retry_flaky_once: true,
            flaky_quarantine_after: 3,
            flaky_quarantine_sec: 3_600,
            midrun_resource_guard: true,
            resource_recheck_every_tests: 1,
        },
        alerts: AlertsPolicy {
            emit_untested: true,
            emit_changed_without_tests: true,
            max_untested_in_report: 40,
            max_failed_in_report: 40,
        },
        runtime_guard: RuntimeGuardPolicy {
            spine_hot_window_sec: 1_200,
            max_rss_mb: 8_192.0,
        },
        sleep_window_start_hour: 0,
        sleep_window_end_hour: 7,
        external_health_paths: Vec::new(),
        external_health_window_hours: 24,
    }
}
