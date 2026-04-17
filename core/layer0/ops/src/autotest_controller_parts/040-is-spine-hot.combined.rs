// FILE_SIZE_EXCEPTION: reason=Atomic autotest orchestration block requires staged extraction to avoid behavior drift; owner=jay; expires=2026-04-12
fn is_spine_hot(paths: &RuntimePaths, window_sec: i64) -> Value {
    let today = &now_iso()[..10];
    let file = paths.spine_runs_dir.join(format!("{today}.jsonl"));
    if !file.exists() {
        return json!({ "hot": false, "reason": "spine_ledger_missing" });
    }
    let mut latest_started_ms = None::<i64>;
    let mut latest_terminal_ms = None::<i64>;
    for row in read_jsonl(&file) {
        let typ = row.get("type").and_then(Value::as_str).unwrap_or_default();
        let ts_ms = row
            .get("ts")
            .and_then(Value::as_str)
            .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
            .map(|v| v.timestamp_millis());
        if typ == "spine_run_started" {
            latest_started_ms = ts_ms.or(latest_started_ms);
        }
        if typ == "spine_run_complete" || typ == "spine_run_failed" {
            latest_terminal_ms = ts_ms.or(latest_terminal_ms);
        }
    }
    let now_ms = chrono::Utc::now().timestamp_millis();
    let hot = latest_started_ms
        .map(|started| {
            let age_sec = (now_ms - started) / 1000;
            if age_sec > window_sec {
                return false;
            }
            latest_terminal_ms.map(|end| end < started).unwrap_or(true)
        })
        .unwrap_or(false);
    json!({
        "hot": hot,
        "window_sec": window_sec,
        "last_started_ms": latest_started_ms,
        "last_terminal_ms": latest_terminal_ms
    })
}
fn run_shell_command(root: &Path, command: &str, timeout_ms: i64) -> CommandResult {
    let start = Instant::now();
    let mut child = match Command::new("sh")
        .arg("-lc")
        .arg(command)
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(err) => {
            return CommandResult {
                ok: false,
                exit_code: 1,
                signal: None,
                timed_out: false,
                duration_ms: start.elapsed().as_millis(),
                stdout_excerpt: String::new(),
                stderr_excerpt: short_text(&format!("spawn_failed:{err}"), 800),
            }
        }
    };

    let timeout = Duration::from_millis(timeout_ms.max(1000) as u64);
    let mut timed_out = false;

    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    timed_out = true;
                    let _ = child.kill();
                    break;
                }
                thread::sleep(Duration::from_millis(15));
            }
            Err(err) => {
                return CommandResult {
                    ok: false,
                    exit_code: 1,
                    signal: None,
                    timed_out: false,
                    duration_ms: start.elapsed().as_millis(),
                    stdout_excerpt: String::new(),
                    stderr_excerpt: short_text(&format!("wait_failed:{err}"), 800),
                }
            }
        }
    }

    let output = child.wait_with_output();
    match output {
        Ok(out) => {
            let code = out.status.code().unwrap_or(1);
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            CommandResult {
                ok: !timed_out && code == 0,
                exit_code: code,
                signal: None,
                timed_out,
                duration_ms: start.elapsed().as_millis(),
                stdout_excerpt: short_text(&stdout, 800),
                stderr_excerpt: short_text(&stderr, 800),
            }
        }
        Err(err) => CommandResult {
            ok: false,
            exit_code: 1,
            signal: None,
            timed_out,
            duration_ms: start.elapsed().as_millis(),
            stdout_excerpt: String::new(),
            stderr_excerpt: short_text(&format!("output_failed:{err}"), 800),
        },
    }
}

fn command_path_hints(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .map(|tok| tok.trim_matches('"').trim_matches('\''))
        .filter(|tok| {
            (tok.starts_with("client/runtime/systems/")
                || tok.starts_with("tests/client-memory-tools/"))
                && (tok.ends_with(".js") || tok.ends_with(".ts"))
        })
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
}

fn normalize_guard_file_list(files: &[String]) -> Vec<String> {
    let mut uniq = HashSet::new();
    let mut out = Vec::new();
    for file in files {
        let clean = file.trim().replace('\\', "/");
        if clean.is_empty() || clean.contains("..") {
            continue;
        }
        if uniq.insert(clean.clone()) {
            out.push(clean);
        }
    }
    out.sort();
    out
}

fn run_guard_for_files(root: &Path, files: &[String]) -> GuardResult {
    if files.is_empty() {
        return GuardResult {
            ok: true,
            reason: None,
            files: Vec::new(),
            stderr_excerpt: None,
            stdout_excerpt: None,
            duration_ms: 0,
        };
    }
    let guard_path = root.join("client/runtime/systems/security/guard.ts");
    if !guard_path.exists() {
        return GuardResult {
            ok: true,
            reason: Some("guard_missing_fail_open".to_string()),
            files: files.to_vec(),
            stderr_excerpt: None,
            stdout_excerpt: None,
            duration_ms: 0,
        };
    }

    let start = Instant::now();
    let run = Command::new("node")
        .arg(guard_path)
        .arg(format!("--files={}", files.join(",")))
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match run {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            GuardResult {
                ok: out.status.success(),
                reason: if out.status.success() {
                    None
                } else {
                    Some("guard_blocked".to_string())
                },
                files: files.to_vec(),
                stderr_excerpt: Some(short_text(&stderr, 400)),
                stdout_excerpt: Some(short_text(&stdout, 400)),
                duration_ms: start.elapsed().as_millis(),
            }
        }
        Err(err) => GuardResult {
            ok: false,
            reason: Some(format!("guard_exec_failed:{err}")),
            files: files.to_vec(),
            stderr_excerpt: None,
            stdout_excerpt: None,
            duration_ms: start.elapsed().as_millis(),
        },
    }
}

