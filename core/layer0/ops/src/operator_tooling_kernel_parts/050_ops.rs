
fn safe_wrapper_state_path(openclaw_root: &Path) -> PathBuf {
    openclaw_root.join("state/openclaw-safe-state.json")
}

fn safe_wrapper_log_path(openclaw_root: &Path) -> PathBuf {
    openclaw_root.join("logs/openclaw-safe.jsonl")
}

fn read_jsonl_rows(path: &Path, limit: usize) -> Vec<Value> {
    if !path.exists() {
        return Vec::new();
    }
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return Vec::new(),
    };
    let mut rows = Vec::<Value>::new();
    let reader = BufReader::new(file);
    for line in reader.lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            rows.push(value);
            if rows.len() > limit {
                let drop = rows.len().saturating_sub(limit);
                rows = rows.into_iter().skip(drop).collect::<Vec<_>>();
            }
        }
    }
    rows
}

fn epoch_secs_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn safe_fallback_for(cmd_key: &str) -> Option<Vec<String>> {
    match cmd_key {
        "models list" => Some(vec![
            "operator-tooling-kernel".to_string(),
            "smoke-routing".to_string(),
        ]),
        _ if cmd_key.starts_with("models ") => Some(vec![
            "operator-tooling-kernel".to_string(),
            "smoke-routing".to_string(),
        ]),
        _ => None,
    }
}

fn run_core_with_timeout(root: &Path, domain: &str, args: &[String], timeout_ms: u64) -> Result<Value, String> {
    let current_exe = env::current_exe().map_err(|err| format!("current_exe_failed:{err}"))?;
    let mut cmd = Command::new(current_exe);
    cmd.arg(domain);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|err| format!("safe_run_spawn_failed:{err}"))?;
    let start = Instant::now();
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() >= Duration::from_millis(timeout_ms.max(1)) {
                    timed_out = true;
                    let _ = child.kill();
                    break;
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(err) => return Err(format!("safe_run_wait_failed:{err}")),
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|err| format!("safe_run_output_failed:{err}"))?;
    Ok(json!({
        "timed_out": timed_out,
        "ok": output.status.success() && !timed_out,
        "status_code": output.status.code().unwrap_or(-1),
        "stdout": clean_text(&String::from_utf8_lossy(&output.stdout), 4000),
        "stderr": clean_text(&String::from_utf8_lossy(&output.stderr), 4000),
        "duration_ms": start.elapsed().as_millis() as u64,
    }))
}

fn run_openclaw_health(openclaw_root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let since_hours = parsed
        .flags
        .get("since-hours")
        .and_then(|raw| raw.trim().parse::<f64>().ok())
        .unwrap_or(48.0)
        .clamp(1.0, 24.0 * 30.0);
    let cutoff = epoch_secs_now().saturating_sub((since_hours * 3600.0) as u64);
    let rows = read_jsonl_rows(&safe_wrapper_log_path(openclaw_root), 4000)
        .into_iter()
        .filter(|row| row.get("ts_epoch").and_then(Value::as_u64).unwrap_or(0) >= cutoff)
        .collect::<Vec<_>>();
    let mut by_status = BTreeMap::<String, usize>::new();
    let mut timeout_counts = BTreeMap::<String, usize>::new();
    for row in &rows {
        let result = row.get("result").cloned().unwrap_or_else(|| json!({}));
        let status = if result.get("timed_out").and_then(Value::as_bool).unwrap_or(false) {
            "timeout".to_string()
        } else if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            "ok".to_string()
        } else {
            "error".to_string()
        };
        *by_status.entry(status.clone()).or_insert(0) += 1;
        if status == "timeout" {
            let cmd = row
                .get("cmd_key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 240))
                .unwrap_or_else(|| "(unknown)".to_string());
            *timeout_counts.entry(cmd).or_insert(0) += 1;
        }
    }
    let top_timeouts = timeout_counts
        .into_iter()
        .map(|(cmd, count)| json!({ "cmd_key": cmd, "count": count }))
        .collect::<Vec<_>>();
    let safe_state = read_json_file(&safe_wrapper_state_path(openclaw_root))
        .unwrap_or_else(|| json!({"blacklist": {}, "timeouts": {}}));
    let blacklist = safe_state
        .get("blacklist")
        .cloned()
        .unwrap_or_else(|| json!({}));

    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_openclaw_health",
        "since_hours": since_hours,
        "rows_count": rows.len(),
        "status_counts": by_status,
        "top_timeouts": top_timeouts,
        "adaptive_blacklist": blacklist,
        "state_path": safe_wrapper_state_path(openclaw_root).to_string_lossy().to_string(),
        "log_path": safe_wrapper_log_path(openclaw_root).to_string_lossy().to_string(),
    }))
}

