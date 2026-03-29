fn score_module_test_pair(module: &ModuleCandidate, test: &TestCandidate, policy: &Policy) -> i64 {
    let module_base = normalize_token(&module.basename, 120);
    let test_stem = normalize_token(&test.stem, 180);
    let mut score = 0i64;

    if test_stem.contains(&module_base) || module_base.contains(&test_stem) {
        score += policy.basename_contains_score;
    }

    let module_tokens = tokenize_name(&module_base, policy.min_token_len);
    let test_tokens = tokenize_name(&test_stem, policy.min_token_len)
        .into_iter()
        .collect::<HashSet<_>>();
    for tok in module_tokens {
        if test_tokens.contains(&tok) {
            score += policy.shared_token_score;
        }
    }

    let mod_layer = layer_hint(&module.path);
    if !mod_layer.is_empty() && test_stem.contains(&mod_layer) {
        score += policy.layer_hint_score;
    }

    score
}

fn map_module_tests(
    modules: &[ModuleCandidate],
    tests: &[TestCandidate],
    policy: &Policy,
) -> HashMap<String, Vec<String>> {
    let by_path = tests
        .iter()
        .map(|t| (t.path.clone(), t.id.clone()))
        .collect::<HashMap<_, _>>();

    let mut mapping = HashMap::new();
    for module in modules {
        let mut test_ids = HashSet::new();

        for (prefix, test_paths) in &policy.explicit_prefix_maps {
            if !module.path.starts_with(prefix) {
                continue;
            }
            for test_path in test_paths {
                if let Some(id) = by_path.get(test_path) {
                    test_ids.insert(id.clone());
                }
            }
        }

        for test in tests {
            let score = score_module_test_pair(module, test, policy);
            if score >= policy.min_match_score {
                test_ids.insert(test.id.clone());
            }
        }

        let mut ids = test_ids.into_iter().collect::<Vec<_>>();
        ids.sort();
        mapping.insert(module.path.clone(), ids);
    }

    mapping
}

fn emit_alerts(paths: &RuntimePaths, status: &mut StatusState, alerts: Vec<Value>) -> Vec<Value> {
    let mut emitted = Vec::new();
    for alert in alerts {
        let signature = alert
            .get("signature")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if signature.is_empty() {
            continue;
        }
        if status.alerts.emitted_signatures.contains_key(&signature) {
            continue;
        }
        status
            .alerts
            .emitted_signatures
            .insert(signature, now_iso());
        let _ = append_jsonl(&paths.events_path, &alert);
        emitted.push(alert);
    }
    status.alerts.latest = emitted.iter().take(200).cloned().collect();
    emitted
}

fn update_module_check_states(status: &mut StatusState) {
    let test_map = status.tests.clone();
    for module in status.modules.values_mut() {
        let ids = module.mapped_test_ids.clone();
        let all_pass = !ids.is_empty()
            && ids.iter().all(|id| {
                test_map
                    .get(id)
                    .map(|t| t.last_status == "pass")
                    .unwrap_or(false)
            });
        let has_fail = ids.iter().any(|id| {
            test_map.get(id).is_some_and(|t| {
                t.last_status == "fail" || t.last_guard.as_ref().map(|g| !g.ok).unwrap_or(false)
            })
        });

        module.checked = all_pass && !module.changed;
        module.untested = ids.is_empty();
        if module.checked && module.last_pass_ts.is_none() {
            module.last_pass_ts = Some(now_iso());
        }

        if module.untested {
            module.health_state = Some("untested".to_string());
            module.health_reason = Some("no_mapped_tests".to_string());
        } else if has_fail {
            module.health_state = Some("red".to_string());
            module.health_reason = Some("failing_or_guard_blocked_test".to_string());
        } else if module.changed {
            module.health_state = Some("pending".to_string());
            module.health_reason = Some("changed_waiting_for_fresh_pass".to_string());
        } else if module.checked {
            module.health_state = Some("green".to_string());
            module.health_reason = Some("all_mapped_tests_passing".to_string());
        } else {
            module.health_state = Some("yellow".to_string());
            module.health_reason = Some("partial_or_stale_coverage".to_string());
        }
    }
}

