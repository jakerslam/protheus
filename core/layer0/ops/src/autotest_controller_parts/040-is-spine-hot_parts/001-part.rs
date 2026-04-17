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