fn reverse_module_mapping(status: &StatusState) -> HashMap<String, Vec<String>> {
    let mut out = HashMap::<String, Vec<String>>::new();
    for module in status.modules.values() {
        for test_id in &module.mapped_test_ids {
            out.entry(test_id.clone())
                .or_default()
                .push(module.path.clone());
        }
    }
    out
}

fn test_set_for_scope(status: &StatusState, scope: &str) -> HashSet<String> {
    let mut selected = HashSet::new();
    match scope {
        "all" => {
            selected.extend(status.tests.keys().cloned());
        }
        "critical" => {
            for test in status.tests.values() {
                if test.critical {
                    selected.insert(test.id.clone());
                }
            }
        }
        _ => {
            for module in status.modules.values() {
                if module.changed {
                    for id in &module.mapped_test_ids {
                        selected.insert(id.clone());
                    }
                }
            }
            for test in status.tests.values() {
                if test.critical {
                    selected.insert(test.id.clone());
                }
            }
        }
    }
    selected
}

fn module_stale_ms(module: &ModuleRow, now_ms: i64) -> i64 {
    let last_test_ms = module
        .last_test_ts
        .as_deref()
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.timestamp_millis())
        .unwrap_or(0);
    (now_ms - last_test_ms).max(0)
}

fn prioritize_tests(status: &StatusState, test_ids: &HashSet<String>) -> Vec<PrioritizedTest> {
    let reverse = reverse_module_mapping(status);
    let now_ms = chrono::Utc::now().timestamp_millis();

    let mut out = Vec::new();
    for id in test_ids {
        let Some(test) = status.tests.get(id).cloned() else {
            continue;
        };
        let mapped_modules = reverse.get(id).cloned().unwrap_or_default();
        let mut score = 0i64;
        let mut priority = "normal".to_string();

        if test.critical {
            score += 100;
            priority = "critical".to_string();
        }
        if test.last_status == "fail" {
            score += 40;
            priority = "high".to_string();
        }
        if test.last_status == "untested" {
            score += 30;
        }

        let mut changed_count = 0i64;
        let mut stale_score = 0i64;
        for module_path in mapped_modules {
            if let Some(module) = status.modules.get(&module_path) {
                if module.changed {
                    changed_count += 1;
                }
                stale_score += (module_stale_ms(module, now_ms) / 1000).min(300);
            }
        }
        score += changed_count * 20;
        score += stale_score.min(120);

        out.push(PrioritizedTest {
            id: id.clone(),
            score,
            priority,
            test,
        });
    }

    out.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.id.cmp(&b.id))
            .then_with(|| a.test.command.cmp(&b.test.command))
    });
    out
}

