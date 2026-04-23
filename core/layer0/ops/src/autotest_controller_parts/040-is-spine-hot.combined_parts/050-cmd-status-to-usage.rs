
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
    println!("  infring-ops autotest-controller sync [--policy=path] [--strict=1|0]");
    println!("  infring-ops autotest-controller run [--policy=path] [--scope=critical|changed|all] [--max-tests=N] [--strict=1|0] [--sleep-only=1|0] [--force=1|0] [--run-timeout-ms=N]");
    println!("  infring-ops autotest-controller report [YYYY-MM-DD|latest] [--policy=path] [--write=1|0]");
    println!("  infring-ops autotest-controller status [--policy=path]");
    println!("  infring-ops autotest-controller pulse [--policy=path] [--scope=changed|critical|all] [--max-tests=N] [--strict=1|0] [--force=1|0] [--run-timeout-ms=N]");
    println!("  infring-ops autotest-controller daemon [--policy=path] [--interval-sec=N] [--max-cycles=N] [--scope=changed|critical|all] [--max-tests=N] [--strict=1|0] [--run-timeout-ms=N]");
}