fn sync_state(root: &Path, paths: &RuntimePaths, policy: &Policy) -> Value {
    let prev = load_status(paths);
    let modules = module_candidates(root, paths, policy);
    let tests = test_candidates(root, paths, policy);
    let mapping = map_module_tests(&modules, &tests, policy);
    let now = now_iso();

    let mut next_modules = HashMap::<String, ModuleRow>::new();
    let mut alerts = Vec::<Value>::new();

    let mut changed_count = 0usize;
    let mut new_count = 0usize;
    let mut untested_count = 0usize;

    for module in &modules {
        let fp = sha256_file(&module.abs_path);
        let prev_row = prev.modules.get(&module.path);
        let mapped_tests = mapping.get(&module.path).cloned().unwrap_or_default();
        let has_tests = !mapped_tests.is_empty();
        let is_new = prev_row.is_none();
        let fingerprint_changed = prev_row.map(|r| r.fingerprint != fp).unwrap_or(true);
        let pending_prior = prev_row.map(|r| r.changed && !r.checked).unwrap_or(false);
        let changed = fingerprint_changed || pending_prior;

        if is_new {
            new_count += 1;
        }
        if changed {
            changed_count += 1;
        }
        if !has_tests {
            untested_count += 1;
        }

        let checked = if changed {
            false
        } else {
            prev_row
                .map(|r| r.checked && r.mapped_test_count == mapped_tests.len())
                .unwrap_or(false)
        };

        let row = ModuleRow {
            id: prev_row
                .map(|r| r.id.clone())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| module.id.clone()),
            path: module.path.clone(),
            fingerprint: fp.clone(),
            checked,
            changed,
            is_new,
            untested: !has_tests,
            mapped_test_ids: mapped_tests.clone(),
            mapped_test_count: mapped_tests.len(),
            last_change_ts: if fingerprint_changed {
                Some(now.clone())
            } else {
                prev_row.and_then(|r| r.last_change_ts.clone())
            },
            last_test_ts: prev_row.and_then(|r| r.last_test_ts.clone()),
            last_pass_ts: prev_row.and_then(|r| r.last_pass_ts.clone()),
            last_fail_ts: prev_row.and_then(|r| r.last_fail_ts.clone()),
            seed_fields: SeedFields {
                owner: prev_row.and_then(|r| r.seed_fields.owner.clone()),
                priority: prev_row
                    .and_then(|r| r.seed_fields.priority.clone())
                    .or_else(|| Some("normal".to_string())),
                notes: prev_row.and_then(|r| r.seed_fields.notes.clone()),
            },
            health_state: prev_row.and_then(|r| r.health_state.clone()),
            health_reason: prev_row.and_then(|r| r.health_reason.clone()),
        };

        if policy.alerts.emit_untested && !has_tests {
            let should_emit = is_new
                || (policy.alerts.emit_changed_without_tests && changed)
                || prev_row.map(|r| !r.untested).unwrap_or(true);
            if should_emit {
                alerts.push(json!({
                    "ts": now,
                    "type": "autotest_alert",
                    "severity": "warn",
                    "alert_kind": "untested_module",
                    "module_path": module.path,
                    "reason": if changed { "changed_module_without_tests" } else { "module_without_tests" },
                    "signature": stable_id(&format!("untested|{}|{}", module.path, fp), "alert")
                }));
            }
        }

        next_modules.insert(module.path.clone(), row);
    }

    let mut next_tests = HashMap::<String, TestRow>::new();
    for test in &tests {
        let prev_row = prev.tests.get(&test.id);
        next_tests.insert(
            test.id.clone(),
            TestRow {
                id: test.id.clone(),
                kind: test.kind.clone(),
                path: Some(test.path.clone()),
                command: test.command.clone(),
                critical: false,
                last_status: prev_row
                    .map(|r| r.last_status.clone())
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| "untested".to_string()),
                last_exit_code: prev_row.and_then(|r| r.last_exit_code),
                last_run_ts: prev_row.and_then(|r| r.last_run_ts.clone()),
                last_duration_ms: prev_row.and_then(|r| r.last_duration_ms),
                last_stdout_excerpt: prev_row.and_then(|r| r.last_stdout_excerpt.clone()),
                last_stderr_excerpt: prev_row.and_then(|r| r.last_stderr_excerpt.clone()),
                last_guard: prev_row.and_then(|r| r.last_guard.clone()),
                last_retry_count: prev_row.and_then(|r| r.last_retry_count),
                last_flaky: prev_row.and_then(|r| r.last_flaky),
                consecutive_flaky: prev_row.and_then(|r| r.consecutive_flaky),
                quarantined_until_ts: prev_row.and_then(|r| r.quarantined_until_ts.clone()),
                last_pass_ts: prev_row.and_then(|r| r.last_pass_ts.clone()),
                last_fail_ts: prev_row.and_then(|r| r.last_fail_ts.clone()),
            },
        );
    }

    for command in &policy.critical_commands {
        let id = stable_id(&format!("critical|{command}"), "tst");
        let prev_row = prev.tests.get(&id);
        next_tests.insert(
            id.clone(),
            TestRow {
                id,
                kind: "shell_command".to_string(),
                path: None,
                command: command.clone(),
                critical: true,
                last_status: prev_row
                    .map(|r| r.last_status.clone())
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| "untested".to_string()),
                last_exit_code: prev_row.and_then(|r| r.last_exit_code),
                last_run_ts: prev_row.and_then(|r| r.last_run_ts.clone()),
                last_duration_ms: prev_row.and_then(|r| r.last_duration_ms),
                last_stdout_excerpt: prev_row.and_then(|r| r.last_stdout_excerpt.clone()),
                last_stderr_excerpt: prev_row.and_then(|r| r.last_stderr_excerpt.clone()),
                last_guard: prev_row.and_then(|r| r.last_guard.clone()),
                last_retry_count: prev_row.and_then(|r| r.last_retry_count),
                last_flaky: prev_row.and_then(|r| r.last_flaky),
                consecutive_flaky: prev_row.and_then(|r| r.consecutive_flaky),
                quarantined_until_ts: prev_row.and_then(|r| r.quarantined_until_ts.clone()),
                last_pass_ts: prev_row.and_then(|r| r.last_pass_ts.clone()),
                last_fail_ts: prev_row.and_then(|r| r.last_fail_ts.clone()),
            },
        );
    }

    let registry = json!({
        "ok": true,
        "type": "autotest_registry",
        "ts": now,
        "policy_version": policy.version,
        "module_root": rel_path(root, &paths.module_root),
        "test_root": rel_path(root, &paths.test_root),
        "modules": modules.iter().map(|m| json!({
            "id": m.id,
            "path": m.path,
            "mapped_test_ids": mapping.get(&m.path).cloned().unwrap_or_default()
        })).collect::<Vec<_>>(),
        "tests": next_tests.values().map(|t| json!({
            "id": t.id,
            "kind": t.kind,
            "path": t.path,
            "command": t.command,
            "critical": t.critical
        })).collect::<Vec<_>>()
    });

    let mut next_status = StatusState {
        version: "1.0".to_string(),
        updated_at: Some(now.clone()),
        modules: next_modules,
        tests: next_tests,
        alerts: prev.alerts,
        last_sync: Some(now.clone()),
        last_run: prev.last_run,
        last_report: prev.last_report,
    };
    update_module_check_states(&mut next_status);

    let emitted_alerts = emit_alerts(paths, &mut next_status, alerts);

    let _ = write_json_atomic(&paths.registry_path, &registry);
    let _ = write_json_atomic(
        &paths.status_path,
        &serde_json::to_value(&next_status).unwrap_or(Value::Null),
    );

    let claims = vec![
        json!({
            "id": "modules_scanned",
            "claim": "module_registry_is_current",
            "evidence": {
                "modules": modules.len(),
                "changed_modules": changed_count,
                "new_modules": new_count
            }
        }),
        json!({
            "id": "mapping_computed",
            "claim": "module_to_test_mapping_available",
            "evidence": {
                "tests_discovered": next_status.tests.len(),
                "untested_modules": untested_count
            }
        }),
    ];

    let persona_lenses = json!({
        "operator": {
            "focus": if untested_count > 0 { "coverage_gap" } else { "execution" },
            "risk_level": if untested_count > 0 { "medium" } else { "low" }
        },
        "auditor": {
            "alerts_emitted": emitted_alerts.len(),
            "deterministic_registry": true
        }
    });

    let mut out = json!({
        "ok": true,
        "type": "autotest_sync",
        "ts": now,
        "changed_modules": changed_count,
        "new_modules": new_count,
        "untested_modules": untested_count,
        "tests_discovered": next_status.tests.len(),
        "emitted_alerts": emitted_alerts.len(),
        "registry_path": rel_path(root, &paths.registry_path),
        "status_path": rel_path(root, &paths.status_path),
        "claim_evidence": claims,
        "persona_lenses": persona_lenses
    });
    let hash = receipt_hash(&out);
    out["receipt_hash"] = Value::String(hash);

    let _ = write_json_atomic(&paths.latest_path, &out);
    let _ = append_jsonl(
        &paths.runs_dir.join(format!("{}.jsonl", &now[..10])),
        &json!({
            "ts": now,
            "type": "autotest_sync",
            "changed_modules": changed_count,
            "new_modules": new_count,
            "untested_modules": untested_count,
            "emitted_alerts": emitted_alerts.len(),
            "receipt_hash": out.get("receipt_hash").cloned().unwrap_or(Value::Null)
        }),
    );

    out
}

fn in_sleep_window(policy: &Policy) -> bool {
    let hour = chrono::Local::now().hour();
    let start = policy.sleep_window_start_hour;
    let end = policy.sleep_window_end_hour;
    if start == end {
        return true;
    }
    if start < end {
        hour >= start && hour < end
    } else {
        hour >= start || hour < end
    }
}

fn runtime_resource_within(policy: &Policy) -> Value {
    let mut system = System::new_all();
    system.refresh_memory();
    let rss_mb = (system.used_memory() as f64) / 1024.0;
    let ok = rss_mb <= policy.runtime_guard.max_rss_mb;
    json!({
        "ok": ok,
        "rss_mb": ((rss_mb * 100.0).round() / 100.0),
        "max_rss_mb": policy.runtime_guard.max_rss_mb,
        "reason": if ok { Value::Null } else { Value::String("rss_limit_exceeded".to_string()) }
    })
}
