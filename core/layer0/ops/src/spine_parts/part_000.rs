// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
use crate::{deterministic_receipt_hash, now_iso};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use parking_lot::Mutex;
use protheus_spine_core_v1::{
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
    deterministic_receipt_hash(v)
}

fn receipt_ledger_io_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn to_base36(mut n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while n > 0 {
        let digit = (n % 36) as u8;
        let ch = if digit < 10 {
            (b'0' + digit) as char
        } else {
            (b'a' + (digit - 10)) as char
        };
        out.push(ch);
        n /= 36;
    }
    out.into_iter().rev().collect()
}

fn parse_cli(argv: &[String]) -> Option<CliArgs> {
    if argv.is_empty() {
        return None;
    }

    let mut idx = 0usize;
    let mut command = "run".to_string();
    let mut mode = argv[idx].to_ascii_lowercase();
    if mode == "status" {
        command = "status".to_string();
        mode = "daily".to_string();
    } else if mode == "run" {
        idx += 1;
        mode = argv.get(idx)?.to_ascii_lowercase();
    }

    if command != "status" && mode != "eyes" && mode != "daily" {
        return None;
    }

    if command != "status" {
        idx += 1;
    }
    let mut date = argv
        .get(idx)
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() == 10 && s.chars().nth(4) == Some('-') && s.chars().nth(7) == Some('-'))
        .unwrap_or_else(|| now_iso()[..10].to_string());

    let mut max_eyes = None::<i64>;
    let mut i = 0usize;
    while i < argv.len() {
        let token = argv[i].trim();
        if let Some((k, v)) = token.split_once('=') {
            if k == "--max-eyes" {
                if let Ok(n) = v.parse::<i64>() {
                    max_eyes = Some(n.clamp(1, 500));
                }
            } else if k == "--mode" {
                let candidate = v.trim().to_ascii_lowercase();
                if candidate == "eyes" || candidate == "daily" {
                    mode = candidate;
                }
            } else if k == "--date" {
                let candidate = v.trim();
                if candidate.len() == 10
                    && candidate.chars().nth(4) == Some('-')
                    && candidate.chars().nth(7) == Some('-')
                {
                    date = candidate.to_string();
                }
            }
            i += 1;
            continue;
        }
        if token == "--max-eyes" {
            if let Some(next) = argv.get(i + 1) {
                if !next.starts_with("--") {
                    if let Ok(n) = next.trim().parse::<i64>() {
                        max_eyes = Some(n.clamp(1, 500));
                    }
                    i += 2;
                    continue;
                }
            }
        } else if token == "--mode" {
            if let Some(next) = argv.get(i + 1) {
                let candidate = next.trim().to_ascii_lowercase();
                if !next.starts_with("--") && (candidate == "eyes" || candidate == "daily") {
                    mode = candidate;
                    i += 2;
                    continue;
                }
            }
        } else if token == "--date" {
            if let Some(next) = argv.get(i + 1) {
                let candidate = next.trim();
                if !next.starts_with("--")
                    && candidate.len() == 10
                    && candidate.chars().nth(4) == Some('-')
                    && candidate.chars().nth(7) == Some('-')
                {
                    date = candidate.to_string();
                    i += 2;
                    continue;
                }
            }
        }
        i += 1;
    }

    Some(CliArgs {
        command,
        mode,
        date,
        max_eyes,
    })
}