fn summarize_external_health(paths: &RuntimePaths, policy: &Policy) -> Value {
    let mut sources = Vec::<PathBuf>::new();
    if !policy.external_health_paths.is_empty() {
        for raw in &policy.external_health_paths {
            let p = PathBuf::from(raw);
            sources.push(if p.is_absolute() {
                p
            } else {
                paths
                    .state_dir
                    .parent()
                    .unwrap_or(paths.state_dir.as_path())
                    .join(p)
            });
        }
    } else {
        sources.push(paths.pain_signals_path.clone());
    }

    let since_ms = chrono::Utc::now().timestamp_millis()
        - (policy.external_health_window_hours * 60 * 60 * 1000);

    let mut total = 0usize;
    let mut high_or_critical = 0usize;
    let mut latest_ts = None::<String>;

    for src in &sources {
        for row in read_jsonl(src) {
            let ts = row
                .get("ts")
                .or_else(|| row.get("timestamp"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let ts_ms = chrono::DateTime::parse_from_rfc3339(ts)
                .ok()
                .map(|v| v.timestamp_millis())
                .unwrap_or(0);
            if ts_ms < since_ms {
                continue;
            }
            total += 1;
            let sev = row
                .get("severity")
                .and_then(Value::as_str)
                .unwrap_or("medium")
                .to_ascii_lowercase();
            if sev == "high" || sev == "critical" {
                high_or_critical += 1;
            }
            latest_ts = Some(ts.to_string());
        }
    }

    let available = total > 0;
    json!({
        "enabled": true,
        "available": available,
        "window_hours": policy.external_health_window_hours,
        "total": total,
        "high_or_critical": high_or_critical,
        "latest_ts": latest_ts,
        "path": sources
            .first()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| paths.pain_signals_path.to_string_lossy().to_string())
    })
}

fn cmd_run(root: &Path, cli: &CliArgs, policy: &Policy, paths: &RuntimePaths) -> Value {
    let run_start = Instant::now();
    let strict = to_bool(
        cli.flags.get("strict").map(String::as_str),
        policy.execution.strict,
    );
    let sleep_only = to_bool(cli.flags.get("sleep-only").map(String::as_str), false);
    let force = to_bool(cli.flags.get("force").map(String::as_str), false);
    let scope = cli
        .flags
        .get("scope")
        .map(String::as_str)
        .filter(|s| ["critical", "changed", "all"].contains(s))
        .unwrap_or(policy.execution.default_scope.as_str())
        .to_string();
    let max_tests = clamp_i64(
        cli.flags.get("max-tests").map(String::as_str),
        1,
        500,
        policy.execution.max_tests_per_run as i64,
    ) as usize;
    let run_timeout_ms = clamp_i64(
        cli.flags.get("run-timeout-ms").map(String::as_str),
        1_000,
        2 * 60 * 60 * 1_000,
        policy.execution.run_timeout_ms,
    );

    let run_deadline = Instant::now() + Duration::from_millis(run_timeout_ms as u64);
    let mut phase_ms = json!({
        "sync_ms": 0,
        "select_ms": 0,
        "execute_ms": 0,
        "total_ms": 0
    });

    let sync_started = Instant::now();
    let sync_out = sync_state(root, paths, policy);
    phase_ms["sync_ms"] = json!(sync_started.elapsed().as_millis());

    let mut status = load_status(paths);
    let external_health = summarize_external_health(paths, policy);
    let sleep_gate = in_sleep_window(policy);
    let resources = runtime_resource_within(policy);
    let spine_hot = is_spine_hot(paths, policy.runtime_guard.spine_hot_window_sec);

    let mut skip_reasons = Vec::<String>::new();
    if sleep_only && !sleep_gate {
        skip_reasons.push("outside_sleep_window".to_string());
    }
    if !resources.get("ok").and_then(Value::as_bool).unwrap_or(true) {
        skip_reasons.push("resource_guard".to_string());
    }
    if spine_hot
        .get("hot")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        skip_reasons.push("spine_hot".to_string());
    }

    if !skip_reasons.is_empty() && !force {
        let now = now_iso();
        phase_ms["total_ms"] = json!(run_start.elapsed().as_millis());
        let mut out = json!({
            "ok": true,
            "type": "autotest_run",
            "ts": now,
            "scope": scope,
            "strict": strict,
            "skipped": true,
            "skip_reasons": skip_reasons,
            "synced": sync_out,
            "external_health": external_health,
            "sleep_window_ok": sleep_gate,
            "resource_guard": resources,
            "spine_hot": spine_hot,
            "run_timeout_ms": run_timeout_ms,
            "phase_ms": phase_ms,
            "claim_evidence": [
                {
                    "id": "execution_gate",
                    "claim": "autotest_run_was_safely_skipped",
                    "evidence": {
                        "skip_reasons": skip_reasons
                    }
                }
            ],
            "persona_lenses": {
                "operator": {
                    "mode": "defensive",
                    "reason": "runtime_guard"
                }
            }
        });
        out["receipt_hash"] = Value::String(receipt_hash(&out));
        let _ = write_json_atomic(&paths.latest_path, &out);
        let _ = append_jsonl(&paths.runs_dir.join(format!("{}.jsonl", &now[..10])), &out);
        return out;
    }

    let select_started = Instant::now();
    let test_ids = test_set_for_scope(&status, &scope);
    let prioritized = prioritize_tests(&status, &test_ids);
    let selected = prioritized
        .iter()
        .take(max_tests)
        .map(|row| row.test.clone())
        .collect::<Vec<_>>();
    let selection_preview = prioritized
        .iter()
        .take(24)
        .map(|row| {
            json!({
                "id": row.id,
                "score": row.score,
                "priority": row.priority
            })
        })
        .collect::<Vec<_>>();
    let test_to_modules = reverse_module_mapping(&status);
    phase_ms["select_ms"] = json!(select_started.elapsed().as_millis());

    let execute_started = Instant::now();
    let mut results = Vec::<Value>::new();
    let mut guard_blocked = 0usize;
    let mut flaky_count = 0usize;
    let mut quarantined_count = 0usize;
    let mut executed_status = HashMap::<String, String>::new();

    for (idx, test) in selected.iter().enumerate() {
        if policy.execution.midrun_resource_guard
            && idx % policy.execution.resource_recheck_every_tests == 0
        {
            let loop_resources = runtime_resource_within(policy);
            if !loop_resources
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && !force
            {
                let selected_tests = results.len();
                let passed = results
                    .iter()
                    .filter(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false))
                    .count();
                let failed = results
                    .iter()
                    .filter(|row| !row.get("ok").and_then(Value::as_bool).unwrap_or(false))
                    .count();
                phase_ms["execute_ms"] = json!(execute_started.elapsed().as_millis());
                phase_ms["total_ms"] = json!(run_start.elapsed().as_millis());
                let mut out = json!({
                    "ok": false,
                    "type": "autotest_run",
                    "ts": now_iso(),
                    "scope": scope,
                    "strict": strict,
                    "aborted": true,
                    "abort_reason": "resource_guard_during_execution",
                    "selected_tests": selected_tests,
                    "passed": passed,
                    "failed": failed,
                    "resource_guard_runtime": loop_resources,
                    "partial_results": results,
                    "phase_ms": phase_ms,
                    "claim_evidence": [
                        {
                            "id": "resource_guard_runtime",
                            "claim": "run_aborted_when_runtime_resource_guard_failed",
                            "evidence": {
                                "selected_tests": selected_tests,
                                "failed": failed
                            }
                        }
                    ],
                    "persona_lenses": {
                        "operator": {
                            "mode": "run",
                            "abort_reason": "resource_guard_during_execution"
                        },
                        "auditor": {
                            "strict": strict
                        }
                    }
                });
                out["receipt_hash"] = Value::String(receipt_hash(&out));
                let _ = write_json_atomic(&paths.latest_path, &out);
                return out;
            }
        }

        if Instant::now() > run_deadline {
            let selected_tests = results.len();
            let passed = results
                .iter()
                .filter(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false))
                .count();
            let failed = results
                .iter()
                .filter(|row| !row.get("ok").and_then(Value::as_bool).unwrap_or(false))
                .count();
            phase_ms["execute_ms"] = json!(execute_started.elapsed().as_millis());
            phase_ms["total_ms"] = json!(run_start.elapsed().as_millis());
            let mut out = json!({
                "ok": false,
                "type": "autotest_run",
                "ts": now_iso(),
                "scope": scope,
                "strict": strict,
                "timeout": true,
                "timeout_reason": "execution_budget_exhausted",
                "selected_tests": selected_tests,
                "passed": passed,
                "failed": failed,
                "partial_results": results,
                "phase_ms": phase_ms,
                "claim_evidence": [
                    {
                        "id": "execution_budget",
                        "claim": "run_times_out_when_execution_budget_exhausted",
                        "evidence": {
                            "selected_tests": selected_tests,
                            "failed": failed
                        }
                    }
                ],
                "persona_lenses": {
                    "operator": {
                        "mode": "run",
                        "timeout_reason": "execution_budget_exhausted"
                    },
                    "auditor": {
                        "strict": strict
                    }
                }
            });
            out["receipt_hash"] = Value::String(receipt_hash(&out));
            let _ = write_json_atomic(&paths.latest_path, &out);
            return out;
        }

        let mut guard_files = Vec::<String>::new();
        if let Some(path) = &test.path {
            guard_files.push(path.clone());
        }
        if let Some(modules) = test_to_modules.get(&test.id) {
            guard_files.extend(modules.iter().cloned());
        }
        guard_files.extend(command_path_hints(&test.command));
        let guard_files = normalize_guard_file_list(&guard_files);
        let guard = run_guard_for_files(root, &guard_files);

        let remaining_ms = run_deadline
            .checked_duration_since(Instant::now())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(1_000)
            .max(1_000);
        let per_test_timeout = policy
            .execution
            .timeout_ms_per_test
            .min(remaining_ms)
            .max(1_000);

        let mut res = if guard.ok {
            run_shell_command(root, &test.command, per_test_timeout)
        } else {
            guard_blocked += 1;
            CommandResult {
                ok: false,
                exit_code: 1,
                signal: None,
                timed_out: false,
                duration_ms: guard.duration_ms,
                stdout_excerpt: short_text(
                    &format!("guard_blocked:{}", guard.reason.clone().unwrap_or_default()),
                    800,
                ),
                stderr_excerpt: short_text(
                    &format!(
                        "{} {}",
                        guard.stderr_excerpt.clone().unwrap_or_default(),
                        guard.stdout_excerpt.clone().unwrap_or_default()
                    ),
                    800,
                ),
            }
        };

        let mut retried = false;
        let mut flaky = false;
        if guard.ok
            && !res.ok
            && policy.execution.retry_flaky_once
            && !test.critical
            && !res.timed_out
            && Instant::now() < run_deadline
        {
            retried = true;
            let remaining_ms = run_deadline
                .checked_duration_since(Instant::now())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(1_000)
                .max(1_000);
            let retry_timeout = policy
                .execution
                .timeout_ms_per_test
                .min(remaining_ms)
                .max(1_000);
            let retry = run_shell_command(root, &test.command, retry_timeout);
            if retry.ok {
                flaky = true;
                flaky_count += 1;
                res = retry;
            } else {
                res = retry;
            }
        }

        if let Some(row) = status.tests.get_mut(&test.id) {
            row.last_status = if res.ok {
                "pass".to_string()
            } else {
                "fail".to_string()
            };
            row.last_exit_code = Some(res.exit_code);
            row.last_run_ts = Some(now_iso());
            row.last_duration_ms = Some(res.duration_ms);
            row.last_stdout_excerpt = Some(res.stdout_excerpt.clone());
            row.last_stderr_excerpt = Some(res.stderr_excerpt.clone());
            row.last_guard = Some(GuardMeta {
                ok: guard.ok,
                reason: guard.reason.clone(),
                files: guard.files.clone(),
            });
            row.last_retry_count = Some(if retried { 1 } else { 0 });
            row.last_flaky = Some(flaky);
            let current_flaky = row.consecutive_flaky.unwrap_or(0);
            row.consecutive_flaky = Some(if flaky { current_flaky + 1 } else { 0 });
            if flaky
                && row.consecutive_flaky.unwrap_or(0) >= policy.execution.flaky_quarantine_after
            {
                let ts = chrono::Utc::now()
                    + chrono::Duration::seconds(policy.execution.flaky_quarantine_sec);
                row.quarantined_until_ts = Some(ts.to_rfc3339());
                quarantined_count += 1;
            } else if res.ok {
                row.quarantined_until_ts = None;
            }
            if res.ok {
                row.last_pass_ts = row.last_run_ts.clone();
            } else {
                row.last_fail_ts = row.last_run_ts.clone();
            }
        }

        executed_status.insert(
            test.id.clone(),
            if res.ok {
                "pass".to_string()
            } else {
                "fail".to_string()
            },
        );

        results.push(json!({
            "id": test.id,
            "command": test.command,
            "critical": test.critical,
            "guard_ok": guard.ok,
            "guard_reason": guard.reason,
            "guard_files": guard.files,
            "ok": res.ok,
            "exit_code": res.exit_code,
            "duration_ms": res.duration_ms,
            "signal": res.signal,
            "timed_out": res.timed_out,
            "retried": retried,
            "flaky": flaky,
            "quarantined_until_ts": status.tests.get(&test.id).and_then(|row| row.quarantined_until_ts.clone()),
            "stdout_excerpt": res.stdout_excerpt,
            "stderr_excerpt": res.stderr_excerpt
        }));
    }
    phase_ms["execute_ms"] = json!(execute_started.elapsed().as_millis());

    let run_ts = now_iso();
    for module in status.modules.values_mut() {
        let ids = module.mapped_test_ids.clone();
        if ids.is_empty() {
            continue;
        }
        let has_executed = ids.iter().any(|id| executed_status.contains_key(id));
        if !has_executed {
            continue;
        }
        module.last_test_ts = Some(run_ts.clone());
        let fail = ids.iter().any(|id| {
            executed_status
                .get(id)
                .map(|v| v == "fail")
                .unwrap_or(false)
        });
        let fresh_pass = !ids.is_empty()
            && ids.iter().all(|id| {
                executed_status
                    .get(id)
                    .map(|v| v == "pass")
                    .unwrap_or(false)
            });
        if fail {
            module.last_fail_ts = Some(run_ts.clone());
        }
        if fresh_pass {
            module.last_pass_ts = Some(run_ts.clone());
            if module.changed {
                module.changed = false;
            }
        }
    }

    update_module_check_states(&mut status);
    status.updated_at = Some(run_ts.clone());
    status.last_run = Some(run_ts.clone());
    let _ = write_json_atomic(
        &paths.status_path,
        &serde_json::to_value(&status).unwrap_or(Value::Null),
    );

    let passed = results
        .iter()
        .filter(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let failed = results.len().saturating_sub(passed);
    let untested = status.modules.values().filter(|m| m.untested).count();

    phase_ms["total_ms"] = json!(run_start.elapsed().as_millis());

    let claim_evidence = vec![
        json!({
            "id": "selection_scope",
            "claim": "test_selection_respects_scope",
            "evidence": {
                "scope": scope,
                "selected_tests": results.len(),
                "queued_candidates": prioritized.len()
            }
        }),
        json!({
            "id": "execution_outcome",
            "claim": "run_outcome_matches_observed_test_results",
            "evidence": {
                "passed": passed,
                "failed": failed,
                "guard_blocked": guard_blocked,
                "untested_modules": untested
            }
        }),
    ];

    let persona_lenses = json!({
        "operator": {
            "attention": if failed > 0 { "incident" } else { "maintenance" },
            "guard_blocked": guard_blocked
        },
        "skeptic": {
            "confidence": if failed == 0 { 0.92 } else { 0.58 },
            "flaky_tests": flaky_count,
            "newly_quarantined_tests": quarantined_count
        }
    });

    let mut out = json!({
        "ok": if strict { failed == 0 && untested == 0 } else { failed == 0 },
        "type": "autotest_run",
        "ts": run_ts,
        "scope": scope,
        "strict": strict,
        "synced": sync_out,
        "selected_tests": results.len(),
        "queued_candidates": prioritized.len(),
        "selection_preview": selection_preview,
        "passed": passed,
        "failed": failed,
        "guard_blocked": guard_blocked,
        "flaky_tests": flaky_count,
        "newly_quarantined_tests": quarantined_count,
        "untested_modules": untested,
        "external_health": external_health,
        "sleep_window_ok": sleep_gate,
        "resource_guard": resources,
        "spine_hot": spine_hot,
        "run_timeout_ms": run_timeout_ms,
        "phase_ms": phase_ms,
        "results": results.iter().take(300).cloned().collect::<Vec<_>>(),
        "claim_evidence": claim_evidence,
        "persona_lenses": persona_lenses,
        "pain_signal": Value::Null
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));

    let _ = write_json_atomic(&paths.latest_path, &out);
    let _ = append_jsonl(
        &paths.runs_dir.join(format!("{}.jsonl", &run_ts[..10])),
        &out,
    );

    if failed > 0 || untested > 0 || guard_blocked > 0 || flaky_count > 0 {
        let _ = append_jsonl(
            &paths.events_path,
            &json!({
                "ts": run_ts,
                "type": "autotest_alert",
                "severity": if failed > 0 || guard_blocked > 0 { "error" } else { "warn" },
                "alert_kind": if guard_blocked > 0 {
                    "guard_blocked"
                } else if failed > 0 {
                    "test_failures"
                } else if flaky_count > 0 {
                    "flaky_tests"
                } else {
                    "untested_modules"
                },
                "failed": failed,
                "guard_blocked": guard_blocked,
                "flaky_tests": flaky_count,
                "untested_modules": untested,
                "scope": scope
            }),
        );
    }

    out
}


