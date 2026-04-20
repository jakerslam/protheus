fn detect_parent_lineage_loop(
    state: &SwarmState,
    start_parent_id: Option<&str>,
    max_hops: usize,
) -> Option<Value> {
    let mut current = start_parent_id.map(ToString::to_string);
    let mut visited = BTreeSet::new();
    let mut lineage = Vec::new();
    let mut hops = 0usize;

    while let Some(session_id) = current {
        hops = hops.saturating_add(1);
        if hops > max_hops.max(1) {
            return Some(json!({
                "detected": true,
                "reason": "cost_guard_exceeded",
                "max_hops": max_hops.max(1),
                "hops": hops,
                "lineage": lineage,
            }));
        }
        if !visited.insert(session_id.clone()) {
            lineage.push(session_id.clone());
            return Some(json!({
                "detected": true,
                "reason": "cycle_detected",
                "cycle_at": session_id,
                "hops": hops,
                "lineage": lineage,
            }));
        }
        lineage.push(session_id.clone());
        current = state
            .sessions
            .get(&session_id)
            .and_then(|session| session.parent_id.clone());
    }

    None
}

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

fn recursive_spawn_with_tracking(
    state: &mut SwarmState,
    parent_id: Option<&str>,
    task: &str,
    levels: u8,
    max_depth: u8,
    options: &SpawnOptions,
) -> Result<Value, String> {
    if levels == 0 {
        return Err("recursive_levels_must_be_positive".to_string());
    }

    let mut lineage = Vec::new();
    let mut current_parent = parent_id.map(ToString::to_string);
    let mut level = 0u8;
    let mut visited_parent_chain = BTreeSet::new();
    let mut loop_guard = json!({
        "triggered": false,
        "reason": "none",
    });
    let traversal_limit = (levels as usize).saturating_mul(4).max(8);
    let mut traversal_cost = 0usize;
    while level < levels {
        traversal_cost = traversal_cost.saturating_add(1);
        if traversal_cost > traversal_limit {
            loop_guard = json!({
                "triggered": true,
                "reason": "cost_guard_exceeded",
                "traversal_limit": traversal_limit,
                "traversal_cost": traversal_cost,
                "lineage_count": lineage.len(),
            });
            append_event(
                state,
                json!({
                    "type": "swarm_recursive_loop_guard_triggered",
                    "reason": "cost_guard_exceeded",
                    "task": task,
                    "parent_id": parent_id,
                    "lineage_count": lineage.len(),
                    "timestamp": now_iso(),
                }),
            );
            break;
        }
        if let Some(parent) = current_parent.as_ref() {
            if !visited_parent_chain.insert(parent.clone()) {
                loop_guard = json!({
                    "triggered": true,
                    "reason": "cycle_detected",
                    "cycle_at": parent,
                    "lineage_count": lineage.len(),
                });
                append_event(
                    state,
                    json!({
                        "type": "swarm_recursive_loop_guard_triggered",
                        "reason": "cycle_detected",
                        "cycle_at": parent,
                        "task": task,
                        "parent_id": parent_id,
                        "lineage_count": lineage.len(),
                        "timestamp": now_iso(),
                    }),
                );
                break;
            }
            if let Some(parent_guard) =
                detect_parent_lineage_loop(state, Some(parent.as_str()), traversal_limit)
            {
                loop_guard = json!({
                    "triggered": true,
                    "reason": "lineage_cycle_guard_blocked",
                    "diagnostics": parent_guard.clone(),
                });
                append_event(
                    state,
                    json!({
                        "type": "swarm_recursive_loop_guard_triggered",
                        "reason": "lineage_cycle_guard_blocked",
                        "task": task,
                        "parent_id": parent_id,
                        "diagnostics": parent_guard,
                        "lineage_count": lineage.len(),
                        "timestamp": now_iso(),
                    }),
                );
                break;
            }
        }
        let spawned = spawn_single(state, current_parent.as_deref(), task, max_depth, options)?;
        let child = spawned
            .get("session_id")
            .and_then(Value::as_str)
            .ok_or_else(|| "spawn_missing_session_id".to_string())?
            .to_string();
        lineage.push(spawned);
        current_parent = Some(child);
        level = level.saturating_add(1);
    }

    Ok(json!({
        "recursive": true,
        "terminated_safely": loop_guard.get("triggered").and_then(Value::as_bool).unwrap_or(false),
        "loop_guard": loop_guard,
        "levels": levels,
        "lineage": lineage,
        "final_session_id": current_parent,
        "max_depth": max_depth
    }))
}

fn corrupted_report(corruption_type: &str, session_id: &str) -> Value {
    match corruption_type {
        "wrong_file" => json!({
            "session_id": session_id,
            "file": "FAKE.md",
            "file_size": 9999,
            "word_count": 5000,
            "first_line": "FAKE DATA HERE",
            "corrupted": true,
        }),
        _ => json!({
            "session_id": session_id,
            "file": "SOUL.md",
            "file_size": 9999,
            "word_count": 5000,
            "first_line": "FAKE DATA HERE",
            "corrupted": true,
        }),
    }
}

fn parse_reports(raw: &Value) -> Vec<AgentReport> {
    raw.as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let agent_id = row
                        .get("agent_id")
                        .or_else(|| row.get("agent"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToString::to_string)?;

                    let mut values = BTreeMap::new();
                    if let Some(object) = row.get("values").and_then(Value::as_object) {
                        for (key, value) in object {
                            values.insert(key.to_string(), value.clone());
                        }
                    }
                    Some(AgentReport { agent_id, values })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn reports_from_state(state: &SwarmState, task_id: Option<&str>) -> Vec<AgentReport> {
    let mut reports = Vec::new();
    for session in state.sessions.values() {
        if let Some(filter) = task_id {
            if session.task != filter {
                continue;
            }
        }

        let Some(report_value) = session.report.as_ref() else {
            continue;
        };

        let mut values = BTreeMap::new();
        if let Some(object) = report_value.as_object() {
            for (key, value) in object {
                values.insert(key.to_string(), value.clone());
            }
        }

        reports.push(AgentReport {
            agent_id: session.session_id.clone(),
            values,
        });
    }
    reports
}

fn normalize_fields(fields_csv: Option<String>, reports: &[AgentReport]) -> Vec<String> {
    if let Some(raw) = fields_csv {
        let mut parsed = raw
            .split(',')
            .map(|field| field.trim())
            .filter(|field| !field.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        parsed.sort();
        parsed.dedup();
        if !parsed.is_empty() {
            return parsed;
        }
    }

    let mut keys = reports
        .iter()
        .flat_map(|report| report.values.keys().cloned())
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    keys
}
