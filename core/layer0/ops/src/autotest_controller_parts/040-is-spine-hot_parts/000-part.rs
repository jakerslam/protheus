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
