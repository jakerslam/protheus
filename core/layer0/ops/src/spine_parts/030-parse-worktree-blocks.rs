fn parse_worktree_blocks(raw: &str) -> Vec<(PathBuf, bool)> {
    let mut out = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut detached = false;
    for line in raw.lines().chain(std::iter::once("")) {
        let row = line.trim();
        if row.is_empty() {
            if let Some(path) = current_path.take() {
                out.push((path, detached));
            }
            detached = false;
            continue;
        }
        if let Some(path) = row.strip_prefix("worktree ") {
            if let Some(prev) = current_path.replace(PathBuf::from(path.trim())) {
                out.push((prev, detached));
            }
            detached = false;
            continue;
        }
        if row == "detached" {
            detached = true;
        }
    }
    out
}

fn execute_sleep_cleanup_with_mode(
    root: &Path,
    apply: bool,
    force: bool,
    origin: &str,
    mode: SleepCleanupMode,
) -> (i32, Value) {
    let mut policy = load_sleep_cleanup_policy(root);
    if mode == SleepCleanupMode::Purge {
        policy.pressure_min_age_hours = 0;
        policy.pressure_max_candidates = policy.pressure_max_candidates.max(200_000);
        policy.pressure_jsonl_cap_bytes = policy.pressure_jsonl_cap_bytes.min(128 * 1024);
        policy.pressure_log_cap_bytes = policy.pressure_log_cap_bytes.min(64 * 1024);
    }
    let now_ms = now_epoch_ms();
    let now = now_iso();
    let mut errors = Vec::<String>::new();

    let last_run_ms = read_json(&policy.state_path)
        .and_then(|v| v.get("last_run_ms").and_then(Value::as_i64))
        .unwrap_or(0);
    let elapsed_minutes = ((now_ms - last_run_ms).max(0)) / (1000 * 60);
    let (available_before_bytes, total_before_bytes, free_before_percent, disk_mount_point) =
        disk_free_snapshot(root).unwrap_or((0, 0, 0.0, "".to_string()));
    let hard_floor_breach = total_before_bytes > 0
        && free_before_percent <= policy.hard_free_floor_percent;

    if mode != SleepCleanupMode::Purge && !policy.enabled && !force && !hard_floor_breach {
        let payload = json!({
            "ok": true,
            "type": "spine_sleep_cleanup",
            "ts": now,
            "origin": origin,
            "applied": false,
            "executed": false,
            "skipped_reason": "disabled",
            "policy": {
                "enabled": policy.enabled,
                "min_interval_minutes": policy.min_interval_minutes
            }
        });
        return (0, payload);
    }

    if mode != SleepCleanupMode::Purge
        && !force
        && !hard_floor_breach
        && last_run_ms > 0
        && elapsed_minutes < policy.min_interval_minutes
    {
        let payload = json!({
            "ok": true,
            "type": "spine_sleep_cleanup",
            "ts": now,
            "origin": origin,
            "applied": false,
            "executed": false,
            "skipped_reason": "interval_not_elapsed",
            "elapsed_minutes": elapsed_minutes,
            "min_interval_minutes": policy.min_interval_minutes
        });
        return (0, payload);
    }

    let pressure_mode = if mode == SleepCleanupMode::Purge {
        true
    } else {
        total_before_bytes > 0
            && (free_before_percent <= policy.disk_free_floor_percent || hard_floor_breach)
    };
    let pressure_target_free_percent = if mode == SleepCleanupMode::Purge {
        99.0
    } else {
        policy
            .pressure_target_free_percent
            .max((policy.disk_free_floor_percent + 0.5).min(99.0))
    };
    let pressure_target_available_bytes = if pressure_mode {
        ((total_before_bytes as f64) * (pressure_target_free_percent / 100.0)) as u64
    } else {
        0
    };
    let pressure_reclaim_needed_bytes =
        pressure_target_available_bytes.saturating_sub(available_before_bytes);
    let mut pressure_candidates = if pressure_mode {
        collect_pressure_candidates(root, &policy, now_ms)
    } else {
        Vec::new()
    };
    let mut pressure_candidate_estimated_bytes = 0u64;
    if pressure_mode {
        if mode == SleepCleanupMode::Purge {
            pressure_candidate_estimated_bytes = pressure_candidates
                .iter()
                .map(pressure_candidate_estimated_reclaim)
                .fold(0u64, |acc, value| acc.saturating_add(value));
        } else {
            let mut selected = Vec::<PressureCandidate>::new();
            for candidate in pressure_candidates {
                let estimated = pressure_candidate_estimated_reclaim(&candidate);
                if estimated == 0 {
                    continue;
                }
                pressure_candidate_estimated_bytes =
                    pressure_candidate_estimated_bytes.saturating_add(estimated);
                selected.push(candidate);
                if pressure_candidate_estimated_bytes >= pressure_reclaim_needed_bytes {
                    break;
                }
            }
            pressure_candidates = selected;
        }
    }

    let mut archive_entries: Vec<(i64, PathBuf)> = Vec::new();
    if let Ok(entries) = fs::read_dir(&policy.archive_root) {
        for entry in entries.flatten() {
            let p = entry.path();
            let mt = path_mtime_ms(&p).unwrap_or(0);
            archive_entries.push((mt, p));
        }
    }
    archive_entries.sort_by(|a, b| b.0.cmp(&a.0));

    let mut archive_candidates = Vec::<PathBuf>::new();
    let mut archive_candidate_bytes = 0u64;
    for (idx, (mtime_ms, path)) in archive_entries.iter().enumerate() {
        if mode != SleepCleanupMode::Purge {
            if idx < policy.archive_keep_latest {
                continue;
            }
            if age_hours(now_ms, *mtime_ms) < policy.archive_max_age_hours {
                continue;
            }
        }
        archive_candidate_bytes = archive_candidate_bytes.saturating_add(path_size_bytes(path));
        archive_candidates.push(path.clone());
    }

    let mut target_candidate = false;
    let mut target_candidate_bytes = 0u64;
    let mut target_candidate_reason = "none".to_string();
    if policy.target_root.exists() {
        if mode == SleepCleanupMode::Purge {
            target_candidate = true;
            target_candidate_bytes = path_size_bytes(&policy.target_root);
            target_candidate_reason = "purge_mode".to_string();
        } else if hard_floor_breach {
            target_candidate = true;
            target_candidate_bytes = path_size_bytes(&policy.target_root);
            target_candidate_reason = "hard_floor_breach".to_string();
        } else if let Some(mtime_ms) = path_mtime_ms(&policy.target_root) {
            if age_hours(now_ms, mtime_ms) >= policy.target_max_age_hours {
                target_candidate = true;
                target_candidate_bytes = path_size_bytes(&policy.target_root);
                target_candidate_reason = "target_age_exceeded".to_string();
            }
        }
        if pressure_mode
            && !target_candidate
            && pressure_reclaim_needed_bytes > 0
            && policy.target_root.exists()
        {
            let non_target_reclaim_estimate =
                pressure_candidate_estimated_bytes.saturating_add(archive_candidate_bytes);
            if non_target_reclaim_estimate < pressure_reclaim_needed_bytes {
                target_candidate = true;
                target_candidate_bytes = path_size_bytes(&policy.target_root);
                target_candidate_reason = "pressure_reclaim_deficit".to_string();
            }
        }
    }

    let mut detached_candidates = Vec::<PathBuf>::new();
    let mut worktree_parse_error: Option<String> = None;
    match Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(root)
        .output()
    {
        Ok(out) => {
            if out.status.success() {
                let raw = String::from_utf8_lossy(&out.stdout).to_string();
                let blocks = parse_worktree_blocks(&raw);
                for (path, detached) in blocks {
                    if !detached || path == root || !path.exists() {
                        continue;
                    }
                    let mtime_ms = path_mtime_ms(&path).unwrap_or(0);
                    if mode != SleepCleanupMode::Purge
                        && age_hours(now_ms, mtime_ms) < policy.detached_worktree_max_age_hours
                    {
                        continue;
                    }
                    detached_candidates.push(path);
                }
            } else {
                worktree_parse_error =
                    Some(String::from_utf8_lossy(&out.stderr).trim().to_string());
            }
        }
        Err(err) => {
            worktree_parse_error = Some(format!("git_worktree_list_failed:{err}"));
        }
    }
    if let Some(err) = worktree_parse_error {
        let has_git_root = root.join(".git").exists();
        if has_git_root && !err.trim().is_empty() {
            errors.push(err);
        }
    }

    let mut removed_archive = 0usize;
    let mut removed_archive_bytes = 0u64;
    let mut removed_target = false;
    let mut removed_target_bytes = 0u64;
    let mut removed_detached_worktrees = 0usize;
    let mut pressure_removed_files = 0usize;
    let mut pressure_reclaimed_bytes = 0u64;
    let mut pressure_applied = Vec::<Value>::new();

    let has_git_root = root.join(".git").exists();
    if apply {
        for candidate in &pressure_candidates {
            let result = match candidate.action {
                PressureAction::RemoveFile => {
                    let before = path_size_bytes(&candidate.path);
                    remove_path(&candidate.path).map(|_| before).map_err(|err| {
                        format!("pressure_remove_failed:{}:{err}", candidate.path.display())
                    })
                }
                PressureAction::TrimTail { max_bytes } => {
                    trim_file_to_tail(&candidate.path, max_bytes)
                }
            };
            match result {
                Ok(reclaimed) => {
                    if reclaimed > 0 {
                        pressure_removed_files += 1;
                        pressure_reclaimed_bytes =
                            pressure_reclaimed_bytes.saturating_add(reclaimed);
                    }
                    pressure_applied.push(json!({
                        "path": candidate.path.display().to_string(),
                        "action": pressure_action_label(candidate.action),
                        "last_touch_ms": candidate.last_touch_ms,
                        "reclaimed_bytes": reclaimed
                    }));
                }
                Err(err) => errors.push(err),
            }
        }

        for path in &archive_candidates {
            let bytes = path_size_bytes(path);
            match remove_path(path) {
                Ok(_) => {
                    removed_archive += 1;
                    removed_archive_bytes = removed_archive_bytes.saturating_add(bytes);
                }
                Err(err) => errors.push(format!("archive_remove_failed:{}:{err}", path.display())),
            }
        }

        if target_candidate {
            removed_target_bytes = target_candidate_bytes;
            match remove_path(&policy.target_root) {
                Ok(_) => removed_target = true,
                Err(err) => errors.push(format!(
                    "target_remove_failed:{}:{err}",
                    policy.target_root.display()
                )),
            }
        }

        for path in &detached_candidates {
            let status = Command::new("git")
                .args(["worktree", "remove", "--force"])
                .arg(path)
                .current_dir(root)
                .status();
            match status {
                Ok(code) if code.success() => {
                    removed_detached_worktrees += 1;
                }
                Ok(code) => errors.push(format!(
                    "detached_worktree_remove_failed:{}:exit={}",
                    path.display(),
                    code.code().unwrap_or(1)
                )),
                Err(err) => errors.push(format!(
                    "detached_worktree_remove_failed:{}:{err}",
                    path.display()
                )),
            }
        }

        if has_git_root {
            let _ = Command::new("git")
                .args(["worktree", "prune", "--expire", "now"])
                .current_dir(root)
                .status();
        }
    }

    let (available_after_bytes, total_after_bytes, free_after_percent) = if apply {
        disk_free_snapshot(root)
            .map(|(available, total, free, _)| (available, total, free))
            .unwrap_or((
                available_before_bytes,
                total_before_bytes,
                free_before_percent,
            ))
    } else {
        (
            available_before_bytes,
            total_before_bytes,
            free_before_percent,
        )
    };

    let ok = errors.is_empty();
    let payload = json!({
        "ok": ok,
        "type": "spine_sleep_cleanup",
        "ts": now,
        "mode": if mode == SleepCleanupMode::Purge { "purge" } else { "normal" },
        "origin": origin,
        "applied": apply,
        "executed": true,
        "policy": {
            "enabled": policy.enabled,
            "min_interval_minutes": policy.min_interval_minutes,
            "archive_root": policy.archive_root,
            "archive_max_age_hours": policy.archive_max_age_hours,
            "archive_keep_latest": policy.archive_keep_latest,
            "target_root": policy.target_root,
            "target_max_age_hours": policy.target_max_age_hours,
            "detached_worktree_max_age_hours": policy.detached_worktree_max_age_hours,
            "disk_free_floor_percent": policy.disk_free_floor_percent,
            "hard_free_floor_percent": policy.hard_free_floor_percent,
            "pressure_target_free_percent": pressure_target_free_percent,
            "pressure_jsonl_cap_bytes": policy.pressure_jsonl_cap_bytes,
            "pressure_log_cap_bytes": policy.pressure_log_cap_bytes,
            "pressure_max_candidates": policy.pressure_max_candidates,
            "pressure_min_age_hours": policy.pressure_min_age_hours
        },
        "disk": {
            "mount_point": disk_mount_point,
            "free_percent_before": free_before_percent,
            "available_bytes_before": available_before_bytes,
            "total_bytes_before": total_before_bytes,
            "free_percent_after": free_after_percent,
            "available_bytes_after": available_after_bytes,
            "total_bytes_after": total_after_bytes
        },
        "pressure_mode": {
            "active": pressure_mode,
            "hard_floor_breach": hard_floor_breach,
            "target_available_bytes": pressure_target_available_bytes,
            "reclaim_needed_bytes": pressure_reclaim_needed_bytes
        },
        "candidates": {
            "archive_paths": archive_candidates.len(),
            "archive_bytes": archive_candidate_bytes,
            "target_path": target_candidate,
            "target_bytes": target_candidate_bytes,
            "target_reason": target_candidate_reason,
            "detached_worktrees": detached_candidates.len(),
            "pressure_paths": pressure_candidates.len(),
            "pressure_estimated_bytes": pressure_candidate_estimated_bytes
        },
        "removed": {
            "archive_paths": removed_archive,
            "archive_bytes": removed_archive_bytes,
            "target_path": removed_target,
            "target_bytes": removed_target_bytes,
            "detached_worktrees": removed_detached_worktrees,
            "pressure_paths": pressure_removed_files,
            "pressure_bytes": pressure_reclaimed_bytes
        },
        "pressure_actions": pressure_applied,
        "errors": errors
    });

    if apply {
        write_json_atomic(
            &policy.state_path,
            &json!({
                "type": "spine_sleep_cleanup_latest",
                "ts": now_iso(),
                "last_run_ms": now_epoch_ms(),
                "origin": origin,
                "result": payload
            }),
        );
        append_jsonl(
            &policy.history_path,
            &json!({
                "type": "spine_sleep_cleanup_history",
                "ts": now_iso(),
                "origin": origin,
                "result": payload
            }),
        );
    }

    (if ok { 0 } else { 1 }, payload)
}

pub(crate) fn execute_sleep_cleanup(
    root: &Path,
    apply: bool,
    force: bool,
    origin: &str,
) -> (i32, Value) {
    execute_sleep_cleanup_with_mode(root, apply, force, origin, SleepCleanupMode::Normal)
}

fn execute_sleep_cleanup_purge(
    root: &Path,
    apply: bool,
    force: bool,
    origin: &str,
) -> (i32, Value) {
    execute_sleep_cleanup_with_mode(root, apply, force, origin, SleepCleanupMode::Purge)
}

#[cfg(test)]
mod parse_worktree_blocks_tests {
    use super::*;

    #[test]
    fn parse_worktree_blocks_flushes_previous_block_without_blank_line() {
        let rows = parse_worktree_blocks(
            "worktree /tmp/a\ndetached\nworktree /tmp/b\nbranch refs/heads/main\n",
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], (PathBuf::from("/tmp/a"), true));
        assert_eq!(rows[1], (PathBuf::from("/tmp/b"), false));
    }
}
