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

