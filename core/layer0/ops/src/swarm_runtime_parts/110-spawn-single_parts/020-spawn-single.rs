
fn spawn_single(
    state: &mut SwarmState,
    parent_id: Option<&str>,
    task: &str,
    max_depth: u8,
    options: &SpawnOptions,
) -> Result<Value, String> {
    if let Some(loop_guard) = detect_parent_lineage_loop(
        state,
        parent_id,
        (state.scale_policy.max_depth_hard as usize).saturating_mul(2),
    ) {
        append_event(
            state,
            json!({
                "type": "swarm_spawn_loop_guard_blocked",
                "task": task,
                "parent_id": parent_id,
                "diagnostics": loop_guard,
                "timestamp": now_iso(),
            }),
        );
        return Err("lineage_cycle_detected".to_string());
    }

    let request_received_ms = now_epoch_ms();
    let depth = ensure_spawn_capacity(state, parent_id, max_depth)?;

    if options.byzantine && !state.byzantine_test_mode {
        return Err("byzantine_test_mode_required".to_string());
    }

    let queue_wait_ms = now_epoch_ms().saturating_sub(request_received_ms);
    let spawn_initiated_ms = now_epoch_ms();
    let session_id = next_session_id(state, task, depth);
    let (effective_budget, budget_parent_session_id, budget_reservation_tokens) =
        reserve_budget_from_parent(state, parent_id, &session_id, options.token_budget)?;
    let spawn_completed_ms = now_epoch_ms();

    let scaled_task = if options.adaptive_complexity {
        effective_budget
            .map(|budget| scale_task_complexity(task, budget))
            .unwrap_or_else(|| task.to_string())
    } else {
        task.to_string()
    };
    let role_card = resolve_spawn_role_card(options, &scaled_task)?;
    let tool_plan = estimate_tool_plan(&scaled_task, effective_budget);
    let mut budget_telemetry = effective_budget.map(|max_tokens| {
        BudgetTelemetry::new(
            session_id.clone(),
            TokenBudgetConfig {
                max_tokens,
                warning_threshold: options.token_warning_threshold,
                exhaustion_action: options.budget_exhaustion_action.clone(),
            },
        )
    });
    let mut budget_events = Vec::new();
    let mut budget_action_taken: Option<String> = None;

    if let Some(telemetry) = budget_telemetry.as_mut() {
        for (tool, requested_tokens) in &tool_plan {
            match telemetry.record_tool_usage(tool, *requested_tokens) {
                BudgetUsageOutcome::Ok => {}
                BudgetUsageOutcome::Warning(event) => budget_events.push(event),
                BudgetUsageOutcome::ExhaustedAllowed { event, action } => {
                    budget_events.push(event);
                    budget_action_taken = Some(action);
                }
                BudgetUsageOutcome::ExceededDenied(reason) => {
                    budget_action_taken = Some("fail".to_string());
                    let mut failed_metadata = session_metadata_base(
                        session_id.clone(),
                        parent_id.map(ToString::to_string),
                        depth,
                        task.to_string(),
                        "failed".to_string(),
                    );
                    failed_metadata.reachable = false;
                    failed_metadata.byzantine = options.byzantine;
                    failed_metadata.corruption_type = if options.byzantine {
                        Some(options.corruption_type.clone())
                    } else {
                        None
                    };
                    failed_metadata.report = Some(json!({
                        "task": scaled_task,
                        "original_task": task,
                        "session_id": session_id,
                        "depth": depth,
                        "result": "failed",
                        "reason_code": "token_budget_exceeded"
                    }));
                    failed_metadata.budget_telemetry = Some(telemetry.clone());
                    failed_metadata.scaled_task = Some(scaled_task.clone());
                    failed_metadata.budget_action_taken = budget_action_taken.clone();
                    failed_metadata.role = Some(role_card.role.clone());
                    failed_metadata.role_card = Some(role_card.clone());
                    failed_metadata.agent_label = options.agent_label.clone();
                    failed_metadata.budget_parent_session_id = budget_parent_session_id.clone();
                    failed_metadata.budget_reservation_tokens = budget_reservation_tokens;
                    state.sessions.insert(session_id.clone(), failed_metadata);
                    settle_budget_reservation(state, &session_id);
                    append_event(
                        state,
                        json!({
                            "type": "swarm_spawn_failed",
                            "reason_code": "token_budget_exceeded",
                            "session_id": session_id,
                            "task": task,
                            "scaled_task": scaled_task,
                            "depth": depth,
                            "timestamp": now_iso(),
                        }),
                    );
                    return Err(reason);
                }
            }
        }
    }

    let execution_start_ms = now_epoch_ms();
    if options.metrics_detailed {
        thread::sleep(Duration::from_millis(1));
    }
    let execution_end_ms = now_epoch_ms();
    let token_usage_estimate = budget_telemetry
        .as_ref()
        .map(|telemetry| telemetry.final_usage)
        .unwrap_or_else(|| tool_plan.iter().map(|(_, tokens)| *tokens).sum::<u32>());

    let mut report = json!({
        "task": scaled_task,
        "original_task": task,
        "session_id": session_id,
        "depth": depth,
        "result": "ok",
        "token_usage_estimate": token_usage_estimate,
        "token_budget": options.token_budget,
    });
    if options.byzantine {
        report = corrupted_report(options.corruption_type.as_str(), &session_id);
    }

    let metrics = SpawnMetrics {
        request_received_ms,
        queue_wait_ms,
        spawn_initiated_ms,
        spawn_completed_ms,
        execution_start_ms,
        execution_end_ms,
        report_back_latency_ms: now_epoch_ms().saturating_sub(execution_end_ms),
    };

    let mut metadata = session_metadata_base(
        session_id.clone(),
        parent_id.map(ToString::to_string),
        depth,
        task.to_string(),
        "running".to_string(),
    );
    metadata.reachable = !options.simulate_unreachable;
    metadata.byzantine = options.byzantine;
    metadata.corruption_type = if options.byzantine {
        Some(options.corruption_type.clone())
    } else {
        None
    };
    metadata.report = Some(report.clone());
    metadata.metrics = Some(metrics.clone());
    metadata.budget_telemetry = budget_telemetry.clone();
    metadata.scaled_task = Some(scaled_task.clone());
    metadata.budget_action_taken = budget_action_taken.clone();
    metadata.role = Some(role_card.role.clone());
    metadata.role_card = Some(role_card.clone());
    metadata.agent_label = options.agent_label.clone();
    metadata.budget_parent_session_id = budget_parent_session_id;
    metadata.budget_reservation_tokens = budget_reservation_tokens;

    state.sessions.insert(session_id.clone(), metadata);
    register_service_instance(
        state,
        &session_id,
        Some(role_card.role.clone()),
        role_card.capability_envelope.clone(),
    );

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
    settle_budget_reservation(state, &session_id);

    let verification = if options.verify {
        match verify_session_reachable(state, &session_id, options.timeout_ms) {
            Ok(result) => result,
            Err(err) => {
                if let Some(session) = state.sessions.get_mut(&session_id) {
                    session.status = "failed".to_string();
                }
                return Err(err);
            }
        }
    } else {
        json!({"status": "skipped"})
    };

    append_event(
        state,
        json!({
            "type": "swarm_spawn",
            "session_id": session_id,
            "parent_id": parent_id,
            "lineage_parent_id": parent_id,
            "depth": depth,
            "task": task,
            "scaled_task": scaled_task.clone(),
            "role_card": role_card,
            "verified": options.verify,
            "byzantine": options.byzantine,
            "token_budget": options.token_budget,
            "token_usage_estimate": token_usage_estimate,
            "budget_action_taken": budget_action_taken,
            "budget_events": budget_events,
            "timestamp": now_iso()
        }),
    );

    let auto_publish_receipt = if options.auto_publish_results {
        let payload = if let Some(value) = options.result_value {
            ResultPayload::Calculation { value }
        } else if let Some(text) = options.result_text.as_ref() {
            ResultPayload::Text {
                content: text.to_string(),
            }
        } else {
            ResultPayload::Structured {
                schema: "swarm_runtime_report_v1".to_string(),
                data: report.clone(),
            }
        };
        let mut metadata = Map::new();
        metadata.insert("source".to_string(), json!("spawn_auto_publish"));
        metadata.insert("task".to_string(), json!(scaled_task));
        metadata.insert("report".to_string(), report.clone());
        Some(publish_result(
            state,
            &session_id,
            options.agent_label.clone(),
            Some(task.to_string()),
            payload,
            Value::Object(metadata),
            options.result_confidence,
            options.verification_status.clone(),
        )?)
    } else {
        None
    };

    Ok(json!({
        "session_id": session_id,
        "session_key": session_key(&session_id),
        "parent_id": parent_id,
        "depth": depth,
        "verification": verification,
        "report": report,
        "metrics": metrics.as_json(),
        "budget_report": budget_telemetry.map(|telemetry| telemetry.generate_report()),
        "auto_publish_result": auto_publish_receipt,
        "role_card": state.sessions.get(&session_id).and_then(|session| session.role_card.clone()),
        "session_state": {
            "session_id": session_id,
            "session_key": session_key(&session_id),
            "tool_access": default_session_tool_access(),
            "tool_manifest": session_tool_manifest(state, state.sessions.get(&session_id).expect("session inserted")),
        }
    }))
}
