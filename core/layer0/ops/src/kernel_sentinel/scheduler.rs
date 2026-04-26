// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::cli_args::{bool_flag, option_path, option_usize, state_dir_from_args};
use super::{auto_run, write_json};

const DEFAULT_SCHEDULE_ARTIFACT: &str = "core/local/artifacts/kernel_sentinel_schedule_current.json";
const DEFAULT_HEARTBEAT_ARTIFACT: &str = "core/local/artifacts/kernel_sentinel_heartbeat_current.json";
const DEFAULT_SCHEDULE_INTERVAL_SECONDS: usize = 900;
const DEFAULT_HEARTBEAT_INTERVAL_SECONDS: usize = 1800;
const DEFAULT_STALE_WINDOW_SECONDS: usize = 5400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchedulerMode {
    Schedule,
    Heartbeat,
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

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn state_path(dir: &Path) -> PathBuf {
    dir.join("kernel_sentinel_schedule_state.json")
}

fn read_last_success(path: &Path) -> Option<u64> {
    let value: Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    value["last_success_epoch_secs"].as_u64()
}

fn default_interval(mode: SchedulerMode) -> usize {
    match mode {
        SchedulerMode::Schedule => DEFAULT_SCHEDULE_INTERVAL_SECONDS,
        SchedulerMode::Heartbeat => DEFAULT_HEARTBEAT_INTERVAL_SECONDS,
    }
}

fn default_artifact(root: &Path, mode: SchedulerMode) -> PathBuf {
    match mode {
        SchedulerMode::Schedule => root.join(DEFAULT_SCHEDULE_ARTIFACT),
        SchedulerMode::Heartbeat => root.join(DEFAULT_HEARTBEAT_ARTIFACT),
    }
}

fn schedule_artifact_path(root: &Path, args: &[String], mode: SchedulerMode) -> PathBuf {
    option_path(args, "--schedule-artifact", default_artifact(root, mode))
}

fn scheduler_args(args: &[String], mode: SchedulerMode) -> Vec<String> {
    let mut out = args.to_vec();
    if !has_option(&out, "--cadence") {
        let cadence = match mode {
            SchedulerMode::Schedule => "maintenance",
            SchedulerMode::Heartbeat => "heartbeat",
        };
        out.push(format!("--cadence={cadence}"));
    }
    if !has_option(&out, "--quiet-success") {
        out.push("--quiet-success=1".to_string());
    }
    out
}

fn build_scheduler_artifact(
    root: &Path,
    args: &[String],
    mode: SchedulerMode,
    now: u64,
    last_success_before: Option<u64>,
    last_success_after: Option<u64>,
    due: bool,
    auto_exit: Option<i32>,
    stale: bool,
) -> Value {
    let dir = state_dir_from_args(root, args);
    let interval_seconds = option_usize(args, "--interval-seconds", default_interval(mode));
    let stale_window_seconds =
        option_usize(args, "--stale-window-seconds", DEFAULT_STALE_WINDOW_SECONDS);
    let cadence = option_string(
        args,
        "--cadence",
        match mode {
            SchedulerMode::Schedule => "maintenance",
            SchedulerMode::Heartbeat => "heartbeat",
        },
    );
    let auto_run_invoked = auto_exit.is_some();
    let next_due_epoch_secs = last_success_after
        .map(|last| last.saturating_add(interval_seconds as u64))
        .unwrap_or(now);
    let stale_age_seconds = last_success_after.map(|last| now.saturating_sub(last));
    let exit_code = auto_exit.unwrap_or(0);
    let mut artifact = json!({
        "ok": !stale && exit_code == 0,
        "type": match mode {
            SchedulerMode::Schedule => "kernel_sentinel_schedule_run",
            SchedulerMode::Heartbeat => "kernel_sentinel_heartbeat_run",
        },
        "canonical_name": super::KERNEL_SENTINEL_NAME,
        "module_id": super::KERNEL_SENTINEL_MODULE_ID,
        "generated_at": crate::now_iso(),
        "automatic": true,
        "scheduler": true,
        "heartbeat": mode == SchedulerMode::Heartbeat,
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
        "auto_exit_code": auto_exit,
        "scheduler_exit_code": exit_code,
        "state_dir": dir,
        "schedule_state_path": state_path(&state_dir_from_args(root, args)),
        "next_due_epoch_secs": next_due_epoch_secs,
        "stale_escalation": {
            "operator_visible": stale,
            "strict_exit_code": if stale { 2 } else { 0 },
            "reason": if stale { "kernel_sentinel_auto_run_stale" } else { "fresh" }
        },
        "self_study_contract": {
            "automatic_self_understanding": true,
            "human_manual_invocation_required": false,
            "stale_sentinel_blocks_when_strict": true
        }
    });
    artifact["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&artifact));
    artifact
}

fn persist_schedule_state(
    dir: &Path,
    args: &[String],
    now: u64,
    last_success: Option<u64>,
    auto_exit: Option<i32>,
    stale: bool,
) -> Result<(), String> {
    let cadence = option_string(args, "--cadence", "maintenance");
    let state = json!({
        "type": "kernel_sentinel_schedule_state",
        "canonical_name": super::KERNEL_SENTINEL_NAME,
        "generated_at": crate::now_iso(),
        "cadence": cadence,
        "last_attempt_epoch_secs": now,
        "last_success_epoch_secs": last_success,
        "last_exit_code": auto_exit,
        "stale": stale
    });
    write_json(&state_path(dir), &state)
}

fn run_scheduler(root: &Path, raw_args: &[String], mode: SchedulerMode) -> i32 {
    let effective = scheduler_args(raw_args, mode);
    let now = now_epoch_seconds();
    let dir = state_dir_from_args(root, &effective);
    let previous_success = read_last_success(&state_path(&dir));
    let interval_seconds = option_usize(&effective, "--interval-seconds", default_interval(mode));
    let stale_window_seconds =
        option_usize(&effective, "--stale-window-seconds", DEFAULT_STALE_WINDOW_SECONDS);
    let force = bool_flag(&effective, "--force");
    let strict = bool_flag(&effective, "--strict");
    let due = force
        || previous_success
            .map(|last| now.saturating_sub(last) >= interval_seconds as u64)
            .unwrap_or(true);
    let auto_exit = if due {
        Some(auto_run::run_auto(root, &effective))
    } else {
        None
    };
    let last_success_after = if auto_exit == Some(0) {
        Some(now)
    } else {
        previous_success
    };
    let stale = last_success_after
        .map(|last| now.saturating_sub(last) > stale_window_seconds as u64)
        .unwrap_or(true);
    if let Err(err) =
        persist_schedule_state(&dir, &effective, now, last_success_after, auto_exit, stale)
    {
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
        auto_exit,
        stale,
    );
    let artifact_path = schedule_artifact_path(root, &effective, mode);
    if let Err(err) = write_json(&artifact_path, &artifact) {
        eprintln!("kernel_sentinel_schedule_artifact_write_failed: {err}");
        return 1;
    }
    let exit = if auto_exit.unwrap_or(0) != 0 {
        auto_exit.unwrap_or(1)
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

pub fn run_schedule(root: &Path, args: &[String]) -> i32 {
    run_scheduler(root, args, SchedulerMode::Schedule)
}

pub fn run_heartbeat(root: &Path, args: &[String]) -> i32 {
    run_scheduler(root, args, SchedulerMode::Heartbeat)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "kernel-sentinel-scheduler-{name}-{}",
            crate::deterministic_receipt_hash(&json!({
                "test": name,
                "nonce": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            }))
        ))
    }

    #[test]
    fn schedule_invokes_auto_when_due_and_records_success() {
        let root = unique_root("due");
        let out = root.join("schedule.json");
        let auto = root.join("auto.json");
        let args = vec![
            "--strict=0".to_string(),
            "--force=1".to_string(),
            "--cadence=maintenance".to_string(),
            format!("--schedule-artifact={}", out.display()),
            format!("--auto-artifact={}", auto.display()),
        ];
        let exit = run_schedule(&root, &args);
        assert_eq!(exit, 0);
        let artifact: Value = serde_json::from_str(&fs::read_to_string(out).unwrap()).unwrap();
        assert_eq!(artifact["type"], "kernel_sentinel_schedule_run");
        assert_eq!(artifact["auto_run_invoked"], true);
        assert_eq!(artifact["stale"], false);
        assert!(state_path(&state_dir_from_args(&root, &args)).exists());
        assert!(auto.exists());
    }

    #[test]
    fn strict_schedule_fails_when_previous_success_is_stale() {
        let root = unique_root("stale");
        let state_dir = root.join("local/state/kernel_sentinel");
        fs::create_dir_all(&state_dir).unwrap();
        write_json(
            &state_path(&state_dir),
            &json!({
                "type": "kernel_sentinel_schedule_state",
                "last_success_epoch_secs": 1
            }),
        )
        .unwrap();
        let out = root.join("stale.json");
        let args = vec![
            "--strict=1".to_string(),
            "--interval-seconds=99999999999".to_string(),
            "--stale-window-seconds=1".to_string(),
            format!("--schedule-artifact={}", out.display()),
        ];
        let exit = run_schedule(&root, &args);
        assert_eq!(exit, 2);
        let artifact: Value = serde_json::from_str(&fs::read_to_string(out).unwrap()).unwrap();
        assert_eq!(artifact["stale"], true);
        assert_eq!(artifact["stale_escalation"]["reason"], "kernel_sentinel_auto_run_stale");
        assert_eq!(artifact["auto_run_invoked"], false);
    }
}
