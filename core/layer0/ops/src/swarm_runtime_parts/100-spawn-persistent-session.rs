fn runtime_mode_label(background_worker: bool) -> &'static str {
    if background_worker {
        "background"
    } else {
        "persistent"
    }
}

fn runtime_status_label(background_worker: bool) -> &'static str {
    if background_worker {
        "background_running"
    } else {
        "persistent_running"
    }
}

fn session_state_payload(state: &SwarmState, session_id: &str) -> Value {
    let key = session_key(session_id);
    let session = state.sessions.get(session_id).expect("session inserted");
    json!({
        "session_id": session_id,
        "session_key": key,
        "tool_access": default_session_tool_access(),
        "tool_manifest": session_tool_manifest(state, session),
    })
}

fn spawn_persistent_session(
    state: &mut SwarmState,
    parent_id: Option<&str>,
    task: &str,
    max_depth: u8,
    options: &SpawnOptions,
    cfg: &PersistentAgentConfig,
    background_worker: bool,
) -> Result<Value, String> {
    let depth = ensure_spawn_capacity(state, parent_id, max_depth)?;

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
        mode: runtime_mode_label(background_worker).to_string(),
        config: cfg.clone(),
        started_at_ms: now_ms,
        deadline_ms: now_ms.saturating_add(cfg.lifespan_sec.saturating_mul(1000)),
        next_check_in_ms: now_ms.saturating_add(cfg.check_in_interval_sec.saturating_mul(1000)),
        check_in_count: 0,
        last_check_in_ms: None,
        terminated_at_ms: None,
        terminated_reason: None,
    };
    let mut metadata = session_metadata_base(
        session_id.clone(),
        parent_id.map(ToString::to_string),
        depth,
        task.to_string(),
        runtime_status_label(background_worker).to_string(),
    );
    metadata.metrics = Some(SpawnMetrics {
        request_received_ms: now_ms,
        queue_wait_ms: 0,
        spawn_initiated_ms: now_ms,
        spawn_completed_ms: now_ms,
        execution_start_ms: now_ms,
        execution_end_ms: now_ms,
        report_back_latency_ms: 0,
    });
    metadata.budget_telemetry = effective_budget.map(|max_tokens| {
        BudgetTelemetry::new(
            session_id.clone(),
            TokenBudgetConfig {
                max_tokens,
                warning_threshold: options.token_warning_threshold,
                exhaustion_action: options.budget_exhaustion_action.clone(),
            },
        )
    });
    metadata.scaled_task = Some(scaled_task);
    metadata.role = options.role.clone();
    metadata.agent_label = options.agent_label.clone();
    metadata.persistent = Some(runtime);
    metadata.background_worker = background_worker;
    metadata.budget_parent_session_id = budget_parent_session_id;
    metadata.budget_reservation_tokens = budget_reservation_tokens;

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
        "mode": runtime_mode_label(background_worker),
        "lifespan_sec": cfg.lifespan_sec,
        "check_in_interval_sec": cfg.check_in_interval_sec,
        "report_mode": cfg.report_mode.as_label(),
        "initial_check_in": initial,
        "session_state": session_state_payload(state, &session_id),
    }))
}
