// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::cli_args::{bool_flag, option_path, option_usize, state_dir_from_args};
use super::{auto_run, write_json};
mod cadence_support;
use cadence_support::build_dream_gate;

const DEFAULT_SCHEDULE_ARTIFACT: &str =
    "core/local/artifacts/kernel_sentinel_schedule_current.json";
const DEFAULT_TICK_ARTIFACT: &str = "core/local/artifacts/kernel_sentinel_tick_current.json";
const DEFAULT_HEARTBEAT_ARTIFACT: &str =
    "core/local/artifacts/kernel_sentinel_heartbeat_current.json";
const DEFAULT_DREAM_ARTIFACT: &str = "core/local/artifacts/kernel_sentinel_dream_current.json";
const DEFAULT_TICK_INTERVAL_SECONDS: usize = 10;
const DEFAULT_HEARTBEAT_INTERVAL_SECONDS: usize = 300;
const DEFAULT_DREAM_INTERVAL_SECONDS: usize = 86_400;
const DEFAULT_STALE_WINDOW_SECONDS: usize = 5400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchedulerMode {
    Tick,
    Heartbeat,
    Dream,
}

impl SchedulerMode {
    fn mode_name(self) -> &'static str {
        match self {
            SchedulerMode::Tick => "tick",
            SchedulerMode::Heartbeat => "heartbeat",
            SchedulerMode::Dream => "dream",
        }
    }

    fn artifact_type(self) -> &'static str {
        match self {
            SchedulerMode::Tick => "kernel_sentinel_tick_run",
            SchedulerMode::Heartbeat => "kernel_sentinel_heartbeat_run",
            SchedulerMode::Dream => "kernel_sentinel_dream_run",
        }
    }

    fn default_cadence(self) -> &'static str {
        match self {
            SchedulerMode::Tick => "tick",
            SchedulerMode::Heartbeat => "heartbeat",
            SchedulerMode::Dream => "dream",
        }
    }

    fn default_artifact(self) -> &'static str {
        match self {
            SchedulerMode::Tick => DEFAULT_TICK_ARTIFACT,
            SchedulerMode::Heartbeat => DEFAULT_HEARTBEAT_ARTIFACT,
            SchedulerMode::Dream => DEFAULT_DREAM_ARTIFACT,
        }
    }

    fn default_interval(self) -> usize {
        match self {
            SchedulerMode::Tick => DEFAULT_TICK_INTERVAL_SECONDS,
            SchedulerMode::Heartbeat => DEFAULT_HEARTBEAT_INTERVAL_SECONDS,
            SchedulerMode::Dream => DEFAULT_DREAM_INTERVAL_SECONDS,
        }
    }

    fn cascade_target(self) -> Option<Self> {
        match self {
            SchedulerMode::Tick => Some(SchedulerMode::Heartbeat),
            SchedulerMode::Heartbeat => Some(SchedulerMode::Dream),
            SchedulerMode::Dream => None,
        }
    }
}

fn has_option(args: &[String], name: &str) -> bool {
    args.iter()
        .any(|arg| arg == name || arg.starts_with(&format!("{name}=")))
}

fn option_string(args: &[String], name: &str, fallback: &str) -> String {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).map(str::to_string))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn option_bool_default(args: &[String], name: &str, default: bool) -> bool {
    if has_option(args, name) {
        bool_flag(args, name)
    } else {
        default
    }
}