fn run_safe_run(root: &Path, openclaw_root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let domain = parsed
        .positional
        .get(1)
        .map(|v| clean_text(v, 80))
        .unwrap_or_default();
    if domain.is_empty() {
        return Err("safe_run_domain_required".to_string());
    }
    let args = parsed.positional.iter().skip(2).cloned().collect::<Vec<_>>();
    let cmd_key = format!(
        "{} {}",
        domain,
        args.first().map(|v| v.trim()).unwrap_or("")
    )
    .trim()
    .to_string();
    if cmd_key == "models list" {
        return Ok(with_receipt(json!({
            "ok": false,
            "type": "operator_tooling_safe_run",
            "blocked": true,
            "cmd_key": cmd_key,
            "fallback": ["operator-tooling-kernel smoke-routing"]
        })));
    }

    let state_path = safe_wrapper_state_path(openclaw_root);
    let mut state = read_json_file(&state_path).unwrap_or_else(|| json!({"blacklist": {}, "timeouts": {}}));
    let now = epoch_secs_now();
    let blacklisted_until = state
        .pointer(&format!("/blacklist/{cmd_key}/until_ts"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if blacklisted_until > now {
        return Ok(with_receipt(json!({
            "ok": false,
            "type": "operator_tooling_safe_run",
            "blocked": true,
            "cmd_key": cmd_key,
            "reason": "temporary_blacklist_active",
            "until_ts": blacklisted_until,
            "fallback": safe_fallback_for(&cmd_key)
        })));
    }

    let timeout_ms = parse_usize_flag(&parsed.flags, "timeout-ms", 15_000, 500, 180_000) as u64;
    let retries = parse_usize_flag(&parsed.flags, "retries", 1, 0, 5);
    let mut final_out = json!({});
    for attempt in 0..=retries {
        let out = run_core_with_timeout(root, &domain, &args, timeout_ms)?;
        final_out = out.clone();
        if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            break;
        }
        if attempt < retries {
            thread::sleep(Duration::from_millis(150));
        }
    }

    let timed_out = final_out
        .get("timed_out")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if timed_out {
        let current_timeouts = state
            .pointer(&format!("/timeouts/{cmd_key}"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .saturating_add(1);
        state["timeouts"][&cmd_key] = json!(current_timeouts);
        let cooldown = match current_timeouts {
            2 => 60,
            3 => 300,
            n if n >= 4 => 900,
            _ => 0,
        };
        if cooldown > 0 {
            state["blacklist"][&cmd_key] = json!({
                "until_ts": now.saturating_add(cooldown),
                "reason": "repeated_timeouts",
                "count": current_timeouts
            });
        }
    }
    write_json_file(&state_path, &state)?;

    let log_path = safe_wrapper_log_path(openclaw_root);
    let log_row = with_receipt(json!({
        "ok": final_out.get("ok").cloned().unwrap_or(Value::Bool(false)),
        "type": "operator_tooling_safe_run_event",
        "cmd_key": cmd_key,
        "domain": domain,
        "args": args,
        "result": final_out,
        "ts_epoch": now
    }));
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|err| format!("safe_run_log_open_failed:{err}"))?;
    let encoded = serde_json::to_string(&log_row).unwrap_or_else(|_| "{}".to_string());
    let _ = writeln!(log, "{encoded}");

    Ok(with_receipt(json!({
        "ok": final_out.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "type": "operator_tooling_safe_run",
        "cmd_key": cmd_key,
        "timeout_ms": timeout_ms,
        "retries": retries,
        "state_path": state_path.to_string_lossy().to_string(),
        "log_path": log_path.to_string_lossy().to_string(),
        "result": final_out
    })))
}

fn run_shell_with_timeout(cwd: &Path, command_text: &str, timeout_ms: u64) -> Result<Value, String> {
    let mut cmd = Command::new("bash");
    cmd.arg("-lc")
        .arg(command_text)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|err| format!("safe_apply_spawn_failed:{err}"))?;
    let start = Instant::now();
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() >= Duration::from_millis(timeout_ms.max(1)) {
                    timed_out = true;
                    let _ = child.kill();
                    break;
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(err) => return Err(format!("safe_apply_wait_failed:{err}")),
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|err| format!("safe_apply_output_failed:{err}"))?;
    Ok(json!({
        "ok": output.status.success() && !timed_out,
        "timed_out": timed_out,
        "status_code": output.status.code().unwrap_or(-1),
        "stdout": clean_text(&String::from_utf8_lossy(&output.stdout), 6000),
        "stderr": clean_text(&String::from_utf8_lossy(&output.stderr), 6000),
        "duration_ms": start.elapsed().as_millis() as u64
    }))
}

fn safe_apply_targets(openclaw_root: &Path, payload: &Value) -> Vec<PathBuf> {
    if let Some(rows) = payload.get("targets").and_then(Value::as_array) {
        let selected = rows
            .iter()
            .filter_map(Value::as_str)
            .map(PathBuf::from)
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    openclaw_root.join(path)
                }
            })
            .collect::<Vec<_>>();
        if !selected.is_empty() {
            return selected;
        }
    }
    vec![
        openclaw_root.join("openclaw.json"),
        openclaw_root.join("agents/main/agent/models.json"),
        openclaw_root.join("agents/main/agent/routing-policy.json"),
        openclaw_root.join("agents/main/agent/identity.md"),
    ]
}

