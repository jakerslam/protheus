fn safe_wrapper_state_path(control_runtime_root: &Path) -> PathBuf {
    control_runtime_root.join("local/state/ops/control_runtime_safe/control_runtime-safe-state.json")
}

fn safe_wrapper_log_path(control_runtime_root: &Path) -> PathBuf {
    control_runtime_root.join("local/state/ops/control_runtime_safe/control_runtime-safe.jsonl")
}

include!("050_ops_helpers.rs");
include!("051_ops_tail.rs");

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

fn safe_run_command_key(domain: &str, args: &[String]) -> String {
    let mut parts = Vec::<String>::new();
    parts.push(clean_text(domain, 40).to_ascii_lowercase());
    for arg in args.iter().take(6) {
        let cleaned = clean_text(arg, 80).to_ascii_lowercase();
        if !cleaned.is_empty() {
            parts.push(cleaned);
        }
    }
    clean_text(&parts.join(" "), 320)
}

fn run_core_with_timeout(
    root: &Path,
    domain: &str,
    args: &[String],
    timeout_ms: u64,
) -> Result<Value, String> {
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
    let mut child = cmd
        .spawn()
        .map_err(|err| format!("safe_run_spawn_failed:{err}"))?;
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

fn run_control_runtime_health(control_runtime_root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let since_hours = parsed
        .flags
        .get("since-hours")
        .and_then(|raw| raw.trim().parse::<f64>().ok())
        .unwrap_or(48.0)
        .clamp(1.0, 24.0 * 30.0);
    let cutoff = epoch_secs_now().saturating_sub((since_hours * 3600.0) as u64);
    let rows = read_jsonl_rows(&safe_wrapper_log_path(control_runtime_root), 4000)
        .into_iter()
        .filter(|row| row.get("ts_epoch").and_then(Value::as_u64).unwrap_or(0) >= cutoff)
        .collect::<Vec<_>>();
    let mut by_status = BTreeMap::<String, usize>::new();
    let mut timeout_counts = BTreeMap::<String, usize>::new();
    for row in &rows {
        let result = row.get("result").cloned().unwrap_or_else(|| json!({}));
        let status = if result
            .get("timed_out")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
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
    let safe_state = read_json_file(&safe_wrapper_state_path(control_runtime_root))
        .unwrap_or_else(|| json!({"blacklist": {}, "timeouts": {}}));
    let blacklist = safe_state
        .get("blacklist")
        .cloned()
        .unwrap_or_else(|| json!({}));

    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_control_runtime_health",
        "since_hours": since_hours,
        "rows_count": rows.len(),
        "status_counts": by_status,
        "top_timeouts": top_timeouts,
        "adaptive_blacklist": blacklist,
        "state_path": safe_wrapper_state_path(control_runtime_root).to_string_lossy().to_string(),
        "log_path": safe_wrapper_log_path(control_runtime_root).to_string_lossy().to_string(),
    }))
}

fn run_safe_run(
    root: &Path,
    control_runtime_root: &Path,
    parsed: &crate::ParsedArgs,
) -> Result<Value, String> {
    let domain = parsed
        .positional
        .get(1)
        .map(|v| clean_text(v, 80))
        .unwrap_or_default();
    if domain.is_empty() {
        return Err("safe_run_domain_required".to_string());
    }
    let args = parsed
        .positional
        .iter()
        .skip(2)
        .cloned()
        .collect::<Vec<_>>();
    let cmd_key = safe_run_command_key(&domain, &args);
    if cmd_key == "models list" {
        return Ok(with_receipt(json!({
            "ok": false,
            "type": "operator_tooling_safe_run",
            "blocked": true,
            "cmd_key": cmd_key,
            "fallback": ["operator-tooling-kernel smoke-routing"]
        })));
    }

    let state_path = safe_wrapper_state_path(control_runtime_root);
    let mut state =
        read_json_file(&state_path).unwrap_or_else(|| json!({"blacklist": {}, "timeouts": {}}));
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

    let log_path = safe_wrapper_log_path(control_runtime_root);
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

fn run_shell_with_timeout(
    cwd: &Path,
    command_text: &str,
    timeout_ms: u64,
) -> Result<Value, String> {
    let mut cmd = Command::new("bash");
    cmd.arg("-lc")
        .arg(command_text)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|err| format!("safe_apply_spawn_failed:{err}"))?;
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

fn run_safe_apply(
    control_runtime_root: &Path,
    parsed: &crate::ParsedArgs,
    payload: &Value,
) -> Result<Value, String> {
    let title = parsed
        .flags
        .get("title")
        .map(|v| clean_text(v, 180))
        .or_else(|| {
            payload
                .get("title")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 180))
        })
        .unwrap_or_else(|| "Safe Apply".to_string());
    let reason = parsed
        .flags
        .get("reason")
        .map(|v| clean_text(v, 260))
        .or_else(|| {
            payload
                .get("reason")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 260))
        })
        .unwrap_or_default();
    let verify_cmd = parsed
        .flags
        .get("verify")
        .map(|v| clean_text(v, 400))
        .or_else(|| {
            payload
                .get("verify")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 400))
        })
        .unwrap_or_default();
    let command_text = parsed
        .flags
        .get("command")
        .map(|v| clean_text(v, 500))
        .or_else(|| {
            payload
                .get("command")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 500))
        })
        .unwrap_or_default();
    if command_text.is_empty() {
        return Err("safe_apply_command_required".to_string());
    }
    let timeout_ms =
        parse_usize_flag(&parsed.flags, "timeout-ms", 120_000, 1_000, 3_600_000) as u64;
    let stamp = crate::now_iso()
        .replace(':', "")
        .replace('-', "")
        .replace('T', "-")
        .replace('Z', "");
    let backup_dir = control_runtime_root.join("backups").join(stamp);
    fs::create_dir_all(&backup_dir)
        .map_err(|err| format!("safe_apply_backup_mkdir_failed:{err}"))?;
    let targets = safe_apply_targets(control_runtime_root, payload);
    let mut backed_up = Vec::<String>::new();
    for target in &targets {
        if !target.exists() {
            continue;
        }
        let backup_path = safe_apply_backup_path(control_runtime_root, &backup_dir, target);
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("safe_apply_backup_mkdir_failed:{err}"))?;
        }
        fs::copy(target, backup_path)
            .map_err(|err| format!("safe_apply_backup_copy_failed:{err}"))?;
        backed_up.push(target.to_string_lossy().to_string());
    }

    let command_result = run_shell_with_timeout(control_runtime_root, &command_text, timeout_ms)?;
    if !command_result
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let restored = rollback_from_backup(control_runtime_root, &targets, &backup_dir)?;
        return Err(format!(
            "safe_apply_command_failed:rolled_back={} status_code={}",
            restored.len(),
            command_result
                .get("status_code")
                .and_then(Value::as_i64)
                .unwrap_or(-1)
        ));
    }

    let verify_result = if verify_cmd.trim().is_empty() {
        Value::Null
    } else {
        let check = run_shell_with_timeout(control_runtime_root, &verify_cmd, timeout_ms)?;
        if !check.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            let restored = rollback_from_backup(control_runtime_root, &targets, &backup_dir)?;
            return Err(format!(
                "safe_apply_verify_failed:rolled_back={} status_code={}",
                restored.len(),
                check
                    .get("status_code")
                    .and_then(Value::as_i64)
                    .unwrap_or(-1)
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
        &decision_log_path(control_runtime_root, parsed),
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