fn option_string_if_present(args: &[String], name: &str) -> Option<String> {
    let value = option_string(args, name, "");
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn legacy_state_path(dir: &Path) -> PathBuf {
    dir.join("kernel_sentinel_schedule_state.json")
}

fn state_path(dir: &Path, mode: SchedulerMode) -> PathBuf {
    match mode {
        SchedulerMode::Tick => dir.join("kernel_sentinel_tick_state.json"),
        SchedulerMode::Heartbeat => dir.join("kernel_sentinel_heartbeat_state.json"),
        SchedulerMode::Dream => legacy_state_path(dir),
    }
}

fn read_scheduler_state(path: &Path) -> Option<Value> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn read_mode_state(dir: &Path, mode: SchedulerMode) -> Option<(Value, bool)> {
    let mode_path = state_path(dir, mode);
    if let Some(value) = read_scheduler_state(&mode_path) {
        return Some((value, false));
    }
    if matches!(mode, SchedulerMode::Heartbeat) {
        let legacy_path = legacy_state_path(dir);
        return read_scheduler_state(&legacy_path).map(|value| (value, true));
    }
    None
}

fn read_last_success_for_mode(dir: &Path, mode: SchedulerMode) -> Option<u64> {
    read_mode_state(dir, mode).and_then(|(value, _)| value["last_success_epoch_secs"].as_u64())
}

fn artifact_path(root: &Path, args: &[String], mode: SchedulerMode) -> PathBuf {
    option_path(args, "--schedule-artifact", root.join(mode.default_artifact()))
}

fn scheduler_args(args: &[String], mode: SchedulerMode) -> Vec<String> {
    let mut out = args.to_vec();
    if !has_option(&out, "--cadence") {
        out.push(format!("--cadence={}", mode.default_cadence()));
    }
    if !has_option(&out, "--quiet-success") {
        out.push("--quiet-success=1".to_string());
    }
    out
}

fn subordinate_args(args: &[String]) -> Vec<String> {
    args.iter()
        .filter(|arg| {
            arg.as_str() != "--force"
                && !arg.starts_with("--force=")
                && arg.as_str() != "--interval-seconds"
                && !arg.starts_with("--interval-seconds=")
                && arg.as_str() != "--schedule-artifact"
                && !arg.starts_with("--schedule-artifact=")
                && arg.as_str() != "--cadence"
                && !arg.starts_with("--cadence=")
                && arg.as_str() != "--command-alias"
                && !arg.starts_with("--command-alias=")
        })
        .cloned()
        .collect()
}

fn is_due(now: u64, last_success: Option<u64>, interval_seconds: usize, force: bool) -> bool {
    force
        || last_success
            .map(|last| now.saturating_sub(last) >= interval_seconds as u64)
            .unwrap_or(true)
}

#[allow(clippy::too_many_arguments)]
fn build_scheduler_artifact(
    root: &Path,
    args: &[String],
    mode: SchedulerMode,
    now: u64,
    last_success_before: Option<u64>,
    last_success_after: Option<u64>,
    due: bool,
    recorded_exit: Option<i32>,
    stale: bool,
    cascade_due: Option<bool>,
    cascade_invoked: bool,
    cascade_exit: Option<i32>,
    dream_gate: Option<Value>,
) -> Value {
    let dir = state_dir_from_args(root, args);
    let mode_state_path = state_path(&dir, mode);
    let interval_seconds = option_usize(args, "--interval-seconds", mode.default_interval());
    let stale_window_seconds =
        option_usize(args, "--stale-window-seconds", DEFAULT_STALE_WINDOW_SECONDS);
    let command_mode = option_string_if_present(args, "--command-alias")
        .unwrap_or_else(|| mode.mode_name().to_string());
    let cadence = option_string(args, "--cadence", &command_mode);
    let auto_run_invoked = matches!(mode, SchedulerMode::Dream) && due;
    let next_due_epoch_secs = last_success_after
        .map(|last| last.saturating_add(interval_seconds as u64))
        .unwrap_or(now);
    let stale_age_seconds = last_success_after.map(|last| now.saturating_sub(last));
    let exit_code = recorded_exit.unwrap_or(0);
    let cascade_target = mode.cascade_target().map(|next| next.mode_name().to_string());
    let mut artifact = json!({
        "ok": !stale && exit_code == 0,
        "type": if command_mode == "schedule" {
            "kernel_sentinel_schedule_run"
        } else {
            mode.artifact_type()
        },
        "canonical_name": super::KERNEL_SENTINEL_NAME,
        "module_id": super::KERNEL_SENTINEL_MODULE_ID,
        "generated_at": crate::now_iso(),
        "automatic": true,
        "scheduler": true,
        "mode": command_mode,
        "coordinator_mode": mode.mode_name(),
        "tick": matches!(mode, SchedulerMode::Tick),
        "heartbeat": matches!(mode, SchedulerMode::Heartbeat),
        "dream": matches!(mode, SchedulerMode::Dream),
        "cadence": cadence,
        "interval_seconds": interval_seconds,
        "stale_window_seconds": stale_window_seconds,
        "now_epoch_secs": now,
        "last_success_epoch_secs_before": last_success_before,
        "last_success_epoch_secs": last_success_after,
        "stale_age_seconds": stale_age_seconds,
        "stale": stale,
        "due": due,
        "skipped": !due,
        "auto_run_invoked": auto_run_invoked,
        "scheduler_exit_code": exit_code,
        "state_dir": dir,
        "schedule_state_path": mode_state_path,
        "next_due_epoch_secs": next_due_epoch_secs,
        "stale_escalation": {
            "operator_visible": stale,
            "strict_exit_code": if stale { 2 } else { 0 },
            "reason": if stale { "kernel_sentinel_auto_run_stale" } else { "fresh" }
        },
        "self_study_contract": {
            "automatic_self_understanding": true,
            "human_manual_invocation_required": false,
            "stale_sentinel_blocks_when_strict": true,
            "heavy_maintenance_requires_dream": true
        },
        "cascade": {
            "target": cascade_target,
            "due": cascade_due,
            "invoked": cascade_invoked,
            "exit_code": cascade_exit
        },
        "maintenance_window": {
            "heavy_maintenance_window": "dream",
            "heavy_maintenance_allowed": matches!(mode, SchedulerMode::Dream),
            "dream_gate_checked": dream_gate.is_some()
        }
    });
    if let Some(gate) = dream_gate {
        artifact["dream_gate"] = gate;
    }
    artifact["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&artifact));
    artifact
}

fn persist_schedule_state(
    dir: &Path,
    mode: SchedulerMode,
    args: &[String],
    now: u64,
    last_success: Option<u64>,
    recorded_exit: Option<i32>,
    stale: bool,
) -> Result<(), String> {
    let cadence = option_string(args, "--cadence", mode.default_cadence());
    let state = json!({
        "type": "kernel_sentinel_schedule_state",
        "canonical_name": super::KERNEL_SENTINEL_NAME,
        "generated_at": crate::now_iso(),
        "cadence": cadence,
        "mode": mode.mode_name(),
        "last_attempt_epoch_secs": now,
        "last_success_epoch_secs": last_success,
        "last_exit_code": recorded_exit,
        "stale": stale
    });
    write_json(&state_path(dir, mode), &state)
}

fn build_mode_health(root: &Path, args: &[String], mode: SchedulerMode) -> Value {
    let dir = state_dir_from_args(root, args);
    let path = state_path(&dir, mode);
    let state_with_legacy = read_mode_state(&dir, mode);
    let legacy_fallback_used = state_with_legacy
        .as_ref()
        .map(|(_, used)| *used)
        .unwrap_or(false);
    let state = state_with_legacy.map(|(value, _)| value);
    let now = now_epoch_seconds();
    let interval_seconds = option_usize(args, "--interval-seconds", mode.default_interval());
    let stale_window_seconds =
        option_usize(args, "--stale-window-seconds", DEFAULT_STALE_WINDOW_SECONDS);
    let cadence = option_string(args, "--cadence", mode.default_cadence());
    let last_attempt_epoch_secs = state
        .as_ref()
        .and_then(|value| value["last_attempt_epoch_secs"].as_u64());
    let last_success_epoch_secs = state
        .as_ref()
        .and_then(|value| value["last_success_epoch_secs"].as_u64());
    let last_exit_code = state
        .as_ref()
        .and_then(|value| value["last_exit_code"].as_i64());
    let next_due_epoch_secs =
        last_success_epoch_secs.map(|last| last.saturating_add(interval_seconds as u64));
    let stale_age_seconds = last_success_epoch_secs.map(|last| now.saturating_sub(last));
    let due = last_success_epoch_secs
        .map(|last| now.saturating_sub(last) >= interval_seconds as u64)
        .unwrap_or(true);
    let configured = state.is_some();
    let stale = configured
        && stale_age_seconds
            .map(|age| age > stale_window_seconds as u64)
            .unwrap_or(false);
    let fresh = configured
        && !stale
        && last_success_epoch_secs.is_some()
        && last_exit_code.unwrap_or(0) == 0;
    let degraded = configured && !stale && last_exit_code.unwrap_or(0) != 0;
    let lifecycle_status = if !configured {
        "unconfigured"
    } else if stale {
        "stale"
    } else if degraded {
        "degraded"
    } else if last_success_epoch_secs.is_some() {
        "fresh"
    } else if last_attempt_epoch_secs.is_some() {
        "running"
    } else {
        "configured"
    };
    let mut summary = json!({
        "mode": mode.mode_name(),
        "cadence": cadence,
        "configured": configured,
        "fresh": fresh,
        "stale": stale,
        "degraded": degraded,
        "due": due,
        "status": lifecycle_status,
        "lifecycle_status": lifecycle_status,
        "interval_seconds": interval_seconds,
        "stale_window_seconds": stale_window_seconds,
        "last_attempt_epoch_secs": last_attempt_epoch_secs,
        "last_success_epoch_secs": last_success_epoch_secs,
        "last_exit_code": last_exit_code,
        "next_due_epoch_secs": next_due_epoch_secs,
        "stale_age_seconds": stale_age_seconds,
        "state_path": path,
        "legacy_fallback_used": legacy_fallback_used,
        "legacy_state_path": legacy_state_path(&dir)
    });
    if matches!(mode, SchedulerMode::Dream) {
        summary["dream_gate"] = build_dream_gate(root, args, now, last_success_epoch_secs);
    }
    summary
}

pub fn build_scheduler_health_summary(root: &Path, args: &[String]) -> Value {
    let tick = build_mode_health(root, args, SchedulerMode::Tick);
    let heartbeat = build_mode_health(root, args, SchedulerMode::Heartbeat);
    let dream = build_mode_health(root, args, SchedulerMode::Dream);
    let modes = [&tick, &heartbeat, &dream];
    let configured = modes
        .iter()
        .any(|mode| mode["configured"].as_bool().unwrap_or(false));
    let stale = modes
        .iter()
        .any(|mode| mode["stale"].as_bool().unwrap_or(false));
    let degraded = modes
        .iter()
        .any(|mode| mode["degraded"].as_bool().unwrap_or(false));
    let running = modes
        .iter()
        .any(|mode| mode["lifecycle_status"] == "running");
    let fresh = configured
        && modes.iter().all(|mode| {
            !mode["configured"].as_bool().unwrap_or(false) || mode["fresh"].as_bool().unwrap_or(false)
        });
    let lifecycle_status = if !configured {
        "unconfigured"
    } else if stale {
        "stale"
    } else if degraded {
        "degraded"
    } else if running {
        "running"
    } else if fresh {
        "fresh"
    } else {
        "configured"
    };
    json!({
        "configured": configured,
        "fresh": fresh,
        "stale": stale,
        "degraded": degraded,
        "running": running,
        "status": lifecycle_status,
        "lifecycle_status": lifecycle_status,
        "dream_maintenance_only": true,
        "shared_state_path": false,
        "tick": tick,
        "heartbeat": heartbeat,
        "dream": dream,
        "cadence_hierarchy": {
            "tick": "light_watchdog",
            "heartbeat": "supervisory_health",
            "dream": "heavy_maintenance_and_self_study"
        }
    })
}

fn run_scheduler(root: &Path, raw_args: &[String], mode: SchedulerMode) -> i32 {
    let effective = scheduler_args(raw_args, mode);
    let now = now_epoch_seconds();
    let dir = state_dir_from_args(root, &effective);
    let previous_success = read_last_success_for_mode(&dir, mode);
    let interval_seconds = option_usize(&effective, "--interval-seconds", mode.default_interval());
    let stale_window_seconds = option_usize(
        &effective,
        "--stale-window-seconds",
        DEFAULT_STALE_WINDOW_SECONDS,
    );
    let force = bool_flag(&effective, "--force");
    let strict = bool_flag(&effective, "--strict");
    let due_by_interval = is_due(now, previous_success, interval_seconds, force);
    let mut dream_gate = None;
    let due = match mode {
        SchedulerMode::Dream => {
            let gate = build_dream_gate(root, &effective, now, previous_success);
            let gate_due = gate["due"].as_bool().unwrap_or(false);
            dream_gate = Some(gate);
            force || gate_due
        }
        _ => due_by_interval,
    };

    let mut cascade_due = None;
    let mut cascade_invoked = false;
    let mut cascade_exit = None;
    if due {
        match mode {
            SchedulerMode::Tick => {
                let heartbeat_previous = read_last_success_for_mode(&dir, SchedulerMode::Heartbeat);
                let heartbeat_interval = option_usize(
                    &effective,
                    "--heartbeat-interval-seconds",
                    SchedulerMode::Heartbeat.default_interval(),
                );
                let heartbeat_due = is_due(now, heartbeat_previous, heartbeat_interval, false);
                cascade_due = Some(heartbeat_due);
                if heartbeat_due && option_bool_default(&effective, "--cascade-heartbeat", true) {
                    cascade_invoked = true;
                    let mut nested = subordinate_args(&effective);
                    nested.push(format!("--interval-seconds={heartbeat_interval}"));
                    cascade_exit = Some(run_scheduler(root, &nested, SchedulerMode::Heartbeat));
                }
            }
            SchedulerMode::Heartbeat => {
                let dream_previous = read_last_success_for_mode(&dir, SchedulerMode::Dream);
                let gate = build_dream_gate(root, &effective, now, dream_previous);
                let dream_due = gate["due"].as_bool().unwrap_or(false);
                cascade_due = Some(dream_due);
                dream_gate = Some(gate);
                if dream_due && option_bool_default(&effective, "--cascade-dream", true) {
                    cascade_invoked = true;
                    let nested = subordinate_args(&effective);
                    cascade_exit = Some(run_scheduler(root, &nested, SchedulerMode::Dream));
                }
            }
            SchedulerMode::Dream => {}
        }
    }

    let auto_exit = if matches!(mode, SchedulerMode::Dream) && due {
        Some(auto_run::run_auto(root, &effective))
    } else {
        None
    };
    let recorded_exit = if due {
        Some(auto_exit.or(cascade_exit).unwrap_or(0))
    } else {
        None
    };
    let last_success_after = match mode {
        SchedulerMode::Dream => {
            if due && recorded_exit == Some(0) {
                Some(now)
            } else {
                previous_success
            }
        }
        _ => {
            if due {
                Some(now)
            } else {
                previous_success
            }
        }
    };
    let stale = last_success_after
        .map(|last| now.saturating_sub(last) > stale_window_seconds as u64)
        .unwrap_or(false);
    if let Err(err) = persist_schedule_state(
        &dir,
        mode,
        &effective,
        now,
        last_success_after,
        recorded_exit,
        stale,
    ) {
        eprintln!("kernel_sentinel_schedule_state_write_failed: {err}");
        return 1;
    }
    let artifact = build_scheduler_artifact(
        root,
        &effective,
        mode,
        now,
        previous_success,
        last_success_after,
        due,
        recorded_exit,
        stale,
        cascade_due,
        cascade_invoked,
        cascade_exit,
        dream_gate,
    );
    let artifact_path = artifact_path(root, &effective, mode);
    if let Err(err) = write_json(&artifact_path, &artifact) {
        eprintln!("kernel_sentinel_schedule_artifact_write_failed: {err}");
        return 1;
    }
    let exit = if recorded_exit.unwrap_or(0) != 0 {
        recorded_exit.unwrap_or(1)
    } else if strict && stale {
        2
    } else {
        0
    };
    if !(bool_flag(&effective, "--quiet-success") && exit == 0) {
        println!(
            "{}",
            serde_json::to_string_pretty(&artifact)
                .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
        );
    }
    exit
}

pub fn run_tick(root: &Path, args: &[String]) -> i32 {
    run_scheduler(root, args, SchedulerMode::Tick)
}

pub fn run_schedule(root: &Path, args: &[String]) -> i32 {
    let mut aliased = args.to_vec();
    if !has_option(&aliased, "--command-alias") {
        aliased.push("--command-alias=schedule".to_string());
    }
    if !has_option(&aliased, "--schedule-artifact") {
        aliased.push(format!(
            "--schedule-artifact={}",
            root.join(DEFAULT_SCHEDULE_ARTIFACT).display()
        ));
    }
    run_scheduler(root, &aliased, SchedulerMode::Tick)
}

pub fn run_heartbeat(root: &Path, args: &[String]) -> i32 {
    run_scheduler(root, args, SchedulerMode::Heartbeat)
}

pub fn run_dream(root: &Path, args: &[String]) -> i32 {
    run_scheduler(root, args, SchedulerMode::Dream)
}

#[cfg(test)]
mod tests;