fn cmd_report(root: &Path, cli: &CliArgs, policy: &Policy, paths: &RuntimePaths) -> Value {
    let token = cli
        .positional
        .get(1)
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "latest".to_string());
    let write = to_bool(cli.flags.get("write").map(String::as_str), true);
    let mut status = load_status(paths);

    let latest_run = read_json(&paths.latest_path);
    let ts = now_iso();
    let date = if token == "latest" {
        ts[..10].to_string()
    } else {
        token
    };

    let modules = status.modules.values().cloned().collect::<Vec<_>>();
    let tests = status.tests.values().cloned().collect::<Vec<_>>();
    let external_health = summarize_external_health(paths, policy);

    let mut untested = modules
        .iter()
        .filter(|m| m.untested)
        .cloned()
        .collect::<Vec<_>>();
    untested.sort_by(|a, b| a.path.cmp(&b.path));
    untested.truncate(policy.alerts.max_untested_in_report);

    let mut red_modules = modules
        .iter()
        .filter(|m| m.health_state.as_deref() == Some("red"))
        .cloned()
        .collect::<Vec<_>>();
    red_modules.sort_by(|a, b| a.path.cmp(&b.path));
    red_modules.truncate(policy.alerts.max_untested_in_report);

    let mut failed_tests = tests
        .iter()
        .filter(|t| t.last_status == "fail")
        .cloned()
        .collect::<Vec<_>>();
    failed_tests.sort_by(|a, b| a.command.cmp(&b.command));
    failed_tests.truncate(policy.alerts.max_failed_in_report);

    let checked_modules = modules.iter().filter(|m| m.checked).count();
    let green_modules = modules
        .iter()
        .filter(|m| m.health_state.as_deref() == Some("green"))
        .count();
    let changed_modules = modules.iter().filter(|m| m.changed).count();

    let mut lines = vec![
        "# Autotest Report".to_string(),
        "".to_string(),
        format!("- Generated: {ts}"),
        format!("- Date: {date}"),
        format!("- Modules: {}", modules.len()),
        format!("- Checked: {checked_modules}"),
        format!("- Green Modules: {green_modules}"),
        format!("- Red Modules: {}", red_modules.len()),
        format!("- Changed/Pending: {changed_modules}"),
        format!("- Untested Modules: {}", untested.len()),
        format!("- Failed Tests: {}", failed_tests.len()),
    ];

    if latest_run.is_object() {
        lines.push(format!(
            "- Last Run Scope: {}",
            latest_run
                .get("scope")
                .and_then(Value::as_str)
                .unwrap_or("n/a")
        ));
        lines.push(format!(
            "- Last Run Passed/Failed: {}/{}",
            latest_run
                .get("passed")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            latest_run
                .get("failed")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ));
    }

    lines.extend(["".to_string(), "## Red Modules (Need Help)".to_string()]);
    if red_modules.is_empty() {
        lines.push("- None".to_string());
    } else {
        for module in &red_modules {
            lines.push(format!("- {}", module.path));
            lines.push(format!(
                "  - reason: {}",
                module
                    .health_reason
                    .clone()
                    .unwrap_or_else(|| "failing_or_guard_blocked_test".to_string())
            ));
        }
    }

    lines.extend(["".to_string(), "## Failed Tests".to_string()]);
    if failed_tests.is_empty() {
        lines.push("- None".to_string());
    } else {
        for test in &failed_tests {
            let label = test.path.clone().unwrap_or_else(|| test.command.clone());
            lines.push(format!("- {label}"));
            if let Some(stderr) = &test.last_stderr_excerpt {
                lines.push(format!("  - stderr: {stderr}"));
            }
        }
    }

    lines.extend([
        "".to_string(),
        "## External Health Signals".to_string(),
        format!(
            "- Total Signals: {}",
            external_health
                .get("total")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- High/Critical: {}",
            external_health
                .get("high_or_critical")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
    ]);

    lines.extend(["".to_string(), "## Untested Modules".to_string()]);
    if untested.is_empty() {
        lines.push("- None".to_string());
    } else {
        for module in &untested {
            lines.push(format!("- {}", module.path));
            if module.changed {
                lines.push("  - reason: changed module with no mapped tests".to_string());
            } else if module.is_new {
                lines.push("  - reason: new module with no mapped tests".to_string());
            } else {
                lines.push("  - reason: no mapped tests".to_string());
            }
        }
    }

    let markdown = format!("{}\n", lines.join("\n"));
    let out_path = paths.reports_dir.join(format!("{date}.md"));
    if write {
        let _ = ensure_dir(&paths.reports_dir);
        let _ = fs::write(&out_path, markdown);
    }

    let mut out = json!({
        "ok": true,
        "type": "autotest_report",
        "ts": ts,
        "date": date,
        "modules_total": modules.len(),
        "modules_checked": checked_modules,
        "modules_green": green_modules,
        "modules_red": red_modules.len(),
        "modules_changed": changed_modules,
        "untested_modules": untested.len(),
        "failed_tests": failed_tests.len(),
        "external_health": external_health,
        "output_path": if write { Value::String(rel_path(root, &out_path)) } else { Value::Null },
        "write": write,
        "claim_evidence": [
            {
                "id": "report_composition",
                "claim": "report_counts_match_status_snapshot",
                "evidence": {
                    "modules_total": modules.len(),
                    "tests_total": tests.len(),
                    "failed_tests": failed_tests.len()
                }
            }
        ],
        "persona_lenses": {
            "operator": {
                "risk_focus": if failed_tests.is_empty() { "coverage" } else { "stability" }
            }
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));

    status.last_report = Some(now_iso());
    let _ = write_json_atomic(
        &paths.status_path,
        &serde_json::to_value(status).unwrap_or(Value::Null),
    );
    let _ = write_json_atomic(&paths.latest_path, &out);
    let _ = append_jsonl(
        &paths.runs_dir.join(format!("{}.jsonl", &now_iso()[..10])),
        &out,
    );

    out
}

fn cmd_status(root: &Path, policy: &Policy, paths: &RuntimePaths) -> Value {
    let status = load_status(paths);
    let modules = status.modules.values().collect::<Vec<_>>();
    let tests = status.tests.values().collect::<Vec<_>>();
    let external_health = summarize_external_health(paths, policy);

    let mut out = json!({
        "ok": true,
        "type": "autotest_status",
        "ts": now_iso(),
        "policy_version": policy.version,
        "modules_total": modules.len(),
        "modules_checked": modules.iter().filter(|m| m.checked).count(),
        "modules_green": modules.iter().filter(|m| m.health_state.as_deref() == Some("green")).count(),
        "modules_red": modules.iter().filter(|m| m.health_state.as_deref() == Some("red")).count(),
        "modules_pending": modules.iter().filter(|m| m.health_state.as_deref() == Some("pending")).count(),
        "modules_changed": modules.iter().filter(|m| m.changed).count(),
        "untested_modules": modules.iter().filter(|m| m.untested).count(),
        "tests_total": tests.len(),
        "tests_failed": tests.iter().filter(|t| t.last_status == "fail").count(),
        "tests_flaky": tests.iter().filter(|t| t.last_flaky.unwrap_or(false)).count(),
        "tests_quarantined": tests.iter().filter(|t| {
            t.quarantined_until_ts
                .as_deref()
                .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
                .map(|v| v.timestamp_millis() > chrono::Utc::now().timestamp_millis())
                .unwrap_or(false)
        }).count(),
        "tests_passed": tests.iter().filter(|t| t.last_status == "pass").count(),
        "tests_untested": tests.iter().filter(|t| t.last_status == "untested").count(),
        "external_health": external_health,
        "last_sync": status.last_sync,
        "last_run": status.last_run,
        "last_report": status.last_report,
        "status_path": rel_path(root, &paths.status_path),
        "registry_path": rel_path(root, &paths.registry_path),
        "claim_evidence": [
            {
                "id": "status_snapshot",
                "claim": "status_is_derived_from_current_registry",
                "evidence": {
                    "modules_total": modules.len(),
                    "tests_total": tests.len()
                }
            }
        ],
        "persona_lenses": {
            "auditor": {
                "coverage_gap": modules.iter().filter(|m| m.untested).count()
            }
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn cmd_daemon(root: &Path, cli: &CliArgs, policy: &Policy, paths: &RuntimePaths) -> Value {
    let interval_sec = clamp_i64(
        cli.flags.get("interval-sec").map(String::as_str),
        20,
        24 * 60 * 60,
        300,
    );
    let max_cycles = clamp_i64(
        cli.flags.get("max-cycles").map(String::as_str),
        0,
        1_000_000,
        0,
    );
    let jitter_sec = clamp_i64(cli.flags.get("jitter-sec").map(String::as_str), 0, 600, 0);
    let scope = cli
        .flags
        .get("scope")
        .map(String::as_str)
        .filter(|s| ["critical", "changed", "all"].contains(s))
        .unwrap_or(policy.execution.default_scope.as_str())
        .to_string();
    let strict = to_bool(
        cli.flags.get("strict").map(String::as_str),
        policy.execution.strict,
    );
    let max_tests = clamp_i64(
        cli.flags.get("max-tests").map(String::as_str),
        1,
        500,
        policy.execution.max_tests_per_run as i64,
    );

    let mut cycles = 0i64;
    let mut last: Option<Value>;

    loop {
        cycles += 1;
        let run_cli = CliArgs {
            positional: vec!["run".to_string()],
            flags: HashMap::from([
                ("scope".to_string(), scope.clone()),
                (
                    "strict".to_string(),
                    if strict { "1" } else { "0" }.to_string(),
                ),
                ("max-tests".to_string(), max_tests.to_string()),
                ("sleep-only".to_string(), "1".to_string()),
            ]),
        };
        let run_out = cmd_run(root, &run_cli, policy, paths);

        let report_cli = CliArgs {
            positional: vec!["report".to_string(), "latest".to_string()],
            flags: HashMap::from([("write".to_string(), "1".to_string())]),
        };
        let report_out = cmd_report(root, &report_cli, policy, paths);

        last = Some(json!({
            "run": run_out,
            "report": report_out
        }));

        if max_cycles > 0 && cycles >= max_cycles {
            break;
        }

        let jitter = if jitter_sec > 0 {
            (chrono::Utc::now().timestamp() % (jitter_sec + 1)).abs()
        } else {
            0
        };
        thread::sleep(Duration::from_secs((interval_sec + jitter) as u64));
    }

    let mut out = json!({
        "ok": true,
        "type": "autotest_daemon",
        "ts": now_iso(),
        "cycles": cycles,
        "interval_sec": interval_sec,
        "jitter_sec": jitter_sec,
        "scope": scope,
        "strict": strict,
        "max_tests": max_tests,
        "last": last.unwrap_or(Value::Null),
        "claim_evidence": [
            {
                "id": "daemon_cycles",
                "claim": "daemon_executed_expected_cycle_pattern",
                "evidence": {
                    "cycles": cycles,
                    "interval_sec": interval_sec
                }
            }
        ],
        "persona_lenses": {
            "operator": {
                "mode": "background"
            }
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn cli_failure_receipt(cmd: &str, error: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "autotest_cli_error",
        "ts": now_iso(),
        "command": cmd,
        "error": error,
        "exit_code": code,
        "claim_evidence": [
            {
                "id": "fail_closed_cli",
                "claim": "controller_cli_failures_emit_deterministic_receipts",
                "evidence": {
                    "command": cmd,
                    "error": error
                }
            }
        ],
        "persona_lenses": {
            "operator": {
                "mode": "cli",
                "exit_code": code
            },
            "auditor": {
                "deterministic_receipt": true
            }
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops autotest-controller sync [--policy=path] [--strict=1|0]");
    println!("  protheus-ops autotest-controller run [--policy=path] [--scope=critical|changed|all] [--max-tests=N] [--strict=1|0] [--sleep-only=1|0] [--force=1|0] [--run-timeout-ms=N]");
    println!("  protheus-ops autotest-controller report [YYYY-MM-DD|latest] [--policy=path] [--write=1|0]");
    println!("  protheus-ops autotest-controller status [--policy=path]");
    println!("  protheus-ops autotest-controller pulse [--policy=path] [--scope=changed|critical|all] [--max-tests=N] [--strict=1|0] [--force=1|0] [--run-timeout-ms=N]");
    println!("  protheus-ops autotest-controller daemon [--policy=path] [--interval-sec=N] [--max-cycles=N] [--scope=changed|critical|all] [--max-tests=N] [--strict=1|0] [--run-timeout-ms=N]");
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cli = parse_cli(argv);
    let cmd = cli
        .positional
        .first()
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_default();

    if cmd.is_empty() || matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let mut paths = runtime_paths(root);
    if let Some(p) = cli.flags.get("policy") {
        let pb = PathBuf::from(p);
        paths.policy_path = if pb.is_absolute() { pb } else { root.join(pb) };
    }
    let policy = load_policy(root, &paths.policy_path);

    if let Err(err) = ensure_state_dirs(&paths) {
        print_json_line(&cli_failure_receipt(&cmd, &err, 1));
        return 1;
    }

    if !policy.enabled && cmd != "status" {
        print_json_line(&json!({
            "ok": true,
            "type": "autotest",
            "ts": now_iso(),
            "disabled": true,
            "reason": "policy_disabled"
        }));
        return 0;
    }

    let out = match cmd.as_str() {
        "sync" => sync_state(root, &paths, &policy),
        "run" => cmd_run(root, &cli, &policy, &paths),
        "report" => cmd_report(root, &cli, &policy, &paths),
        "status" => cmd_status(root, &policy, &paths),
        "pulse" => {
            let run_cli = CliArgs {
                positional: vec!["run".to_string()],
                flags: {
                    let mut flags = cli.flags.clone();
                    flags.insert("sleep-only".to_string(), "1".to_string());
                    flags
                },
            };
            let run_out = cmd_run(root, &run_cli, &policy, &paths);
            let report_cli = CliArgs {
                positional: vec!["report".to_string(), "latest".to_string()],
                flags: HashMap::from([("write".to_string(), "1".to_string())]),
            };
            let report_out = cmd_report(root, &report_cli, &policy, &paths);
            let mut payload = json!({
                "ok": run_out.get("ok").and_then(Value::as_bool).unwrap_or(false)
                    && report_out.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "type": "autotest_pulse",
                "ts": now_iso(),
                "run": run_out,
                "report": report_out,
                "claim_evidence": [
                    {
                        "id": "pulse_pair",
                        "claim": "pulse_runs_and_reports_in_sequence",
                        "evidence": {
                            "run_ok": run_out.get("ok").and_then(Value::as_bool).unwrap_or(false),
                            "report_ok": report_out.get("ok").and_then(Value::as_bool).unwrap_or(false)
                        }
                    }
                ],
                "persona_lenses": {
                    "operator": {
                        "mode": "pulse"
                    }
                }
            });
            payload["receipt_hash"] = Value::String(receipt_hash(&payload));
            payload
        }
        "daemon" => cmd_daemon(root, &cli, &policy, &paths),
        _ => {
            usage();
            print_json_line(&cli_failure_receipt(&cmd, "unknown_command", 2));
            return 2;
        }
    };

    let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
    print_json_line(&out);
    if ok {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parity_fixture_score_prefers_layer_and_tokens() {
        let policy = default_policy();
        let module = ModuleCandidate {
            id: "mod_1".to_string(),
            path: "client/runtime/systems/ops/autotest_controller.ts".to_string(),
            abs_path: PathBuf::from("client/runtime/systems/ops/autotest_controller.ts"),
            basename: "autotest_controller".to_string(),
        };
        let test = TestCandidate {
            id: "tst_1".to_string(),
            kind: "node_test".to_string(),
            path: "tests/client-memory-tools/autotest_controller.test.ts".to_string(),
            command: "node tests/client-memory-tools/autotest_controller.test.ts".to_string(),
            stem: "autotest_controller.test".to_string(),
        };
        let score = score_module_test_pair(&module, &test, &policy);
        assert!(score >= policy.min_match_score);
    }

    #[test]
    fn deterministic_receipt_hash_for_fixture() {
        let payload = json!({
            "ok": true,
            "type": "autotest_status",
            "modules_total": 2,
            "tests_total": 4,
            "claim_evidence": [{"id":"c1","claim":"x","evidence":{"a":1}}]
        });
        let h1 = receipt_hash(&payload);
        let h2 = receipt_hash(&payload);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn cli_failure_receipt_includes_hash_and_invariants() {
        let out = cli_failure_receipt("daemonx", "unknown_command", 2);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("autotest_cli_error")
        );
        assert!(out.get("claim_evidence").is_some());
        assert!(out.get("persona_lenses").is_some());

        let expected_hash = out
            .get("receipt_hash")
            .and_then(Value::as_str)
            .expect("hash")
            .to_string();
        let mut unhashed = out.clone();
        unhashed
            .as_object_mut()
            .expect("object")
            .remove("receipt_hash");
        assert_eq!(receipt_hash(&unhashed), expected_hash);
    }

    #[test]
    fn parse_cli_preserves_run_command() {
        let cli = parse_cli(&["run".to_string()]);
        assert_eq!(cli.positional.first().map(String::as_str), Some("run"));
    }

    #[test]
    fn parse_cli_honors_double_dash_passthrough() {
        let cli = parse_cli(&[
            "run".to_string(),
            "--scope=web".to_string(),
            "--".to_string(),
            "--grep".to_string(),
            "web fetch".to_string(),
        ]);
        assert_eq!(cli.flags.get("scope").map(String::as_str), Some("web"));
        assert_eq!(
            cli.positional,
            vec![
                "run".to_string(),
                "--grep".to_string(),
                "web fetch".to_string(),
            ]
        );
    }
}

