
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
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
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