fn rollback_from_backup(targets: &[PathBuf], backup_dir: &Path) -> Result<Vec<String>, String> {
    let mut restored = Vec::<String>::new();
    for target in targets {
        let Some(name) = target.file_name().map(|v| v.to_string_lossy().to_string()) else {
            continue;
        };
        let backup = backup_dir.join(name);
        if backup.exists() {
            fs::copy(&backup, target).map_err(|err| format!("safe_apply_rollback_copy_failed:{err}"))?;
            restored.push(target.to_string_lossy().to_string());
        }
    }
    Ok(restored)
}

fn run_safe_apply(openclaw_root: &Path, parsed: &crate::ParsedArgs, payload: &Value) -> Result<Value, String> {
    let title = parsed
        .flags
        .get("title")
        .map(|v| clean_text(v, 180))
        .or_else(|| payload.get("title").and_then(Value::as_str).map(|v| clean_text(v, 180)))
        .unwrap_or_else(|| "Safe Apply".to_string());
    let reason = parsed
        .flags
        .get("reason")
        .map(|v| clean_text(v, 260))
        .or_else(|| payload.get("reason").and_then(Value::as_str).map(|v| clean_text(v, 260)))
        .unwrap_or_default();
    let verify_cmd = parsed
        .flags
        .get("verify")
        .map(|v| clean_text(v, 400))
        .or_else(|| payload.get("verify").and_then(Value::as_str).map(|v| clean_text(v, 400)))
        .unwrap_or_default();
    let command_text = parsed
        .flags
        .get("command")
        .map(|v| clean_text(v, 500))
        .or_else(|| payload.get("command").and_then(Value::as_str).map(|v| clean_text(v, 500)))
        .unwrap_or_default();
    if command_text.is_empty() {
        return Err("safe_apply_command_required".to_string());
    }
    let timeout_ms = parse_usize_flag(&parsed.flags, "timeout-ms", 120_000, 1_000, 3_600_000) as u64;
    let stamp = crate::now_iso()
        .replace(':', "")
        .replace('-', "")
        .replace('T', "-")
        .replace('Z', "");
    let backup_dir = openclaw_root.join("backups").join(stamp);
    fs::create_dir_all(&backup_dir).map_err(|err| format!("safe_apply_backup_mkdir_failed:{err}"))?;
    let targets = safe_apply_targets(openclaw_root, payload);
    let mut backed_up = Vec::<String>::new();
    for target in &targets {
        if !target.exists() {
            continue;
        }
        let Some(name) = target.file_name() else {
            continue;
        };
        let backup_path = backup_dir.join(name);
        fs::copy(target, backup_path).map_err(|err| format!("safe_apply_backup_copy_failed:{err}"))?;
        backed_up.push(target.to_string_lossy().to_string());
    }

    let command_result = run_shell_with_timeout(openclaw_root, &command_text, timeout_ms)?;
    if !command_result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let restored = rollback_from_backup(&targets, &backup_dir)?;
        return Err(format!(
            "safe_apply_command_failed:rolled_back={} status_code={}",
            restored.len(),
            command_result.get("status_code").and_then(Value::as_i64).unwrap_or(-1)
        ));
    }

    let verify_result = if verify_cmd.trim().is_empty() {
        Value::Null
    } else {
        let check = run_shell_with_timeout(openclaw_root, &verify_cmd, timeout_ms)?;
        if !check.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            let restored = rollback_from_backup(&targets, &backup_dir)?;
            return Err(format!(
                "safe_apply_verify_failed:rolled_back={} status_code={}",
                restored.len(),
                check.get("status_code").and_then(Value::as_i64).unwrap_or(-1)
            ));
        }
        check
    };

    let rollback_hint = format!("Restore from {}", backup_dir.to_string_lossy());
    let details = json!({
        "backup_dir": backup_dir.to_string_lossy().to_string(),
        "command": command_text,
        "targets": backed_up
    });
    append_decision_markdown(
        &decision_log_path(openclaw_root, parsed),
        &title,
        &reason,
        &verify_cmd,
        &rollback_hint,
        &details,
    )?;

    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_safe_apply",
        "backup_dir": backup_dir.to_string_lossy().to_string(),
        "command": command_result,
        "verify": verify_result
    })))
}

