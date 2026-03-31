fn spawn_persistent_session(
    state: &mut SwarmState,
    parent_id: Option<&str>,
    task: &str,
    max_depth: u8,
    options: &SpawnOptions,
    cfg: &PersistentAgentConfig,
    background_worker: bool,
) -> Result<Value, String> {
    let parent_depth = if let Some(parent) = parent_id {
        let parent_session = state
            .sessions
            .get(parent)
            .ok_or_else(|| format!("parent_session_missing:{parent}"))?;
        parent_session.depth
    } else {
        0
    };
    let depth = if parent_id.is_some() {
        parent_depth.saturating_add(1)
    } else {
        0
    };
    if depth >= max_depth {
        return Err(format!("max_depth_exceeded:{depth}>=max_depth:{max_depth}"));
    }

    let session_id = next_session_id(state, task, depth);
    let (effective_budget, budget_parent_session_id, budget_reservation_tokens) =
        reserve_budget_from_parent(state, parent_id, &session_id, options.token_budget)?;
    let now_ms = now_epoch_ms();
    let scaled_task = if options.adaptive_complexity {
        effective_budget
            .map(|budget| scale_task_complexity(task, budget))
            .unwrap_or_else(|| task.to_string())
    } else {
        task.to_string()
    };
    let runtime = PersistentRuntime {
        mode: if background_worker {
            "background".to_string()
        } else {
            "persistent".to_string()
        },
        config: cfg.clone(),
        started_at_ms: now_ms,
        deadline_ms: now_ms.saturating_add(cfg.lifespan_sec.saturating_mul(1000)),
        next_check_in_ms: now_ms.saturating_add(cfg.check_in_interval_sec.saturating_mul(1000)),
        check_in_count: 0,
        last_check_in_ms: None,
        terminated_at_ms: None,
        terminated_reason: None,
    };
    let mut metadata = SessionMetadata {
        session_id: session_id.clone(),
        parent_id: parent_id.map(ToString::to_string),
        children: Vec::new(),
        depth,
        task: task.to_string(),
        created_at: now_iso(),
        status: if background_worker {
            "background_running".to_string()
        } else {
            "persistent_running".to_string()
        },
        reachable: true,
        byzantine: false,
        corruption_type: None,
        report: None,
        metrics: Some(SpawnMetrics {
            request_received_ms: now_ms,
            queue_wait_ms: 0,
            spawn_initiated_ms: now_ms,
            spawn_completed_ms: now_ms,
            execution_start_ms: now_ms,
            execution_end_ms: now_ms,
            report_back_latency_ms: 0,
        }),
        budget_telemetry: effective_budget.map(|max_tokens| {
            BudgetTelemetry::new(
                session_id.clone(),
                TokenBudgetConfig {
                    max_tokens,
                    warning_threshold: options.token_warning_threshold,
                    exhaustion_action: options.budget_exhaustion_action.clone(),
                },
            )
        }),
        scaled_task: Some(scaled_task),
        budget_action_taken: None,
        role: options.role.clone(),
        agent_label: options.agent_label.clone(),
        tool_access: default_session_tool_access(),
        context_vars: BTreeMap::new(),
        context_mode: None,
        handoff_ids: Vec::new(),
        registered_tool_ids: Vec::new(),
        stream_turn_ids: Vec::new(),
        turn_run_ids: Vec::new(),
        network_ids: Vec::new(),
        check_ins: Vec::new(),
        metrics_timeline: Vec::new(),
        anomalies: Vec::new(),
        persistent: Some(runtime),
        background_worker,
        budget_parent_session_id,
        budget_reservation_tokens,
        budget_reservation_settled: false,
        thorn_cell: false,
        thorn_target_session_id: None,
        thorn_expires_at_ms: None,
        quarantine_reason: None,
        quarantine_previous_status: None,
    };

    let initial = perform_persistent_check_in(&mut metadata, "initial", false)?;
    state.sessions.insert(session_id.clone(), metadata);
    if let Some(parent) = parent_id {
        if let Some(parent_session) = state.sessions.get_mut(parent) {
            if !parent_session
                .children
                .iter()
                .any(|child| child == &session_id)
            {
                parent_session.children.push(session_id.clone());
            }
        }
    }
    register_service_instance(
        state,
        &session_id,
        options.role.clone(),
        options.capabilities.clone(),
    );

    append_event(
        state,
        json!({
            "type": if background_worker { "swarm_background_spawn" } else { "swarm_persistent_spawn" },
            "session_id": session_id,
            "task": task,
            "lifespan_sec": cfg.lifespan_sec,
            "check_in_interval_sec": cfg.check_in_interval_sec,
            "report_mode": cfg.report_mode.as_label(),
            "timestamp": now_iso(),
        }),
    );

    Ok(json!({
        "session_id": session_id,
        "session_key": session_key(&session_id),
        "mode": if background_worker { "background" } else { "persistent" },
        "lifespan_sec": cfg.lifespan_sec,
        "check_in_interval_sec": cfg.check_in_interval_sec,
        "report_mode": cfg.report_mode.as_label(),
        "initial_check_in": initial,
        "session_state": {
            "session_id": session_id,
            "session_key": session_key(&session_id),
            "tool_access": default_session_tool_access(),
            "tool_manifest": session_tool_manifest(state, state.sessions.get(&session_id).expect("session inserted")),
        }
    }))
}
