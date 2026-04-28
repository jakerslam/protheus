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

fn legacy_state_path(dir: &Path) -> PathBuf {
    dir.join("kernel_sentinel_schedule_state.json")
}

fn state_path(dir: &Path, mode: SchedulerMode) -> PathBuf {
    match mode {
        SchedulerMode::Schedule => dir.join("kernel_sentinel_schedule_state.json"),
        SchedulerMode::Heartbeat => dir.join("kernel_sentinel_heartbeat_state.json"),
    }
}

fn read_last_success(path: &Path) -> Option<u64> {
    let value: Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    value["last_success_epoch_secs"].as_u64()
}

fn read_scheduler_state(path: &Path) -> Option<Value> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn read_mode_state(dir: &Path, mode: SchedulerMode) -> Option<(Value, bool)> {
    let mode_path = state_path(dir, mode);
    if let Some(value) = read_scheduler_state(&mode_path) {
        return Some((value, false));
    }
    let legacy_path = legacy_state_path(dir);
    if mode_path != legacy_path {
        read_scheduler_state(&legacy_path).map(|value| (value, true))
    } else {
        None
    }
}

fn read_last_success_for_mode(dir: &Path, mode: SchedulerMode) -> Option<u64> {
    read_mode_state(dir, mode).and_then(|(value, _)| value["last_success_epoch_secs"].as_u64())
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
    let mode_state_path = state_path(&dir, mode);
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
            "stale_sentinel_blocks_when_strict": true
        }
    });
    artifact["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&artifact));
    artifact
}

fn persist_schedule_state(
    dir: &Path,
    mode: SchedulerMode,
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
    let last_attempt_epoch_secs = state
        .as_ref()
        .and_then(|value| value["last_attempt_epoch_secs"].as_u64());
    let last_success_epoch_secs = state
        .as_ref()
        .and_then(|value| value["last_success_epoch_secs"].as_u64());
    let last_exit_code = state
        .as_ref()
        .and_then(|value| value["last_exit_code"].as_i64());
    let next_due_epoch_secs = last_success_epoch_secs
        .map(|last| last.saturating_add(interval_seconds as u64));
    let stale_age_seconds = last_success_epoch_secs.map(|last| now.saturating_sub(last));
    let due = last_success_epoch_secs
        .map(|last| now.saturating_sub(last) >= interval_seconds as u64)
        .unwrap_or(true);
    let configured = state.is_some();
    let stale = if !configured {
        true
    } else {
        stale_age_seconds
            .map(|age| age > stale_window_seconds as u64)
            .unwrap_or(false)
    };
    let fresh = configured && !stale && last_success_epoch_secs.is_some();
    let lifecycle_status = if !configured {
        "unconfigured"
    } else if stale {
        "stale"
    } else if last_success_epoch_secs.is_some() {
        "fresh"
    } else if last_attempt_epoch_secs.is_some() {
        "running"
    } else {
        "configured"
    };
    json!({
        "mode": match mode {
            SchedulerMode::Schedule => "schedule",
            SchedulerMode::Heartbeat => "heartbeat",
        },
        "cadence": cadence,
        "configured": configured,
        "fresh": fresh,
        "stale": stale,
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
    })
}

pub fn build_scheduler_health_summary(root: &Path, args: &[String]) -> Value {
    let schedule = build_mode_health(root, args, SchedulerMode::Schedule);
    let heartbeat = build_mode_health(root, args, SchedulerMode::Heartbeat);
    let fresh = schedule["fresh"].as_bool().unwrap_or(false)
        && heartbeat["fresh"].as_bool().unwrap_or(false);
    let stale = schedule["stale"].as_bool().unwrap_or(true)
        || heartbeat["stale"].as_bool().unwrap_or(true);
    let configured = schedule["configured"].as_bool().unwrap_or(false)
        || heartbeat["configured"].as_bool().unwrap_or(false);
    let running = schedule["lifecycle_status"] == "running" || heartbeat["lifecycle_status"] == "running";
    let lifecycle_status = if !configured {
        "unconfigured"
    } else if stale {
        "stale"
    } else if fresh {
        "fresh"
    } else if running {
        "running"
    } else {
        "configured"
    };
    json!({
        "configured": configured,
        "fresh": fresh,
        "stale": stale,
        "running": running,
        "status": lifecycle_status,
        "lifecycle_status": lifecycle_status,
        "shared_state_path": false,
        "schedule": schedule,
        "heartbeat": heartbeat
    })
}