fn cron_runtime_jobs_path(openclaw_root: &Path) -> PathBuf {
    openclaw_root.join("cron/jobs.json")
}

fn cron_workspace_mirror_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join("config/infring_assimilation/cron/jobs.json")
}

fn run_cron_drift(openclaw_root: &Path, workspace_root: &Path) -> Value {
    let runtime = cron_runtime_jobs_path(openclaw_root);
    let mirror = cron_workspace_mirror_path(workspace_root);
    let runtime_raw = fs::read_to_string(&runtime).unwrap_or_default();
    let mirror_raw = fs::read_to_string(&mirror).unwrap_or_default();
    let runtime_json = parse_json(runtime_raw.trim()).unwrap_or_else(|| json!({}));
    let mirror_json = parse_json(mirror_raw.trim()).unwrap_or_else(|| json!({}));
    let runtime_norm = serde_json::to_string(&runtime_json).unwrap_or_default();
    let mirror_norm = serde_json::to_string(&mirror_json).unwrap_or_default();
    let in_sync = !runtime_norm.is_empty() && runtime_norm == mirror_norm;
    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_cron_drift",
        "in_sync": in_sync,
        "runtime_path": runtime.to_string_lossy().to_string(),
        "mirror_path": mirror.to_string_lossy().to_string(),
        "runtime_exists": runtime.exists(),
        "mirror_exists": mirror.exists(),
        "runtime_jobs_count": runtime_json.get("jobs").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "mirror_jobs_count": mirror_json.get("jobs").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
    }))
}

