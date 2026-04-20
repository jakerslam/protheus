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

