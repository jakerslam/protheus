fn parse_json_payload(raw: &str) -> Option<Value> {
    let text = raw.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        return Some(v);
    }
    for line in text.lines().rev() {
        let line = line.trim();
        if !line.starts_with('{') {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            return Some(v);
        }
    }
    None
}

fn spine_runs_dir(root: &Path) -> PathBuf {
    root.join("client/runtime/local/state/spine/runs")
}

fn ensure_dir(path: &Path) {
    let _ = fs::create_dir_all(path);
}

fn write_json_atomic(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        ensure_dir(parent);
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    if let Ok(mut payload) = serde_json::to_string_pretty(value) {
        payload.push('\n');
        if fs::write(&tmp, payload).is_ok() {
            let _ = fs::rename(tmp, path);
        }
    }
}

fn read_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn bool_from_env(name: &str) -> Option<bool> {
    let raw = std::env::var(name).ok()?;
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn bool_from_flag(argv: &[String], name: &str, default: bool) -> bool {
    let key = format!("--{name}");
    let key_eq = format!("--{name}=");
    let mut idx = 0usize;
    while idx < argv.len() {
        let token = argv[idx].trim();
        if token == key {
            if let Some(next) = argv.get(idx + 1) {
                match next.trim().to_ascii_lowercase().as_str() {
                    "1" | "true" | "yes" | "on" => return true,
                    "0" | "false" | "no" | "off" => return false,
                    _ => {}
                }
            }
        }
        if let Some(value) = token.strip_prefix(&key_eq) {
            match value.trim().to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => return true,
                "0" | "false" | "no" | "off" => return false,
                _ => {}
            }
        }
        idx += 1;
    }
    default
}

fn parse_i64_env(name: &str, default: i64, min: i64, max: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn parse_f64_env(name: &str, default: f64, min: f64, max: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<f64>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn parse_usize_env(name: &str, default: usize, max: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .unwrap_or(default)
        .min(max)
}

fn parse_u64_env(name: &str, default: u64, max: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(default)
        .min(max)
}

fn now_epoch_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn path_mtime_ms(path: &Path) -> Option<i64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let duration = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(duration.as_millis().min(i64::MAX as u128) as i64)
}

fn age_hours(now_ms: i64, mtime_ms: i64) -> i64 {
    ((now_ms - mtime_ms).max(0)) / (1000 * 60 * 60)
}

fn path_size_bytes(path: &Path) -> u64 {
    let meta = match fs::symlink_metadata(path) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(v) => v,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let m = match fs::symlink_metadata(&p) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if m.is_file() {
                total = total.saturating_add(m.len());
            } else if m.is_dir() {
                stack.push(p);
            }
        }
    }
    total
}

fn remove_path(path: &Path) -> std::io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn append_jsonl(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        ensure_dir(parent);
    }
    if let Ok(payload) = serde_json::to_string(value) {
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, format!("{payload}\n").as_bytes()));
    }
}

fn normalize_path(root: &Path, value: Option<&Value>, fallback: &str) -> PathBuf {
    let raw = value
        .and_then(Value::as_str)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback);
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn system_time_to_epoch_ms(ts: std::time::SystemTime) -> Option<i64> {
    ts.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
}

fn path_last_touch_ms(path: &Path) -> i64 {
    let Ok(meta) = fs::metadata(path) else {
        return 0;
    };
    let accessed = meta.accessed().ok().and_then(system_time_to_epoch_ms);
    let modified = meta.modified().ok().and_then(system_time_to_epoch_ms);
    accessed.or(modified).unwrap_or(0)
}

fn disk_free_snapshot(root: &Path) -> Option<(u64, u64, f64, String)> {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let disks = Disks::new_with_refreshed_list();
    let mut best: Option<(usize, u64, u64, String)> = None;
    for disk in disks.list() {
        let mount = disk.mount_point();
        if !canonical.starts_with(mount) {
            continue;
        }
        let rank = mount.as_os_str().len();
        let available = disk.available_space();
        let total = disk.total_space();
        let mount_label = mount.to_string_lossy().to_string();
        match &best {
            Some((best_rank, _, _, _)) if *best_rank >= rank => {}
            _ => best = Some((rank, available, total, mount_label)),
        }
    }
    if best.is_none() {
        if let Some(disk) = disks.list().first() {
            best = Some((
                0,
                disk.available_space(),
                disk.total_space(),
                disk.mount_point().to_string_lossy().to_string(),
            ));
        }
    }
    let (_, available, total, mount_label) = best?;
    let free_percent = if total == 0 {
        0.0
    } else {
        (available as f64 * 100.0) / total as f64
    };
    Some((available, total, free_percent, mount_label))
}

fn trim_file_to_tail(path: &Path, max_bytes: u64) -> Result<u64, String> {
    if max_bytes == 0 {
        return Ok(0);
    }
    let current = fs::metadata(path)
        .map(|meta| meta.len())
        .map_err(|err| format!("trim_metadata_failed:{}:{err}", path.display()))?;
    if current <= max_bytes {
        return Ok(0);
    }
    let keep = max_bytes.min(current);
    let mut file =
        fs::File::open(path).map_err(|err| format!("trim_open_failed:{}:{err}", path.display()))?;
    file.seek(std::io::SeekFrom::End(-(keep as i64)))
        .map_err(|err| format!("trim_seek_failed:{}:{err}", path.display()))?;
    let mut tail = Vec::<u8>::new();
    std::io::Read::read_to_end(&mut file, &mut tail)
        .map_err(|err| format!("trim_read_failed:{}:{err}", path.display()))?;
    let tmp = path.with_extension(format!(
        "trim-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    fs::write(&tmp, &tail).map_err(|err| format!("trim_write_failed:{}:{err}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|err| {
        format!(
            "trim_rename_failed:{}:{}:{err}",
            tmp.display(),
            path.display()
        )
    })?;
    Ok(current.saturating_sub(keep))
}

fn pressure_candidate_estimated_reclaim(candidate: &PressureCandidate) -> u64 {
    match candidate.action {
        PressureAction::RemoveFile => candidate.size_bytes,
        PressureAction::TrimTail { max_bytes } => candidate.size_bytes.saturating_sub(max_bytes),
    }
}

fn pressure_action_label(action: PressureAction) -> &'static str {
    match action {
        PressureAction::TrimTail { .. } => "trim_tail",
        PressureAction::RemoveFile => "remove_file",
    }
}