fn run_cron_sync(openclaw_root: &Path, workspace_root: &Path) -> Result<Value, String> {
    let runtime = cron_runtime_jobs_path(openclaw_root);
    let mirror = cron_workspace_mirror_path(workspace_root);
    if !runtime.exists() {
        return Err("cron_runtime_jobs_missing".to_string());
    }
    let raw = fs::read_to_string(&runtime).map_err(|err| format!("cron_runtime_read_failed:{err}"))?;
    if let Some(parent) = mirror.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("cron_mirror_mkdir_failed:{err}"))?;
    }
    fs::write(&mirror, raw).map_err(|err| format!("cron_mirror_write_failed:{err}"))?;
    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_cron_sync",
        "runtime_path": runtime.to_string_lossy().to_string(),
        "mirror_path": mirror.to_string_lossy().to_string()
    })))
}

fn run_doctor(openclaw_root: &Path, workspace_root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let routing = run_smoke_routing(openclaw_root, parsed);
    let cron = run_cron_drift(openclaw_root, workspace_root);
    let checks = vec![
        json!({
            "id": "routing_smoke",
            "ok": routing.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "detail": routing
        }),
        json!({
            "id": "cron_in_sync",
            "ok": cron.get("in_sync").and_then(Value::as_bool).unwrap_or(false),
            "detail": cron
        }),
        json!({
            "id": "agent_state_exists",
            "ok": state_path(openclaw_root, parsed).exists(),
            "detail": state_path(openclaw_root, parsed).to_string_lossy().to_string()
        }),
    ];
    let ok = checks
        .iter()
        .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    with_receipt(json!({
        "ok": ok,
        "type": "operator_tooling_doctor",
        "openclaw_root": openclaw_root.to_string_lossy().to_string(),
        "workspace_root": workspace_root.to_string_lossy().to_string(),
        "checks": checks
    }))
}

fn run_audit_plane(openclaw_root: &Path, workspace_root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let doctor = run_doctor(openclaw_root, workspace_root, parsed);
    let memory_recent = run_memory_last_change(openclaw_root, 10);
    with_receipt(json!({
        "ok": doctor.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "type": "operator_tooling_audit_plane",
        "doctor": doctor,
        "memory_recent": memory_recent
    }))
}

fn run_daily_brief(openclaw_root: &Path, workspace_root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let state_file = state_path(openclaw_root, parsed);
    let state = read_json_file(&state_file).unwrap_or_else(|| json!({}));
    let last_task = state.get("last_task").cloned().unwrap_or_else(|| json!({}));
    let routing = state.get("routing").cloned().unwrap_or_else(|| json!({}));
    let prefs = state.get("preferences").cloned().unwrap_or_else(|| json!({}));
    let spawn_events = read_jsonl_rows(&openclaw_root.join("logs/spawn-safe.jsonl"), 40);
    let mut seen = HashSet::<String>::new();
    let mut recent_models = Vec::<String>::new();
    for row in spawn_events.iter().rev() {
        let model = row
            .get("model")
            .and_then(Value::as_str)
            .or_else(|| row.pointer("/packet/handoff/selected_model").and_then(Value::as_str))
            .map(|v| clean_text(v, 240))
            .unwrap_or_default();
        if model.is_empty() || !seen.insert(model.clone()) {
            continue;
        }
        recent_models.push(model);
        if recent_models.len() >= 5 {
            break;
        }
    }
    recent_models.reverse();

    let mut recommendations = Vec::<String>::new();
    if !state_file.exists() {
        recommendations.push("Create state file by running `infring state-write` for the first task.".to_string());
    }
    if prefs
        .get("always_sync_allowlist")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && !agent_root(openclaw_root).join("models.json").exists()
    {
        recommendations.push("Allowlist appears missing. Run `infring sync-allowed-models`.".to_string());
    }
    if recommendations.is_empty() {
        recommendations.push("No blockers detected. Continue with tagged tasks via `infring smart-spawn`.".to_string());
    }

    let audit = run_audit_plane(openclaw_root, workspace_root, parsed);

    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_daily_brief",
        "generated_at": crate::now_iso(),
        "project": state.get("project").cloned().unwrap_or_else(|| json!({})),
        "last_task": last_task,
        "routing": {
            "required_tags_min": routing.get("required_tags_min").cloned().unwrap_or(json!(3)),
            "required_tags_max": routing.get("required_tags_max").cloned().unwrap_or(json!(6)),
            "high_risk_tags": routing.get("high_risk_tags").cloned().unwrap_or_else(|| json!([])),
            "high_risk_requires_plan": routing
                .get("high_risk_requires_plan")
                .cloned()
                .unwrap_or(json!(true))
        },
        "preferences": {
            "default_timeout_seconds": prefs.get("default_timeout_seconds").cloned().unwrap_or(json!(30)),
            "always_use_spawn_safe": prefs.get("always_use_spawn_safe").cloned().unwrap_or(json!(true)),
            "always_sync_allowlist": prefs.get("always_sync_allowlist").cloned().unwrap_or(json!(true))
        },
        "recent_models": recent_models,
        "recommendations": recommendations,
        "control_plane": audit
    }))
}

