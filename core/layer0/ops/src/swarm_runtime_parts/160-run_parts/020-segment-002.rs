            let payload_result = if recursive {
                if !matches!(mode, ExecutionMode::TaskOriented) {
                    Err("recursive_mode_requires_task_execution_mode".to_string())
                } else {
                    recursive_spawn_with_tracking(
                        &mut state,
                        parent_id.as_deref(),
                        &task,
                        levels,
                        max_depth,
                        &options,
                    )
                }
            } else {
                match mode {
                    ExecutionMode::TaskOriented => {
                        spawn_single(&mut state, parent_id.as_deref(), &task, max_depth, &options)
                    }
                    ExecutionMode::Persistent(cfg) => spawn_persistent_session(
                        &mut state,
                        parent_id.as_deref(),
                        &task,
                        max_depth,
                        &options,
                        &cfg,
                        false,
                    ),
                    ExecutionMode::Background(cfg) => spawn_persistent_session(
                        &mut state,
                        parent_id.as_deref(),
                        &task,
                        max_depth,
                        &options,
                        &cfg,
                        true,
                    ),
                }
            };
            payload_result.map(|payload| {
                json!({
                    "ok": true,
                    "type": "swarm_runtime_spawn",
                    "recursive": recursive,
                    "mode": match options.execution_mode {
                        ExecutionMode::TaskOriented => "task",
                        ExecutionMode::Persistent(_) => "persistent",
                        ExecutionMode::Background(_) => "background",
                    },
                    "payload": payload,
                })
            })
        }
        "tick" => {
            let now_ms = now_epoch_ms().saturating_add(parse_u64_flag(argv, "advance-ms", 0));
            let max_check_ins = parse_u64_flag(argv, "max-check-ins", 16).max(1);
            tick_persistent_sessions(&mut state, now_ms, max_check_ins).map(|payload| {
                json!({
                    "ok": true,
                    "type": "swarm_runtime_tick",
                    "payload": payload,
                })
            })
        }
        "byzantine-test" => {
            let action = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            match action.as_str() {
                "enable" => {
                    state.byzantine_test_mode = true;
                    Ok(json!({
                        "ok": true,
                        "type": "swarm_runtime_byzantine_test",
                        "enabled": true,
                    }))
                }
                "disable" => {
                    state.byzantine_test_mode = false;
                    Ok(json!({
                        "ok": true,
                        "type": "swarm_runtime_byzantine_test",
                        "enabled": false,
                    }))
                }
                _ => Ok(json!({
                    "ok": true,
                    "type": "swarm_runtime_byzantine_test",
                    "enabled": state.byzantine_test_mode,
                })),
            }
        }
        "consensus-check" => {
            let task_id = parse_flag(argv, "task-id");
            let threshold = parse_f64_flag(argv, "threshold", 0.6).clamp(0.0, 1.0);
            let report_flag = parse_flag(argv, "reports-json");
            let mut reports = parse_reports_from_flag(report_flag);
            if reports.is_empty() {
                reports = reports_from_state(&state, task_id.as_deref());
            }
            let fields = normalize_fields(parse_flag(argv, "fields"), &reports);
            let consensus = evaluate_consensus(&reports, &fields, threshold);
            Ok(json!({
                "ok": true,
                "type": "swarm_runtime_consensus",
                "task_id": task_id,
                "consensus": consensus,
                "sample_size": reports.len(),
            }))
        }
        "budget-report" => {
            if let Some(session_id) =
                parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
            {
                budget_report_for_session(&state, &session_id)
            } else {
                Err("session_id_required".to_string())
            }
        }
        "background" => run_background_command(&mut state, argv),
        "scheduled" => {
            let sub = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            match sub.as_str() {
                "add" => scheduled_add(&mut state, argv, now_epoch_ms()),
                "status" => Ok(scheduled_status(&state)),
                "run-due" => {
                    let now_ms =
                        now_epoch_ms().saturating_add(parse_u64_flag(argv, "advance-ms", 0));
                    let max_runs = parse_u64_flag(argv, "max-runs", 8).max(1);
                    scheduled_run_due(&mut state, now_ms, max_runs)
                }
                _ => Err(format!("unknown_scheduled_subcommand:{sub}")),
            }
        }
        "plans" => run_plans_command(&mut state, argv),
        "sessions" => {
            let sub = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            match sub.as_str() {
                "budget-report" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        budget_report_for_session(&state, &session_id)
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "wake" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        sessions_wake(&mut state, &session_id, now_epoch_ms())
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "resume" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        sessions_resume(&mut state, &session_id, now_epoch_ms())
                    } else {
                        Err("session_id_required".to_string())
                    }
                }
                "terminate" => {
                    if let Some(session_id) =
                        parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
                    {
                        sessions_terminate(
                            &mut state,
                            &session_id,
                            parse_bool_flag(argv, "graceful", true),
                            now_epoch_ms(),