fn collect_pressure_candidates(
    root: &Path,
    policy: &SleepCleanupPolicy,
    now_ms: i64,
) -> Vec<PressureCandidate> {
    let roots = [
        root.join("core/local/state"),
        root.join("client/runtime/local/state"),
        root.join("local/state"),
    ];
    let mut rows = Vec::<PressureCandidate>::new();
    for state_root in roots {
        if !state_root.exists() {
            continue;
        }
        let mut stack = vec![state_root];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let Ok(meta) = fs::symlink_metadata(&path) else {
                    continue;
                };
                if meta.is_dir() {
                    stack.push(path);
                    continue;
                }
                if !meta.is_file() {
                    continue;
                }
                let size_bytes = meta.len();
                if size_bytes == 0 {
                    continue;
                }
                let last_touch_ms = path_last_touch_ms(&path);
                if age_hours(now_ms, last_touch_ms) < policy.pressure_min_age_hours {
                    continue;
                }
                let rel = path
                    .strip_prefix(root)
                    .ok()
                    .map(|v| v.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_else(|| path.to_string_lossy().to_ascii_lowercase());
                let action = if rel.ends_with("history.bin") {
                    Some(PressureAction::RemoveFile)
                } else if rel.ends_with(".jsonl") && size_bytes > policy.pressure_jsonl_cap_bytes {
                    Some(PressureAction::TrimTail {
                        max_bytes: policy.pressure_jsonl_cap_bytes,
                    })
                } else if rel.ends_with(".log") && size_bytes > policy.pressure_log_cap_bytes {
                    Some(PressureAction::TrimTail {
                        max_bytes: policy.pressure_log_cap_bytes,
                    })
                } else {
                    None
                };
                if let Some(action) = action {
                    rows.push(PressureCandidate {
                        path,
                        size_bytes,
                        last_touch_ms,
                        action,
                    });
                }
            }
        }
    }
    rows.sort_by(|a, b| {
        a.last_touch_ms
            .cmp(&b.last_touch_ms)
            .then_with(|| b.size_bytes.cmp(&a.size_bytes))
    });
    rows.truncate(policy.pressure_max_candidates);
    rows
}

fn load_sleep_cleanup_policy(root: &Path) -> SleepCleanupPolicy {
    SleepCleanupPolicy {
        enabled: bool_from_env("SPINE_SLEEP_CLEANUP_ENABLED").unwrap_or(true),
        min_interval_minutes: parse_i64_env(
            "SPINE_SLEEP_CLEANUP_MIN_INTERVAL_MINUTES",
            360,
            0,
            7 * 24 * 60,
        ),
        archive_root: root.join("local/workspace/archive"),
        archive_max_age_hours: parse_i64_env(
            "SPINE_SLEEP_CLEANUP_ARCHIVE_MAX_AGE_HOURS",
            7 * 24,
            0,
            365 * 24,
        ),
        archive_keep_latest: parse_usize_env("SPINE_SLEEP_CLEANUP_ARCHIVE_KEEP_LATEST", 6, 10_000),
        target_root: root.join("target"),
        target_max_age_hours: parse_i64_env(
            "SPINE_SLEEP_CLEANUP_TARGET_MAX_AGE_HOURS",
            48,
            0,
            365 * 24,
        ),
        detached_worktree_max_age_hours: parse_i64_env(
            "SPINE_SLEEP_CLEANUP_DETACHED_WORKTREE_MAX_AGE_HOURS",
            72,
            0,
            365 * 24,
        ),
        disk_free_floor_percent: parse_f64_env(
            "SPINE_SLEEP_CLEANUP_FREE_SPACE_FLOOR_PERCENT",
            20.0,
            1.0,
            95.0,
        ),
        pressure_target_free_percent: parse_f64_env(
            "SPINE_SLEEP_CLEANUP_PRESSURE_TARGET_FREE_PERCENT",
            30.0,
            2.0,
            98.0,
        ),
        pressure_jsonl_cap_bytes: parse_u64_env(
            "SPINE_SLEEP_CLEANUP_PRESSURE_JSONL_CAP_BYTES",
            512 * 1024,
            128 * 1024 * 1024,
        ),
        pressure_log_cap_bytes: parse_u64_env(
            "SPINE_SLEEP_CLEANUP_PRESSURE_LOG_CAP_BYTES",
            256 * 1024,
            64 * 1024 * 1024,
        ),
        pressure_max_candidates: parse_usize_env(
            "SPINE_SLEEP_CLEANUP_PRESSURE_MAX_CANDIDATES",
            10_000,
            200_000,
        ),
        pressure_min_age_hours: parse_i64_env(
            "SPINE_SLEEP_CLEANUP_PRESSURE_MIN_AGE_HOURS",
            4,
            0,
            365 * 24,
        ),
        state_path: root.join("client/runtime/local/state/ops/sleep_cleanup/latest.json"),
        history_path: root.join("client/runtime/local/state/ops/sleep_cleanup/history.jsonl"),
    }
}