fn run_fail_playbook(openclaw_root: &Path, workspace_root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let doctor = run_doctor(openclaw_root, workspace_root, parsed);
    let mut actions = Vec::<String>::new();
    if doctor
        .pointer("/checks/0/ok")
        .and_then(Value::as_bool)
        == Some(false)
    {
        actions.push("Run: infring smoke-routing".to_string());
        actions.push("Then: infring sync-allowed-models".to_string());
    }
    if doctor
        .pointer("/checks/1/ok")
        .and_then(Value::as_bool)
        == Some(false)
    {
        actions.push("Run: infring cron-sync".to_string());
    }
    if doctor
        .pointer("/checks/2/ok")
        .and_then(Value::as_bool)
        == Some(false)
    {
        actions.push("Run: infring state-write --payload='{\"task\":\"bootstrap\"}'".to_string());
    }
    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_fail_playbook",
        "actions": actions,
        "doctor": doctor
    }))
}

fn summary_status(openclaw_root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let policy = routing_policy_path(openclaw_root, parsed);
    let state = state_path(openclaw_root, parsed);
    let decisions = decision_log_path(openclaw_root, parsed);
    let files = vec![
        ("routing_policy", policy),
        ("state", state),
        ("decisions", decisions),
        ("logs_spawn_safe", openclaw_root.join("logs/spawn-safe.jsonl")),
        ("logs_spawn_run", openclaw_root.join("logs/spawn-run.jsonl")),
    ];
    let file_rows = files
        .into_iter()
        .map(|(label, path)| {
            json!({
                "label": label,
                "path": path.to_string_lossy().to_string(),
                "exists": path.exists()
            })
        })
        .collect::<Vec<_>>();
    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_status",
        "openclaw_root": openclaw_root.to_string_lossy().to_string(),
        "commands": [
            "status",
            "route-model",
            "escalate-model",
            "plan-auto",
            "plan-validate",
            "postflight-validate",
            "output-validate",
            "state-read",
            "state-write",
            "decision-log-append",
            "safe-apply",
            "memory-search",
            "memory-summarize",
            "memory-last-change",
            "membrief",
            "trace-find",
            "sync-allowed-models",
            "smoke-routing",
            "spawn-safe",
            "smart-spawn",
            "auto-spawn",
            "execute-handoff",
            "safe-run",
            "openclaw-health",
            "cron-drift",
            "cron-sync",
            "doctor",
            "audit-plane",
            "daily-brief",
            "fail-playbook"
        ],
        "paths": file_rows
    }))
}

fn error_receipt(command: &str, error: &str, code: i32) -> Value {
    with_receipt(json!({
        "ok": false,
        "type": "operator_tooling_error",
        "command": command,
        "error": clean_text(error, 220),
        "exit_code": code
    }))
}
