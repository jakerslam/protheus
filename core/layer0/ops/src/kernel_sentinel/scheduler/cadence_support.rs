use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::kernel_sentinel::cli_args::{option_path, option_usize};

const DEFAULT_DREAM_IDLE_SECONDS: usize = 3600;
const DEFAULT_DREAM_MAX_WITHOUT_SECONDS: usize = 86_400;

fn activity_root(root: &Path, args: &[String]) -> PathBuf {
    option_path(args, "--activity-root", root.join("local/state/hands"))
}

fn dream_idle_seconds(args: &[String]) -> usize {
    option_usize(
        args,
        "--dream-idle-seconds",
        DEFAULT_DREAM_IDLE_SECONDS,
    )
}

fn dream_max_without_seconds(args: &[String]) -> usize {
    option_usize(
        args,
        "--dream-max-without-seconds",
        DEFAULT_DREAM_MAX_WITHOUT_SECONDS,
    )
}

fn latest_activity_epoch_seconds(root: &Path, args: &[String]) -> Option<u64> {
    let root = activity_root(root, args);
    if !root.exists() {
        return None;
    }
    let mut stack = vec![root];
    let mut latest = None;
    while let Some(path) = stack.pop() {
        let metadata = fs::metadata(&path).ok()?;
        if metadata.is_dir() {
            for entry in fs::read_dir(&path).ok()? {
                let entry = entry.ok()?;
                stack.push(entry.path());
            }
            continue;
        }
        let modified = metadata.modified().ok()?;
        let epoch = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
        latest = Some(
            latest
                .map(|current: u64| current.max(epoch))
                .unwrap_or(epoch),
        );
    }
    latest
}

pub(super) fn build_dream_gate(
    root: &Path,
    args: &[String],
    now: u64,
    last_dream_success: Option<u64>,
) -> Value {
    let latest_activity = latest_activity_epoch_seconds(root, args);
    let idle_threshold = dream_idle_seconds(args) as u64;
    let max_without = dream_max_without_seconds(args) as u64;
    let inactivity_seconds = latest_activity.map(|last| now.saturating_sub(last));
    let since_last_dream_seconds = last_dream_success.map(|last| now.saturating_sub(last));
    let idle_due = inactivity_seconds
        .map(|elapsed| elapsed >= idle_threshold)
        .unwrap_or(false);
    let max_without_due = since_last_dream_seconds
        .map(|elapsed| elapsed >= max_without)
        .unwrap_or(last_dream_success.is_none());
    let due = idle_due || max_without_due;
    let reason = if idle_due {
        "inactive"
    } else if max_without_due && last_dream_success.is_none() {
        "never_dreamed"
    } else if max_without_due {
        "max_without_dream_elapsed"
    } else {
        "activity_recent"
    };
    json!({
        "due": due,
        "reason": reason,
        "activity_root": activity_root(root, args),
        "latest_activity_epoch_secs": latest_activity,
        "inactivity_seconds": inactivity_seconds,
        "idle_threshold_seconds": idle_threshold,
        "idle_due": idle_due,
        "last_dream_success_epoch_secs": last_dream_success,
        "since_last_dream_seconds": since_last_dream_seconds,
        "max_without_seconds": max_without,
        "max_without_due": max_without_due,
        "maintenance_window": "dream"
    })
}
