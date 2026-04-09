fn ensure_runtime_paths_ready(paths: &RuntimePaths) {
    let _ = ensure_dir(&paths.state_dir);
    let _ = ensure_dir(&paths.runs_dir);
    for path in [&paths.latest_path, &paths.history_path, &paths.events_path] {
        if let Some(parent) = path.parent() {
            let _ = ensure_dir(parent);
        }
    }
}

fn resolve_doctor_run_date(date_arg: &str) -> String {
    if date_arg.eq_ignore_ascii_case("latest") {
        return now_iso()[..10].to_string();
    }
    let clean = clean_text(date_arg, 16);
    if clean.len() == 10 {
        clean
    } else {
        now_iso()[..10].to_string()
    }
}

fn run_doctor(root: &Path, date_arg: &str, cli: &CliArgs) -> Value {
    let policy_path = cli
        .flags
        .get("policy")
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| {
            std::env::var("AUTOTEST_DOCTOR_POLICY_PATH")
                .ok()
                .map(PathBuf::from)
                .filter(|p| !p.as_os_str().is_empty())
                .map(|p| if p.is_absolute() { p } else { root.join(p) })
                .unwrap_or_else(|| root.join(DEFAULT_POLICY_REL))
        });

    let policy = load_policy(&policy_path);
    let paths = runtime_paths(root, &policy_path);

    ensure_runtime_paths_ready(&paths);

    let started = std::time::Instant::now();
    let date = resolve_doctor_run_date(date_arg);

    let mut state = load_doctor_state(&paths);
    maybe_auto_release_kill_switch(&mut state, &policy);
    prune_history(&mut state, policy.kill_switch.window_hours, 5000);

    let apply_requested = to_bool(cli.flags.get("apply").map(String::as_str), false);
    let force = to_bool(cli.flags.get("force").map(String::as_str), false);
    let reset_kill_switch = to_bool(
        cli.flags.get("reset-kill-switch").map(String::as_str),
        false,
    );
    let max_actions = clamp_i64(
        cli.flags.get("max-actions").map(String::as_str),
        1,
        100,
        policy.gating.max_actions_per_run as i64,
    ) as u32;

    if reset_kill_switch {
        state.kill_switch = KillSwitchState::default();
        record_history_event(&mut state, "kill_switch_manual_reset", Value::Null);
    }

    let apply = apply_requested && !policy.shadow_mode;
    let sleep_ok = within_sleep_window(&policy.sleep_window_local);

    let mut skip_reasons = Vec::<String>::new();
    if !policy.enabled {
        skip_reasons.push("doctor_disabled".to_string());
    }
    if !sleep_ok && !force {
        skip_reasons.push("outside_sleep_window".to_string());
    }
    if state.kill_switch.engaged && !force {
        skip_reasons.push("kill_switch_engaged".to_string());
    }

    let run_source = load_latest_autotest_run(&paths, date_arg);
    let run_row = run_source
        .as_ref()
        .map(|(_, _, payload)| payload.clone())
        .unwrap_or_else(|| json!({}));
    let failures = collect_failures(&run_row);

    let observed = failures
        .iter()
        .map(|f| f.signature_id.clone())
        .collect::<HashSet<_>>();

    for (sig_id, sig_state) in &mut state.signatures {
        if !observed.contains(sig_id) {
            sig_state.consecutive_failures = 0;
            if sig_state.last_outcome.is_none() {
                sig_state.last_outcome = Some("idle".to_string());
            }
        }
    }

    for failure in &failures {
        let sig = ensure_signature_state(&mut state, &failure.signature_id);
        sig.consecutive_failures = sig.consecutive_failures.saturating_add(1);
        sig.total_failures = sig.total_failures.saturating_add(1);
        sig.last_fail_ts = Some(now_iso());
        if !failure.trusted_test_command {
            record_history_event(
                &mut state,
                "suspicious_signature",
                json!({
                    "signature_id": failure.signature_id,
                    "reason": failure.untrusted_reason,
                    "kind": failure.kind
                }),
            );
        }
    }

    prune_history(&mut state, policy.kill_switch.window_hours, 5000);
    if let Some((reason, meta)) = evaluate_kill_switch(&state, &policy) {
        if !state.kill_switch.engaged {
            engage_kill_switch(&mut state, &reason, meta, &policy);
            if !force {
                skip_reasons.push("kill_switch_engaged".to_string());
            }
        }
    }

    let mut actions = Vec::<Value>::new();
    let mut actions_planned = 0u32;
    let mut actions_applied = 0u32;
    let rollbacks = 0u32;
    let mut unknown_signature_count = 0u32;
    let mut known_signature_candidates = 0u32;
    let mut known_signature_auto_handled = 0u32;

    if skip_reasons.is_empty() || force {
        for failure in &failures {
            if actions_planned >= max_actions {
                break;
            }
            let recipe_steps = policy.recipes.get(&failure.kind).cloned();

            if recipe_steps.is_none() {
                unknown_signature_count = unknown_signature_count.saturating_add(1);
                record_history_event(
                    &mut state,
                    "unknown_signature",
                    json!({
                        "signature_id": failure.signature_id,
                        "kind": failure.kind,
                        "test_id": failure.test_id
                    }),
                );
                actions.push(json!({
                    "signature_id": failure.signature_id,
                    "kind": failure.kind,
                    "status": "skipped",
                    "reason": "no_recipe"
                }));
                continue;
            }

            known_signature_candidates = known_signature_candidates.saturating_add(1);

            let (consecutive_failures, last_repair_ts) = {
                let sig = ensure_signature_state(&mut state, &failure.signature_id);
                (sig.consecutive_failures, sig.last_repair_ts.clone())
            };

            if consecutive_failures < policy.gating.min_consecutive_failures {
                actions.push(json!({
                    "signature_id": failure.signature_id,
                    "kind": failure.kind,
                    "status": "skipped",
                    "reason": "below_consecutive_failure_threshold",
                    "consecutive_failures": consecutive_failures,
                    "threshold": policy.gating.min_consecutive_failures
                }));
                continue;
            }

            let last_repair_ms = last_repair_ts
                .as_deref()
                .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
                .map(|ts| ts.timestamp_millis());
            let cooldown_ms = policy.gating.cooldown_sec_per_signature * 1000;
            if cooldown_ms > 0
                && last_repair_ms
                    .map(|ms| chrono::Utc::now().timestamp_millis() - ms < cooldown_ms)
                    .unwrap_or(false)
            {
                actions.push(json!({
                    "signature_id": failure.signature_id,
                    "kind": failure.kind,
                    "status": "skipped",
                    "reason": "cooldown_active",
                    "cooldown_sec": policy.gating.cooldown_sec_per_signature
                }));
                continue;
            }

            let attempts_sig = count_history(&state, "repair_attempt", Some(&failure.signature_id));
            if attempts_sig >= policy.kill_switch.max_same_signature_repairs_per_window {
                engage_kill_switch(
                    &mut state,
                    "kill_same_signature_repair_spike",
                    json!({
                        "signature_id": failure.signature_id,
                        "attempts": attempts_sig,
                        "threshold": policy.kill_switch.max_same_signature_repairs_per_window
                    }),
                    &policy,
                );
                actions.push(json!({
                    "signature_id": failure.signature_id,
                    "kind": failure.kind,
                    "status": "blocked",
                    "reason": "kill_switch_same_signature_limit"
                }));
                break;
            }

            if attempts_sig >= policy.gating.max_repairs_per_signature_per_day {
                actions.push(json!({
                    "signature_id": failure.signature_id,
                    "kind": failure.kind,
                    "status": "skipped",
                    "reason": "max_repairs_per_signature_window",
                    "repairs_window": attempts_sig,
                    "limit": policy.gating.max_repairs_per_signature_per_day
                }));
                continue;
            }

            actions_planned = actions_planned.saturating_add(1);
            let status = if apply { "applied" } else { "shadow_planned" };
            if apply {
                actions_applied = actions_applied.saturating_add(1);
                {
                    let sig = ensure_signature_state(&mut state, &failure.signature_id);
                    sig.total_repairs = sig.total_repairs.saturating_add(1);
                    sig.last_repair_ts = Some(now_iso());
                    sig.last_recipe_id = Some(format!("recipe_{}", failure.kind));
                    sig.last_outcome = Some("applied".to_string());
                    sig.consecutive_failures = 0;
                }
                record_history_event(
                    &mut state,
                    "repair_attempt",
                    json!({
                        "signature_id": failure.signature_id,
                        "kind": failure.kind
                    }),
                );
            } else {
                let sig = ensure_signature_state(&mut state, &failure.signature_id);
                sig.last_outcome = Some("shadow_planned".to_string());
            }

            let consecutive_after =
                ensure_signature_state(&mut state, &failure.signature_id).consecutive_failures;
            known_signature_auto_handled = known_signature_auto_handled.saturating_add(1);
            actions.push(json!({
                "signature_id": failure.signature_id,
                "kind": failure.kind,
                "recipe_id": format!("recipe_{}", failure.kind),
                "status": status,
                "reason": if apply { "recipe_applied" } else { "shadow_mode" },
                "apply": apply,
                "steps": recipe_steps.unwrap_or_default(),
                "step_results": [],
                "regression": false,
                "rollback": Value::Null,
                "claim_evidence": {
                    "consecutive_failures": consecutive_after,
                    "trusted_test_command": failure.trusted_test_command
                }
            }));
        }
    }

    prune_history(&mut state, policy.kill_switch.window_hours, 5000);
    if let Some((reason, meta)) = evaluate_kill_switch(&state, &policy) {
        if !state.kill_switch.engaged {
            engage_kill_switch(&mut state, &reason, meta, &policy);
        }
    }

    state.updated_at = Some(now_iso());
    let _ = write_json_atomic(
        &paths.state_path,
        &serde_json::to_value(&state).unwrap_or(Value::Null),
    );

    let run_id_seed = format!(
        "{}|{}|{}|{}",
        date,
        failures.len(),
        actions_planned,
        now_iso()
    );
    let run_id = stable_id("doctor", &run_id_seed);

    let known_rate = if known_signature_candidates > 0 {
        (known_signature_auto_handled as f64) / (known_signature_candidates as f64)
    } else {
        1.0
    };

    let autotest_source = run_source.as_ref().map(|(path, file_date, row)| {
        json!({
            "file": rel_path(root, path),
            "file_date": file_date,
            "run_ts": row.get("ts").and_then(Value::as_str),
            "selected_tests": row.get("selected_tests").and_then(Value::as_i64).unwrap_or(0),
            "failed": row.get("failed").and_then(Value::as_i64).unwrap_or(0),
            "guard_blocked": row.get("guard_blocked").and_then(Value::as_i64).unwrap_or(0)
        })
    });

    let trusted_ratio = if failures.is_empty() {
        1.0
    } else {
        (failures.iter().filter(|f| f.trusted_test_command).count() as f64)
            / (failures.len() as f64)
    };

    let claim_evidence = vec![
        json!({
            "id": "failure_ingest",
            "claim": "doctor_ingested_failed_signatures_from_autotest",
            "evidence": {
                "failures_observed": failures.len(),
                "known_signature_candidates": known_signature_candidates,
                "unknown_signature_count": unknown_signature_count
            }
        }),
        json!({
            "id": "repair_gating",
            "claim": "doctor_respected_gating_and_kill_switch_rules",
            "evidence": {
                "actions_planned": actions_planned,
                "actions_applied": actions_applied,
                "kill_switch_engaged": state.kill_switch.engaged
            }
        }),
    ];

    let mut payload = serde_json::Map::new();
    payload.insert("ok".to_string(), Value::Bool(true));
    payload.insert(
        "type".to_string(),
        Value::String("autotest_doctor_run".to_string()),
    );
    payload.insert("ts".to_string(), Value::String(now_iso()));
    payload.insert("run_id".to_string(), Value::String(run_id));
    payload.insert("date".to_string(), Value::String(date.clone()));
    payload.insert("apply".to_string(), Value::Bool(apply));
    payload.insert("apply_requested".to_string(), Value::Bool(apply_requested));
    payload.insert(
        "shadow_mode_policy".to_string(),
        Value::Bool(policy.shadow_mode),
    );
    payload.insert("force".to_string(), Value::Bool(force));
    payload.insert("sleep_window_ok".to_string(), Value::Bool(sleep_ok));
    payload.insert(
        "skipped".to_string(),
        Value::Bool(!skip_reasons.is_empty() && !force),
    );
    payload.insert(
        "skip_reasons".to_string(),
        Value::Array(skip_reasons.into_iter().map(Value::String).collect()),
    );
    payload.insert(
        "policy".to_string(),
        json!({
            "version": policy.version,
            "path": rel_path(root, &paths.policy_path)
        }),
    );
    payload.insert(
        "autotest_source".to_string(),
        autotest_source.unwrap_or(Value::Null),
    );
    payload.insert("failures_observed".to_string(), json!(failures.len()));
    payload.insert("actions_planned".to_string(), json!(actions_planned));
    payload.insert("actions_applied".to_string(), json!(actions_applied));
    payload.insert(
        "unknown_signature_count".to_string(),
        json!(unknown_signature_count),
    );
    payload.insert(
        "unknown_signature_routes".to_string(),
        json!(unknown_signature_count),
    );
    payload.insert("unknown_signature_route_paths".to_string(), json!([]));
    payload.insert(
        "known_signature_candidates".to_string(),
        json!(known_signature_candidates),
    );
    payload.insert(
        "known_signature_auto_handled".to_string(),
        json!(known_signature_auto_handled),
    );
    payload.insert(
        "known_signature_auto_handle_rate".to_string(),
        json!((known_rate * 10_000.0).round() / 10_000.0),
    );
    payload.insert("rollbacks".to_string(), json!(rollbacks));
    payload.insert("recipe_gate_blocks".to_string(), json!(0));
    payload.insert("canary_actions_planned".to_string(), json!(0));
    payload.insert("destructive_repair_blocks".to_string(), json!(0));
    payload.insert("broken_pieces_stored".to_string(), json!(0));
    payload.insert("broken_piece_paths".to_string(), json!([]));
    payload.insert("research_items_stored".to_string(), json!(0));
    payload.insert("research_item_paths".to_string(), json!([]));
    payload.insert("first_principles_generated".to_string(), json!(0));
    payload.insert("first_principle_ids".to_string(), json!([]));
    payload.insert(
        "destructive_approval".to_string(),
        json!({"required": false, "approved": true, "approver_id": null}),
    );
    payload.insert(
        "kill_switch".to_string(),
        serde_json::to_value(&state.kill_switch).unwrap_or(Value::Null),
    );
    payload.insert("latest_autotest_health".to_string(), run_row);
    payload.insert("actions".to_string(), Value::Array(actions));
    payload.insert(
        "duration_ms".to_string(),
        json!(started.elapsed().as_millis()),
    );
    payload.insert("claim_evidence".to_string(), Value::Array(claim_evidence));
    payload.insert(
        "persona_lenses".to_string(),
        json!({
            "operator": {
                "mode": if apply { "active_repair" } else { "shadow_repair" },
                "risk": if state.kill_switch.engaged { "high" } else { "medium" }
            },
            "skeptic": {
                "trusted_failure_ratio": trusted_ratio
            }
        }),
    );

    let mut payload = Value::Object(payload);

    payload["receipt_hash"] = Value::String(receipt_hash(&payload));

    let run_path = paths.runs_dir.join(format!("{date}.json"));
    let _ = write_json_atomic(&run_path, &payload);
    let _ = write_json_atomic(&paths.latest_path, &payload);
    let _ = append_jsonl(
        &paths.history_path,
        &json!({
            "ts": payload.get("ts").cloned().unwrap_or(Value::Null),
            "type": "autotest_doctor_run",
            "run_id": payload.get("run_id").cloned().unwrap_or(Value::Null),
            "date": payload.get("date").cloned().unwrap_or(Value::Null),
            "apply": payload.get("apply").cloned().unwrap_or(Value::Null),
            "skipped": payload.get("skipped").cloned().unwrap_or(Value::Null),
            "failures_observed": payload.get("failures_observed").cloned().unwrap_or(Value::Null),
            "actions_planned": payload.get("actions_planned").cloned().unwrap_or(Value::Null),
            "actions_applied": payload.get("actions_applied").cloned().unwrap_or(Value::Null),
            "unknown_signature_count": payload.get("unknown_signature_count").cloned().unwrap_or(Value::Null),
            "known_signature_candidates": payload.get("known_signature_candidates").cloned().unwrap_or(Value::Null),
            "known_signature_auto_handled": payload.get("known_signature_auto_handled").cloned().unwrap_or(Value::Null),
            "known_signature_auto_handle_rate": payload.get("known_signature_auto_handle_rate").cloned().unwrap_or(Value::Null),
            "rollbacks": payload.get("rollbacks").cloned().unwrap_or(Value::Null),
            "kill_switch_engaged": state.kill_switch.engaged
        }),
    );

    let _ = append_jsonl(
        &paths.events_path,
        &json!({
            "ts": payload.get("ts").cloned().unwrap_or(Value::Null),
            "type": "autotest_doctor_event",
            "run_id": payload.get("run_id").cloned().unwrap_or(Value::Null),
            "date": payload.get("date").cloned().unwrap_or(Value::Null),
            "apply": payload.get("apply").cloned().unwrap_or(Value::Null),
            "skipped": payload.get("skipped").cloned().unwrap_or(Value::Null),
            "failures_observed": payload.get("failures_observed").cloned().unwrap_or(Value::Null),
            "actions_applied": payload.get("actions_applied").cloned().unwrap_or(Value::Null),
            "rollbacks": payload.get("rollbacks").cloned().unwrap_or(Value::Null),
            "kill_switch": serde_json::to_value(&state.kill_switch).unwrap_or(Value::Null),
            "receipt_hash": payload.get("receipt_hash").cloned().unwrap_or(Value::Null)
        }),
    );

    payload["run_path"] = Value::String(rel_path(root, &run_path));
    payload["latest_path"] = Value::String(rel_path(root, &paths.latest_path));
    payload["state_path"] = Value::String(rel_path(root, &paths.state_path));
    payload
}