fn usage() {
    eprintln!("Usage:");
    eprintln!("  protheus-ops spine eyes [YYYY-MM-DD] [--max-eyes=N]");
    eprintln!("  protheus-ops spine daily [YYYY-MM-DD] [--max-eyes=N]");
    eprintln!("  protheus-ops spine run [eyes|daily] [YYYY-MM-DD] [--max-eyes=N]");
    eprintln!("  protheus-ops spine status [--mode=eyes|daily] [--date=YYYY-MM-DD]");
    eprintln!(
        "  protheus-ops spine sleep-cleanup <run|plan|status|purge> [--apply=1|0] [--force=1|0]"
    );
    eprintln!(
        "  protheus-ops spine background-hands-scheduler <configure|schedule|status> [flags]"
    );
    eprintln!("  protheus-ops spine rsi-idle-hands-scheduler <run|status> [flags]");
    eprintln!("  protheus-ops spine evidence-run-plan [--configured-runs=N] [--budget-pressure=none|soft|hard] [--projected-pressure=none|soft|hard]");
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn cli_error_receipt(argv: &[String], error: &str, code: i32) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": "spine_cli_error",
        "ts": ts,
        "mode": "unknown",
        "date": ts[..10].to_string(),
        "argv": argv,
        "error": error,
        "exit_code": code,
        "claim_evidence": [
            {
                "id": "fail_closed_cli",
                "claim": "spine_cli_invalid_args_fail_closed_with_deterministic_receipt",
                "evidence": {
                    "error": error,
                    "argv_len": argv.len()
                }
            }
        ],
        "persona_lenses": {
            "guardian": {
                "constitution_integrity_ok": true
            },
            "strategist": {
                "mode": "cli_error"
            }
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn run_node_json(root: &Path, args: &[String]) -> StepResult {
    let output = Command::new("node")
        .args(args)
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let payload = parse_json_payload(&stdout);
            StepResult {
                ok: out.status.success(),
                code: out.status.code().unwrap_or(1),
                payload,
                stdout,
                stderr,
            }
        }
        Err(err) => StepResult {
            ok: false,
            code: 1,
            payload: None,
            stdout: String::new(),
            stderr: format!("spawn_failed:{err}"),
        },
    }
}

fn run_ops_domain_json(
    root: &Path,
    domain: &str,
    args: &[String],
    run_context: Option<&str>,
) -> StepResult {
    let root_buf = root.to_path_buf();
    let (command, mut command_args) = resolve_protheus_ops_command(&root_buf, domain);
    command_args.extend(args.iter().cloned());

    let mut cmd = Command::new(command);
    cmd.args(command_args)
        .current_dir(root)
        .env(
            "PROTHEUS_NODE_BINARY",
            std::env::var("PROTHEUS_NODE_BINARY").unwrap_or_else(|_| "node".to_string()),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(context) = run_context {
        let trimmed = context.trim();
        if !trimmed.is_empty() {
            cmd.env("SPINE_RUN_CONTEXT", trimmed);
        }
    }

    match cmd.output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let payload = parse_json_payload(&stdout);
            StepResult {
                ok: out.status.success(),
                code: out.status.code().unwrap_or(1),
                payload,
                stdout,
                stderr,
            }
        }
        Err(err) => StepResult {
            ok: false,
            code: 1,
            payload: None,
            stdout: String::new(),
            stderr: format!("spawn_failed:{err}"),
        },
    }
}

fn resolve_protheus_ops_command(root: &Path, domain: &str) -> (String, Vec<String>) {
    if let Some(bin) = std::env::var("PROTHEUS_OPS_BIN").ok() {
        let trimmed = bin.trim();
        if !trimmed.is_empty() {
            return (trimmed.to_string(), vec![domain.to_string()]);
        }
    }

    let release = root.join("target").join("release").join("protheus-ops");
    if release.exists() {
        return (
            release.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }
    let debug = root.join("target").join("debug").join("protheus-ops");
    if debug.exists() {
        return (
            debug.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }

    (
        "cargo".to_string(),
        vec![
            "run".to_string(),
            "--quiet".to_string(),
            "--manifest-path".to_string(),
            "core/layer0/ops/Cargo.toml".to_string(),
            "--bin".to_string(),
            "protheus-ops".to_string(),
            "--".to_string(),
            domain.to_string(),
        ],
    )
}

fn enqueue_spine_attention(root: &Path, source_type: &str, severity: &str, summary: &str) {
    let mut event = json!({
        "ts": now_iso(),
        "type": source_type,
        "source": "spine",
        "source_type": source_type,
        "severity": severity,
        "summary": summary,
        "attention_key": format!("spine:{source_type}")
    });
    event["receipt_hash"] = Value::String(receipt_hash(&event));
    let encoded =
        BASE64_STANDARD.encode(serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string()));
    let (command, mut args) = resolve_protheus_ops_command(root, "attention-queue");
    args.push("enqueue".to_string());
    args.push(format!("--event-json-base64={encoded}"));
    args.push("--run-context=spine".to_string());

    let _ = Command::new(command)
        .args(args)
        .current_dir(root)
        .env(
            "PROTHEUS_NODE_BINARY",
            std::env::var("PROTHEUS_NODE_BINARY").unwrap_or_else(|_| "node".to_string()),
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

