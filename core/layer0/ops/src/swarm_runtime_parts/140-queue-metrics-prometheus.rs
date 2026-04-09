fn queue_metrics_prometheus(state: &SwarmState, snapshot: &Value) -> String {
    let sample_count = snapshot
        .get("sample_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let queue_wait_sum = snapshot
        .get("queue_wait_ms")
        .and_then(|row| row.get("sum"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let queue_wait_avg = snapshot
        .get("queue_wait_ms")
        .and_then(|row| row.get("avg"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let queue_wait_p95 = snapshot
        .get("queue_wait_ms")
        .and_then(|row| row.get("p95"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let execution_sum = snapshot
        .get("execution_ms")
        .and_then(|row| row.get("sum"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let execution_avg = snapshot
        .get("execution_ms")
        .and_then(|row| row.get("avg"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let execution_p95 = snapshot
        .get("execution_ms")
        .and_then(|row| row.get("p95"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let unread_total = snapshot
        .get("mailbox")
        .and_then(|row| row.get("unread_total"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let dead_letter_total = snapshot
        .get("mailbox")
        .and_then(|row| row.get("dead_letter_total"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let backpressure_total = snapshot
        .get("mailbox")
        .and_then(|row| row.get("backpressure_total"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let active_sessions = snapshot
        .get("session_counts")
        .and_then(|row| row.get("active"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let persistent_sessions = snapshot
        .get("session_counts")
        .and_then(|row| row.get("persistent"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let scheduled_active = state
        .scheduled_tasks
        .values()
        .filter(|row| row.active)
        .count();

    vec![
        "# HELP swarm_runtime_queue_samples_total Number of sessions with spawn metrics."
            .to_string(),
        "# TYPE swarm_runtime_queue_samples_total gauge".to_string(),
        format!("swarm_runtime_queue_samples_total {sample_count}"),
        "# HELP swarm_runtime_queue_wait_ms_sum Total queue wait time in milliseconds.".to_string(),
        "# TYPE swarm_runtime_queue_wait_ms_sum gauge".to_string(),
        format!("swarm_runtime_queue_wait_ms_sum {queue_wait_sum}"),
        "# HELP swarm_runtime_queue_wait_ms_avg Average queue wait in milliseconds.".to_string(),
        "# TYPE swarm_runtime_queue_wait_ms_avg gauge".to_string(),
        format!("swarm_runtime_queue_wait_ms_avg {:.3}", queue_wait_avg),
        "# HELP swarm_runtime_queue_wait_ms_p95 P95 queue wait in milliseconds.".to_string(),
        "# TYPE swarm_runtime_queue_wait_ms_p95 gauge".to_string(),
        format!("swarm_runtime_queue_wait_ms_p95 {queue_wait_p95}"),
        "# HELP swarm_runtime_execution_ms_sum Total execution time in milliseconds.".to_string(),
        "# TYPE swarm_runtime_execution_ms_sum gauge".to_string(),
        format!("swarm_runtime_execution_ms_sum {execution_sum}"),
        "# HELP swarm_runtime_execution_ms_avg Average execution time in milliseconds.".to_string(),
        "# TYPE swarm_runtime_execution_ms_avg gauge".to_string(),
        format!("swarm_runtime_execution_ms_avg {:.3}", execution_avg),
        "# HELP swarm_runtime_execution_ms_p95 P95 execution time in milliseconds.".to_string(),
        "# TYPE swarm_runtime_execution_ms_p95 gauge".to_string(),
        format!("swarm_runtime_execution_ms_p95 {execution_p95}"),
        "# HELP swarm_runtime_unread_messages_total Total unread inter-agent messages.".to_string(),
        "# TYPE swarm_runtime_unread_messages_total gauge".to_string(),
        format!("swarm_runtime_unread_messages_total {unread_total}"),
        "# HELP swarm_runtime_dead_letters_total Total dead-lettered inter-agent messages."
            .to_string(),
        "# TYPE swarm_runtime_dead_letters_total gauge".to_string(),
        format!("swarm_runtime_dead_letters_total {dead_letter_total}"),
        "# HELP swarm_runtime_mailbox_backpressure_total Total mailbox backpressure dead letters."
            .to_string(),
        "# TYPE swarm_runtime_mailbox_backpressure_total gauge".to_string(),
        format!("swarm_runtime_mailbox_backpressure_total {backpressure_total}"),
        "# HELP swarm_runtime_active_sessions Total active sessions.".to_string(),
        "# TYPE swarm_runtime_active_sessions gauge".to_string(),
        format!("swarm_runtime_active_sessions {active_sessions}"),
        "# HELP swarm_runtime_persistent_sessions Total persistent sessions.".to_string(),
        "# TYPE swarm_runtime_persistent_sessions gauge".to_string(),
        format!("swarm_runtime_persistent_sessions {persistent_sessions}"),
        "# HELP swarm_runtime_sessions_total Total sessions recorded in state.".to_string(),
        "# TYPE swarm_runtime_sessions_total gauge".to_string(),
        format!("swarm_runtime_sessions_total {}", state.sessions.len()),
        "# HELP swarm_runtime_scheduled_active_tasks Total active scheduled background tasks."
            .to_string(),
        "# TYPE swarm_runtime_scheduled_active_tasks gauge".to_string(),
        format!("swarm_runtime_scheduled_active_tasks {scheduled_active}"),
    ]
    .join("\n")
}

fn scheduled_add(state: &mut SwarmState, argv: &[String], now_ms: u64) -> Result<Value, String> {
    let task = parse_flag(argv, "task").unwrap_or_else(|| "scheduled-swarm-task".to_string());
    let interval_sec = parse_u64_flag(argv, "interval-sec", 900).max(1);
    let runs = parse_u64_flag(argv, "runs", 4).max(1);
    let max_runtime_sec = parse_u64_flag(argv, "max-runtime-sec", 30).max(1);
    let task_id = format!(
        "scheduled-{}",
        &deterministic_receipt_hash(&json!({
            "task": task,
            "interval_sec": interval_sec,
            "runs": runs,
            "ts": now_ms,
        }))[..12]
    );
    let row = ScheduledTask {
        task_id: task_id.clone(),
        task,
        interval_sec,
        max_runtime_sec,
        next_run_ms: now_ms.saturating_add(interval_sec.saturating_mul(1000)),
        remaining_runs: runs,
        last_run_ms: None,
        last_session_id: None,
        active: true,
    };
    state.scheduled_tasks.insert(task_id.clone(), row.clone());
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_scheduled_add",
        "task": row,
    }))
}

fn scheduled_status(state: &SwarmState) -> Value {
    let active = state
        .scheduled_tasks
        .values()
        .filter(|row| row.active)
        .count();
    json!({
        "ok": true,
        "type": "swarm_runtime_scheduled_status",
        "total_tasks": state.scheduled_tasks.len(),
        "active_tasks": active,
        "tasks": state.scheduled_tasks.values().cloned().collect::<Vec<_>>(),
    })
}

fn scheduled_run_due(state: &mut SwarmState, now_ms: u64, max_runs: u64) -> Result<Value, String> {
    let mut executed = Vec::new();
    let due_ids = state
        .scheduled_tasks
        .iter()
        .filter(|(_, row)| row.active && row.remaining_runs > 0 && row.next_run_ms <= now_ms)
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    for task_id in due_ids.into_iter().take(max_runs as usize) {
        let Some(task_row) = state.scheduled_tasks.get(&task_id).cloned() else {
            continue;
        };
        let options = SpawnOptions {
            verify: false,
            timeout_ms: task_row.max_runtime_sec.saturating_mul(1000),
            metrics_detailed: true,
            simulate_unreachable: false,
            byzantine: false,
            corruption_type: "data_falsification".to_string(),
            token_budget: None,
            token_warning_threshold: 0.8,
            budget_exhaustion_action: BudgetAction::FailHard,
            adaptive_complexity: false,
            execution_mode: ExecutionMode::TaskOriented,
            role: None,
            capabilities: Vec::new(),
            auto_publish_results: false,
            agent_label: None,
            result_value: None,
            result_text: None,
            result_confidence: 1.0,
            verification_status: "not_verified".to_string(),
        };
        let spawn = spawn_single(state, None, &task_row.task, 64, &options)?;
        let session_id = spawn
            .get("session_id")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        if let Some(task) = state.scheduled_tasks.get_mut(&task_id) {
            task.last_run_ms = Some(now_ms);
            task.last_session_id = session_id.clone();
            task.remaining_runs = task.remaining_runs.saturating_sub(1);
            task.next_run_ms = now_ms.saturating_add(task.interval_sec.saturating_mul(1000));
            if task.remaining_runs == 0 {
                task.active = false;
            }
        }
        executed.push(json!({
            "task_id": task_id,
            "session_id": session_id,
        }));
    }
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_scheduled_run_due",
        "executed": executed,
    }))
}

fn run_background_command(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let sub = argv
        .get(1)
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let now_ms = now_epoch_ms();
    match sub.as_str() {
        "start" => {
            let task = parse_flag(argv, "task")
                .unwrap_or_else(|| "Background worker heartbeat".to_string());
            let parent_id = parse_flag(argv, "session-id");
            let options = build_spawn_options(argv);
            let cfg = match &options.execution_mode {
                ExecutionMode::Background(cfg) | ExecutionMode::Persistent(cfg) => cfg.clone(),
                ExecutionMode::TaskOriented => PersistentAgentConfig {
                    lifespan_sec: parse_u64_flag(argv, "lifespan-sec", 3600).max(1),
                    check_in_interval_sec: parse_u64_flag(argv, "check-in-interval-sec", 900)
                        .max(1),
                    report_mode: ReportMode::from_flag(parse_flag(argv, "report-mode")),
                },
            };
            let payload = spawn_persistent_session(
                state,
                parent_id.as_deref(),
                &task,
                parse_u8_flag(argv, "max-depth", 8).max(1),
                &options,
                &cfg,
                true,
            )?;
            Ok(json!({
                "ok": true,
                "type": "swarm_runtime_background_start",
                "payload": payload,
            }))
        }
        "status" => {
            let workers = state
                .sessions
                .values()
                .filter(|session| session.background_worker)
                .map(|session| {
                    let runtime = session.persistent.as_ref();
                    json!({
                        "session_id": session.session_id,
                        "status": session.status,
                        "check_in_count": runtime.map(|r| r.check_in_count).unwrap_or(0),
                        "next_check_in_ms": runtime.and_then(|r| if matches!(session.status.as_str(), "background_running") { Some(r.next_check_in_ms) } else { None }),
                        "remaining_lifespan_ms": runtime.map(|r| r.deadline_ms.saturating_sub(now_ms)).unwrap_or(0),
                    })
                })
                .collect::<Vec<_>>();
            Ok(json!({
                "ok": true,
                "type": "swarm_runtime_background_status",
                "worker_count": workers.len(),
                "workers": workers,
            }))
        }
        "stop" => {
            let graceful = parse_bool_flag(argv, "graceful", true);
            if let Some(session_id) =
                parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty())
            {
                return sessions_terminate(state, &session_id, graceful, now_ms).map(|payload| {
                    json!({
                        "ok": true,
                        "type": "swarm_runtime_background_stop",
                        "stopped": [payload],
                    })
                });
            }
            let to_stop = state
                .sessions
                .iter()
                .filter(|(_, session)| {
                    session.background_worker
                        && matches!(session.status.as_str(), "background_running")
                })
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            let mut stopped = Vec::new();
            for session_id in to_stop {
                let payload = sessions_terminate(state, &session_id, graceful, now_ms)?;
                stopped.push(payload);
            }
            Ok(json!({
                "ok": true,
                "type": "swarm_runtime_background_stop",
                "stopped_count": stopped.len(),
                "stopped": stopped,
            }))
        }
        _ => Err(format!("unknown_background_subcommand:{sub}")),
    }
}

fn run_test_persistent(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let cfg = PersistentAgentConfig {
        lifespan_sec: parse_u64_flag(argv, "lifespan-sec", 60).max(1),
        check_in_interval_sec: parse_u64_flag(argv, "check-in-interval-sec", 15).max(1),
        report_mode: ReportMode::Always,
    };
    let options = SpawnOptions {
        verify: false,
        timeout_ms: 1_000,
        metrics_detailed: true,
        simulate_unreachable: false,
        byzantine: false,
        corruption_type: "data_falsification".to_string(),
        token_budget: Some(2000),
        token_warning_threshold: 0.8,
        budget_exhaustion_action: BudgetAction::AllowWithWarning,
        adaptive_complexity: true,
        execution_mode: ExecutionMode::Persistent(cfg.clone()),
        role: Some("health-monitor".to_string()),
        capabilities: vec!["check_in".to_string(), "status_report".to_string()],
        auto_publish_results: false,
        agent_label: None,
        result_value: None,
        result_text: None,
        result_confidence: 1.0,
        verification_status: "not_verified".to_string(),
    };
    let task =
        parse_flag(argv, "task").unwrap_or_else(|| "Persistent health check loop".to_string());
    let spawned = spawn_persistent_session(state, None, &task, 8, &options, &cfg, false)?;
    let advance_ms = parse_u64_flag(
        argv,
        "advance-ms",
        cfg.check_in_interval_sec.saturating_mul(1000),
    );
    let ticked = tick_persistent_sessions(state, now_epoch_ms().saturating_add(advance_ms), 16)?;
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_test_persistent",
        "spawned": spawned,
        "ticked": ticked,
    }))
}