fn run_scheduler(root: &Path, raw_args: &[String], mode: SchedulerMode) -> i32 {
    let effective = scheduler_args(raw_args, mode);
    let now = now_epoch_seconds();
    let dir = state_dir_from_args(root, &effective);
    let previous_success = read_last_success_for_mode(&dir, mode);
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
    if let Err(err) = persist_schedule_state(
        &dir,
        mode,
        &effective,
        now,
        last_success_after,
        auto_exit,
        stale,
    )
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
        let state_dir = state_dir_from_args(&root, &args);
        let state_path = state_path(&state_dir, SchedulerMode::Schedule);
        let state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
        assert_eq!(artifact["type"], "kernel_sentinel_schedule_run");
        assert_eq!(artifact["auto_run_invoked"], true);
        assert_eq!(artifact["cadence"], "maintenance");
        assert_eq!(artifact["stale"], false);
        assert_eq!(artifact["schedule_state_path"], state_path.display().to_string());
        assert_eq!(state["cadence"], "maintenance");
        assert_eq!(state["last_attempt_epoch_secs"], artifact["now_epoch_secs"]);
        assert_eq!(state["last_success_epoch_secs"], artifact["last_success_epoch_secs"]);
        assert_eq!(state["stale"], false);
        assert!(auto.exists());
    }

    #[test]
    fn heartbeat_invokes_auto_when_due_and_records_fresh_state() {
        let root = unique_root("heartbeat-due");
        let out = root.join("heartbeat.json");
        let auto = root.join("heartbeat-auto.json");
        let args = vec![
            "--strict=0".to_string(),
            "--force=1".to_string(),
            "--cadence=heartbeat".to_string(),
            format!("--schedule-artifact={}", out.display()),
            format!("--auto-artifact={}", auto.display()),
        ];
        let exit = run_heartbeat(&root, &args);
        assert_eq!(exit, 0);
        let artifact: Value = serde_json::from_str(&fs::read_to_string(out).unwrap()).unwrap();
        let state_dir = state_dir_from_args(&root, &args);
        let state_path = state_path(&state_dir, SchedulerMode::Heartbeat);
        let state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
        assert_eq!(artifact["type"], "kernel_sentinel_heartbeat_run");
        assert_eq!(artifact["heartbeat"], true);
        assert_eq!(artifact["auto_run_invoked"], true);
        assert_eq!(artifact["cadence"], "heartbeat");
        assert_eq!(artifact["schedule_state_path"], state_path.display().to_string());
        assert_eq!(state["cadence"], "heartbeat");
        assert_eq!(state["last_attempt_epoch_secs"], artifact["now_epoch_secs"]);
        assert_eq!(state["last_success_epoch_secs"], artifact["last_success_epoch_secs"]);
        assert_eq!(state["stale"], false);
        assert!(auto.exists());
    }

    #[test]
    fn strict_schedule_fails_when_previous_success_is_stale() {
        let root = unique_root("stale");
        let state_dir = root.join("local/state/kernel_sentinel");
        fs::create_dir_all(&state_dir).unwrap();
        write_json(
            &state_path(&state_dir, SchedulerMode::Schedule),
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

    #[test]
    fn health_summary_reports_unconfigured_when_no_state_exists() {
        let root = unique_root("health-unconfigured");
        let summary = build_scheduler_health_summary(&root, &[]);
        assert_eq!(summary["configured"], false);
        assert_eq!(summary["lifecycle_status"], "unconfigured");
        assert_eq!(summary["schedule"]["lifecycle_status"], "unconfigured");
        assert_eq!(summary["heartbeat"]["lifecycle_status"], "unconfigured");
    }

    #[test]
    fn health_summary_reports_stale_when_last_success_exceeds_window() {
        let root = unique_root("health-stale");
        let state_dir = root.join("local/state/kernel_sentinel");
        fs::create_dir_all(&state_dir).unwrap();
        write_json(
            &state_path(&state_dir, SchedulerMode::Schedule),
            &json!({
                "type": "kernel_sentinel_schedule_state",
                "last_attempt_epoch_secs": 2,
                "last_success_epoch_secs": 1,
                "last_exit_code": 0
            }),
        )
        .unwrap();
        let args = vec!["--stale-window-seconds=1".to_string()];
        let summary = build_scheduler_health_summary(&root, &args);
        assert_eq!(summary["configured"], true);
        assert_eq!(summary["stale"], true);
        assert_eq!(summary["lifecycle_status"], "stale");
        assert_eq!(summary["schedule"]["lifecycle_status"], "stale");
    }

    #[test]
    fn health_summary_tracks_schedule_and_heartbeat_independently() {
        let root = unique_root("health-independent");
        let state_dir = root.join("local/state/kernel_sentinel");
        fs::create_dir_all(&state_dir).unwrap();
        let now = now_epoch_seconds();
        write_json(
            &state_path(&state_dir, SchedulerMode::Schedule),
            &json!({
                "type": "kernel_sentinel_schedule_state",
                "last_attempt_epoch_secs": now,
                "last_success_epoch_secs": now,
                "last_exit_code": 0
            }),
        )
        .unwrap();
        write_json(
            &state_path(&state_dir, SchedulerMode::Heartbeat),
            &json!({
                "type": "kernel_sentinel_schedule_state",
                "last_attempt_epoch_secs": 2,
                "last_success_epoch_secs": 1,
                "last_exit_code": 0
            }),
        )
        .unwrap();
        let args = vec!["--stale-window-seconds=1".to_string()];
        let summary = build_scheduler_health_summary(&root, &args);
        assert_eq!(summary["shared_state_path"], false);
        assert_eq!(summary["schedule"]["lifecycle_status"], "fresh");
        assert_eq!(summary["heartbeat"]["lifecycle_status"], "stale");
        assert_eq!(summary["lifecycle_status"], "stale");
    }

    #[test]
    fn health_summary_uses_legacy_fallback_when_mode_state_missing() {
        let root = unique_root("health-legacy");
        let state_dir = root.join("local/state/kernel_sentinel");
        fs::create_dir_all(&state_dir).unwrap();
        write_json(
            &legacy_state_path(&state_dir),
            &json!({
                "type": "kernel_sentinel_schedule_state",
                "last_attempt_epoch_secs": 5,
                "last_success_epoch_secs": 5,
                "last_exit_code": 0
            }),
        )
        .unwrap();
        let summary = build_scheduler_health_summary(&root, &[]);
        assert_eq!(summary["schedule"]["legacy_fallback_used"], false);
        assert_eq!(summary["heartbeat"]["legacy_fallback_used"], true);
        assert_eq!(
            summary["heartbeat"]["legacy_state_path"],
            legacy_state_path(&state_dir).display().to_string()
        );
    }

    #[test]
    fn health_summary_reports_running_without_success_yet() {
        let root = unique_root("health-running");
        let state_dir = root.join("local/state/kernel_sentinel");
        fs::create_dir_all(&state_dir).unwrap();
        write_json(
            &state_path(&state_dir, SchedulerMode::Schedule),
            &json!({
                "type": "kernel_sentinel_schedule_state",
                "last_attempt_epoch_secs": now_epoch_seconds(),
                "last_success_epoch_secs": null,
                "last_exit_code": null
            }),
        )
        .unwrap();
        let summary = build_scheduler_health_summary(&root, &[]);
        assert_eq!(summary["schedule"]["lifecycle_status"], "running");
        assert_eq!(summary["lifecycle_status"], "running");
        assert_eq!(summary["running"], true);
    }

    #[test]
    fn health_summary_reports_configured_without_attempt_history() {
        let root = unique_root("health-configured");
        let state_dir = root.join("local/state/kernel_sentinel");
        fs::create_dir_all(&state_dir).unwrap();
        write_json(
            &state_path(&state_dir, SchedulerMode::Schedule),
            &json!({
                "type": "kernel_sentinel_schedule_state",
                "last_attempt_epoch_secs": null,
                "last_success_epoch_secs": null,
                "last_exit_code": null
            }),
        )
        .unwrap();
        let summary = build_scheduler_health_summary(&root, &[]);
        assert_eq!(summary["schedule"]["lifecycle_status"], "configured");
        assert_eq!(summary["lifecycle_status"], "configured");
    }
}
