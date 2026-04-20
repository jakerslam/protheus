fn evaluate_consensus(reports: &[AgentReport], fields: &[String], threshold: f64) -> Value {
    if reports.is_empty() {
        return json!({
            "consensus_reached": false,
            "reason_code": "no_reports",
            "confidence": 0.0,
            "outliers": []
        });
    }

    let mut groups: BTreeMap<String, Vec<(String, Map<String, Value>)>> = BTreeMap::new();
    for report in reports {
        let mut selected = Map::new();
        for field in fields {
            selected.insert(
                field.clone(),
                report.values.get(field).cloned().unwrap_or(Value::Null),
            );
        }
        let fingerprint = deterministic_receipt_hash(&Value::Object(selected.clone()));
        groups
            .entry(fingerprint)
            .or_default()
            .push((report.agent_id.clone(), selected));
    }

    let Some((leader_fp, leader_group)) = groups.iter().max_by_key(|(_, rows)| rows.len()) else {
        return json!({
            "consensus_reached": false,
            "reason_code": "grouping_failed",
            "confidence": 0.0,
            "outliers": []
        });
    };

    let extractable_count = groups.values().map(Vec::len).sum::<usize>();
    let confidence = if extractable_count == 0 {
        0.0
    } else {
        leader_group.len() as f64 / extractable_count as f64
    };
    let mut outliers = Vec::new();
    for (fingerprint, rows) in &groups {
        if fingerprint == leader_fp {
            continue;
        }
        for (agent_id, selected) in rows {
            outliers.push(json!({
                "agent": agent_id,
                "values": selected,
                "deviation": "outlier_group"
            }));
        }
    }

    let agreed_value = leader_group
        .first()
        .map(|(_, selected)| Value::Object(selected.clone()))
        .unwrap_or(Value::Object(Map::new()));
    let disagreement_count = extractable_count.saturating_sub(leader_group.len());
    let outlier_rate = if extractable_count == 0 {
        0.0
    } else {
        disagreement_count as f64 / extractable_count as f64
    };
    let confidence_band = if confidence >= 0.9 {
        "high"
    } else if confidence >= threshold {
        "medium"
    } else {
        "low"
    };
    let reason_code = if confidence >= threshold && disagreement_count == 0 {
        "majority_unanimous"
    } else if confidence >= threshold {
        "majority_with_outliers"
    } else {
        "insufficient_majority"
    };
    let recommended_action = if confidence >= threshold && disagreement_count == 0 {
        "accept_majority"
    } else if confidence >= threshold {
        "accept_with_outlier_review"
    } else {
        "request_additional_agents"
    };

    json!({
        "consensus_reached": confidence >= threshold,
        "reason_code": reason_code,
        "confidence": confidence,
        "confidence_band": confidence_band,
        "threshold": threshold,
        "sample_size": reports.len(),
        "extractable_count": extractable_count,
        "group_count": groups.len(),
        "agreement_count": leader_group.len(),
        "disagreement_count": disagreement_count,
        "outlier_rate": outlier_rate,
        "dominant_fingerprint": clean_text(leader_fp, 24),
        "agreed_value": agreed_value,
        "recommended_action": recommended_action,
        "outliers": outliers,
        "fields": fields,
    })
}

fn default_swarm_test_spawn_options() -> SpawnOptions {
    SpawnOptions {
        verify: true,
        timeout_ms: 1_000,
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
    }
}

fn run_test_recursive(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let levels = parse_u8_flag(argv, "levels", 5);
    let mut options = default_swarm_test_spawn_options();
    options.timeout_ms = parse_u64_flag(argv, "timeout-ms", 1_000);

    let result = recursive_spawn_with_tracking(
        state,
        None,
        &format!("recursive-test-{levels}"),
        levels,
        levels.saturating_add(1),
        &options,
    )?;

    Ok(json!({
        "ok": true,
        "test": "recursive",
        "levels_requested": levels,
        "levels_completed": result
            .get("lineage")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0),
        "result": result
    }))
}

fn run_test_byzantine(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let agent_count = parse_u64_flag(argv, "agents", 5).max(1);
    let corrupt_count = parse_u64_flag(argv, "corrupt", 2).min(agent_count);
    let threshold = parse_f64_flag(argv, "threshold", 0.6);

    state.byzantine_test_mode = true;
    let mut reports = Vec::new();
    for idx in 0..agent_count {
        let is_corrupt = idx < corrupt_count;
        let values = if is_corrupt {
            let mut map = BTreeMap::new();
            map.insert("file".to_string(), Value::String("SOUL.md".to_string()));
            map.insert("file_size".to_string(), Value::Number(9999u64.into()));
            map.insert("word_count".to_string(), Value::Number(5000u64.into()));
            map.insert(
                "first_line".to_string(),
                Value::String("FAKE DATA HERE".to_string()),
            );
            map
        } else {
            let mut map = BTreeMap::new();
            map.insert("file".to_string(), Value::String("SOUL.md".to_string()));
            map.insert("file_size".to_string(), Value::Number(1847u64.into()));
            map.insert("word_count".to_string(), Value::Number(292u64.into()));
            map.insert(
                "first_line".to_string(),
                Value::String("# SOUL.md".to_string()),
            );
            map
        };
        reports.push(AgentReport {
            agent_id: format!("agent-{:02}", idx + 1),
            values,
        });
    }

    let fields = vec![
        "file".to_string(),
        "file_size".to_string(),
        "word_count".to_string(),
        "first_line".to_string(),
    ];
    let consensus = evaluate_consensus(&reports, &fields, threshold);
    let outliers = consensus
        .get("outliers")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);

    Ok(json!({
        "ok": true,
        "test": "byzantine",
        "byzantine_test_mode": state.byzantine_test_mode,
        "agents": agent_count,
        "corrupt_requested": corrupt_count,
        "corrupt_detected": outliers,
        "consensus": consensus,
        "truth_constraints_disabled_for_testing": true
    }))
}

fn run_test_concurrency(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let agents = parse_u64_flag(argv, "agents", 25).max(1);
    let metrics_detailed = parse_flag(argv, "metrics")
        .map(|value| value.eq_ignore_ascii_case("detailed"))
        .unwrap_or(true);

    let mut options = default_swarm_test_spawn_options();
    options.metrics_detailed = metrics_detailed;

    let mut report_total = 0u64;

    for idx in 0..agents {
        let task = format!("concurrency-test-{idx}");
        let spawned = spawn_single(state, None, &task, 64, &options)?;
        if let Some(metrics) = spawned.get("metrics") {
            report_total = report_total.saturating_add(
                metrics
                    .get("report_back_latency_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
            );
        }
    }

    let summary = queue_metrics_snapshot(state);
    let denom = agents.max(1) as f64;
    Ok(json!({
        "ok": true,
        "test": "concurrency",
        "agents": agents,
        "metrics": {
            "queue_wait_avg_ms": summary.get("queue_wait_ms").and_then(|row| row.get("avg")).cloned().unwrap_or(json!(0.0)),
            "queue_wait_p95_ms": summary.get("queue_wait_ms").and_then(|row| row.get("p95")).cloned().unwrap_or(json!(0)),
            "execution_avg_ms": summary.get("execution_ms").and_then(|row| row.get("avg")).cloned().unwrap_or(json!(0.0)),
            "execution_p95_ms": summary.get("execution_ms").and_then(|row| row.get("p95")).cloned().unwrap_or(json!(0)),
            "report_back_avg_ms": report_total as f64 / denom,
            "total_latency_avg_ms": summary.get("total_latency_ms").and_then(|row| row.get("avg")).cloned().unwrap_or(json!(0.0)),
            "total_latency_p95_ms": summary.get("total_latency_ms").and_then(|row| row.get("p95")).cloned().unwrap_or(json!(0)),
            "breakdown_available": true,
        }
    }))
}

fn run_test_hierarchy(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let requested_agents = parse_u64_flag(argv, "agents", 10).max(1) as usize;
    let fanout = parse_u64_flag(argv, "fanout", 8).max(2).min(64) as usize;
    let max_depth = parse_u8_flag(argv, "max-depth", 64).max(2);
    let metrics_detailed = parse_flag(argv, "metrics")
        .map(|value| value.eq_ignore_ascii_case("detailed"))
        .unwrap_or(true);
    let timeout_ms = parse_u64_flag(argv, "timeout-ms", 1_000);
    let task_prefix =
        parse_flag(argv, "task-prefix").unwrap_or_else(|| "hierarchy-test".to_string());

    let mut options = default_swarm_test_spawn_options();
    options.timeout_ms = timeout_ms;
    options.metrics_detailed = metrics_detailed;
    options.role = Some("manager".to_string());

    let root_payload = spawn_single(
        state,
        None,
        &format!("{task_prefix}-root"),
        max_depth,
        &options,
    )?;
    let root_id = root_payload
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "hierarchy_missing_root_session_id".to_string())?
        .to_string();

    let mut frontier = std::collections::VecDeque::new();
    frontier.push_back((root_id.clone(), 0u16));

    let mut spawned_agents = 1usize;
    let mut parent_child_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut depth_distribution: BTreeMap<String, usize> = BTreeMap::new();
    depth_distribution.insert("0".to_string(), 1);
    let mut max_observed_depth = 0u16;

    while spawned_agents < requested_agents {
        let Some((parent_id, parent_depth)) = frontier.pop_front() else {
            return Err("hierarchy_frontier_exhausted_before_target".to_string());
        };

        let remaining = requested_agents.saturating_sub(spawned_agents);
        if remaining == 0 {
            break;
        }

        let children_to_spawn = fanout.min(remaining);
        for offset in 0..children_to_spawn {
            let child_depth = parent_depth.saturating_add(1);
            let child_task = format!(
                "{task_prefix}-d{child_depth}-n{}",
                spawned_agents.saturating_add(offset).saturating_add(1)
            );
            let child_payload = spawn_single(
                state,
                Some(parent_id.as_str()),
                &child_task,
                max_depth,
                &options,
            )?;
            let child_id = child_payload
                .get("session_id")
                .and_then(Value::as_str)
                .ok_or_else(|| "hierarchy_missing_child_session_id".to_string())?
                .to_string();

            frontier.push_back((child_id, child_depth));
            spawned_agents = spawned_agents.saturating_add(1);
            *parent_child_counts.entry(parent_id.clone()).or_insert(0) += 1;
            *depth_distribution
                .entry(child_depth.to_string())
                .or_insert(0) += 1;
            max_observed_depth = max_observed_depth.max(child_depth);

            if spawned_agents >= requested_agents {
                break;
            }
        }
    }

    let mut orphan_children = 0usize;
    let mut missing_parent_refs = 0usize;
    let mut fanout_overflow = 0usize;
    for (session_id, session) in &state.sessions {
        if session_id == &root_id {
            continue;
        }
        match session.parent_id.as_deref() {
            Some(parent_id) if state.sessions.contains_key(parent_id) => {}
            Some(_) => missing_parent_refs = missing_parent_refs.saturating_add(1),
            None => orphan_children = orphan_children.saturating_add(1),
        }
    }

    for child_count in parent_child_counts.values() {
        if *child_count > fanout {
            fanout_overflow = fanout_overflow.saturating_add(1);
        }
    }

    let manager_count = parent_child_counts.len();
    let leaf_count = spawned_agents.saturating_sub(manager_count);
    let recommended_manager_fanout = recommended_manager_fanout_for_target(requested_agents);
    let queue_metrics = queue_metrics_snapshot(state);

    Ok(json!({
        "ok": spawned_agents == requested_agents
            && orphan_children == 0
            && missing_parent_refs == 0
            && fanout_overflow == 0,
        "test": "hierarchy",
        "agents_requested": requested_agents,
        "agents_spawned": spawned_agents,
        "fanout": fanout,
        "max_depth": max_observed_depth,
        "manager_count": manager_count,
        "leaf_count": leaf_count,
        "manager_ratio": if spawned_agents == 0 {
            0.0
        } else {
            manager_count as f64 / spawned_agents as f64
        },
        "depth_distribution": depth_distribution,
        "lineage_validation": {
            "orphan_children": orphan_children,
            "missing_parent_refs": missing_parent_refs,
            "fanout_overflow": fanout_overflow,
            "lineage_ok": orphan_children == 0 && missing_parent_refs == 0 && fanout_overflow == 0,
        },
        "recommended_manager_fanout_for_scale": recommended_manager_fanout,
        "queue_metrics": {
            "queue_wait_avg_ms": queue_metrics.get("queue_wait_ms").and_then(|row| row.get("avg")).cloned().unwrap_or(json!(0.0)),
            "execution_avg_ms": queue_metrics.get("execution_ms").and_then(|row| row.get("avg")).cloned().unwrap_or(json!(0.0)),
            "total_latency_avg_ms": queue_metrics.get("total_latency_ms").and_then(|row| row.get("avg")).cloned().unwrap_or(json!(0.0)),
        },
    }))
}

fn run_test_budget(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let budget = parse_u64_flag(argv, "budget", 120).max(1) as u32;
    let warning_at = parse_f64_flag(argv, "warning-at", 0.8).clamp(0.0, 1.0) as f32;
    let assert_hard_enforcement = parse_bool_flag(argv, "assert-hard-enforcement", true);
    let exhaustion_action = BudgetAction::from_flag(parse_flag(argv, "on-budget-exhausted"));
    if assert_hard_enforcement && exhaustion_action != BudgetAction::FailHard {
        return Err("budget_test_hard_enforcement_requires_fail_action".to_string());
    }
    let expect_fail = parse_bool_flag(argv, "expect-fail", assert_hard_enforcement);
    let adaptive_complexity =
        parse_bool_flag(argv, "adaptive-complexity", !assert_hard_enforcement);
    let task = parse_flag(argv, "task").unwrap_or_else(|| {
        "Write a 10-page essay on quantum physics with detailed references".to_string()
    });
    if assert_hard_enforcement {
        let planned_task = if adaptive_complexity {
            scale_task_complexity(&task, budget)
        } else {
            task.clone()
        };
        let planned_tokens = estimate_tool_plan(&planned_task, Some(budget))
            .iter()
            .map(|(_, tokens)| *tokens)
            .sum::<u32>();
        if planned_tokens <= budget {
            return Err(format!(
                "budget_test_hard_enforcement_not_triggered:planned={planned_tokens}:budget={budget}:set_lower_budget_or_disable_adaptive"
            ));
        }
    }

    let mut options = default_swarm_test_spawn_options();
    options.token_budget = Some(budget);
    options.token_warning_threshold = warning_at;
    options.budget_exhaustion_action = exhaustion_action.clone();
    options.adaptive_complexity = adaptive_complexity;

    let result = spawn_single(state, None, &task, 8, &options);
    if expect_fail {
        return match result {
            Ok(_) => Err("expected_budget_failure_but_spawn_succeeded".to_string()),
            Err(reason) if reason.contains("token_budget_exceeded") => Ok(json!({
                "ok": true,
                "test": "budget",
                "hard_enforcement": assert_hard_enforcement,
                "expect_fail": true,
                "expectation_met": true,
                "reason": reason,
                "budget": budget,
                "warning_at": warning_at,
                "on_budget_exhausted": exhaustion_action.as_label(),
            })),
            Err(reason) => Err(format!("unexpected_failure_reason:{reason}")),
        };
    }

    match result {
        Ok(payload) => Ok(json!({
            "ok": true,
            "test": "budget",
            "hard_enforcement": assert_hard_enforcement,
            "expect_fail": false,
            "expectation_met": true,
            "budget": budget,
            "warning_at": warning_at,
            "on_budget_exhausted": exhaustion_action.as_label(),
            "payload": payload,
        })),
        Err(reason) => Err(format!("unexpected_budget_failure:{reason}")),
    }
}

fn budget_report_for_session(state: &SwarmState, session_id: &str) -> Result<Value, String> {
    let Some(session) = state.sessions.get(session_id) else {
        return Err(format!("unknown_session:{session_id}"));
    };
    let Some(telemetry) = session.budget_telemetry.as_ref() else {
        return Err(format!("budget_not_configured_for_session:{session_id}"));
    };
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_budget_report",
        "session_id": session_id,
        "report": telemetry.generate_report(),
    }))
}

fn session_context_flag_true(session: &SessionMetadata, key: &str) -> bool {
    session
        .context_vars
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn session_report_goal_met(session: &SessionMetadata) -> bool {
    session
        .report
        .as_ref()
        .and_then(|report| report.get("result"))
        .and_then(Value::as_str)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "goal_met" | "completed" | "done" | "success"
            )
        })
        .unwrap_or(false)
}

fn evaluate_should_terminate_contract(session: &SessionMetadata, now_ms: u64) -> Value {
    let goal_met = session_context_flag_true(session, "goal_met") || session_report_goal_met(session);
    let budget_exceeded = session
        .budget_telemetry
        .as_ref()
        .map(|telemetry| {
            telemetry.budget_exhausted
                || (telemetry.budget_config.max_tokens > 0 && telemetry.remaining_tokens() == 0)
        })
        .unwrap_or(false);
    let policy_stop = session_context_flag_true(session, "policy_stop")
        || session_context_flag_true(session, "stop_requested")
        || session_context_flag_true(session, "terminate_now");

    let stalled = session
        .persistent
        .as_ref()
        .map(|runtime| {
            let stall_window_ms = runtime
                .config
                .check_in_interval_sec
                .saturating_mul(1000)
                .saturating_mul(3);
            let overdue = now_ms > runtime.next_check_in_ms.saturating_add(stall_window_ms);
            overdue && runtime.check_in_count >= 3
        })
        .unwrap_or(false);

    let mut reason = "continue".to_string();
    let mut detail = "none".to_string();
    let should_terminate = if goal_met {
        reason = "goal_met".to_string();
        detail = "goal_marker_detected".to_string();
        true
    } else if budget_exceeded {
        reason = "budget_exceeded".to_string();
        detail = "budget_guard_triggered".to_string();
        true
    } else if stalled {
        reason = "stalled".to_string();
        detail = "check_in_overdue".to_string();
        true
    } else if policy_stop {
        reason = "policy_stop".to_string();
        detail = "policy_stop_requested".to_string();
        true
    } else if session
        .persistent
        .as_ref()
        .map(|runtime| now_ms >= runtime.deadline_ms)
        .unwrap_or(false)
    {
        reason = "policy_stop".to_string();
        detail = "lifespan_deadline_reached".to_string();
        true
    } else {
        false
    };

    json!({
        "should_terminate": should_terminate,
        "reason": reason,
        "detail": detail,
        "contract": {
            "goal_met": goal_met,
            "budget_exceeded": budget_exceeded,
            "stalled": stalled,
            "policy_stop": policy_stop,
        },
        "deterministic": true,
    })
}

fn termination_status_for_reason(reason: &str) -> &'static str {
    match reason {
        "goal_met" => "completed",
        "budget_exceeded" => "terminated_budget_exceeded",
        "stalled" => "terminated_stalled",
        "policy_stop" => "terminated_policy_stop",
        _ => "terminated",
    }
}

fn tick_persistent_sessions(
    state: &mut SwarmState,
    now_ms: u64,
    max_check_ins: u64,
) -> Result<Value, String> {
    let mut processed_sessions = 0u64;
    let mut check_ins = 0u64;
    let mut finalized_sessions = Vec::new();
    let mut reports = Vec::new();

    for session_id in persistent_session_ids(state) {
        let mut local_processed = 0u64;
        loop {
            if local_processed >= max_check_ins {
                break;
            }
            let termination_contract = match state.sessions.get(&session_id) {
                Some(session) => evaluate_should_terminate_contract(session, now_ms),
                None => json!({
                    "should_terminate": false,
                    "reason": "missing_session",
                    "detail": "session_missing",
                    "contract": {
                        "goal_met": false,
                        "budget_exceeded": false,
                        "stalled": false,
                        "policy_stop": false
                    },
                    "deterministic": true,
                }),
            };
            let should_finalize = termination_contract
                .get("should_terminate")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let should_check_in = match state
                .sessions
                .get(&session_id)
                .and_then(|session| session.persistent.as_ref())
            {
                Some(runtime) => should_finalize || now_ms >= runtime.next_check_in_ms,
                None => false,
            };
            if !should_check_in {
                break;
            }

            let Some(session) = state.sessions.get_mut(&session_id) else {
                break;
            };
            let terminate_reason = termination_contract
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("policy_stop")
                .to_string();
            let parent_id = session.parent_id.clone();
            let role_card = session.role_card.clone();
            let report = if should_finalize {
                let result = perform_persistent_check_in(session, &terminate_reason, true)?;
                session.status = termination_status_for_reason(&terminate_reason).to_string();
                if let Some(runtime) = session.persistent.as_mut() {
                    runtime.terminated_at_ms = Some(now_ms);
                    runtime.terminated_reason = Some(terminate_reason.clone());
                }
                mark_service_instance_unhealthy(state, &session_id);
                settle_budget_reservation(state, &session_id);
                finalized_sessions.push(session_id.clone());
                append_event(
                    state,
                    json!({
                        "type": "swarm_session_terminated",
                        "session_id": session_id,
                        "parent_id": parent_id.clone(),
                        "lineage_parent_id": parent_id.clone(),
                        "reason": terminate_reason,
                        "should_terminate": termination_contract.clone(),
                        "role_card": role_card.clone(),
                        "timestamp": now_iso(),
                    }),
                );
                result
            } else {
                let result = perform_persistent_check_in(session, "interval", false)?;
                if let Some(runtime) = session.persistent.as_mut() {
                    runtime.next_check_in_ms = now_ms
                        .saturating_add(runtime.config.check_in_interval_sec.saturating_mul(1000));
                }
                result
            };

            reports.push(json!({
                "session_id": session_id,
                "result": report,
                "should_terminate": termination_contract,
            }));
            local_processed = local_processed.saturating_add(1);
            check_ins = check_ins.saturating_add(1);
            if should_finalize {
                break;
            }
        }
        if local_processed > 0 {
            processed_sessions = processed_sessions.saturating_add(1);
        }
    }

    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_tick",
        "processed_sessions": processed_sessions,
        "check_ins": check_ins,
        "finalized_sessions": finalized_sessions,
        "reports": reports,
    }))
}

fn sessions_wake(state: &mut SwarmState, session_id: &str, now_ms: u64) -> Result<Value, String> {
    let Some(session) = state.sessions.get_mut(session_id) else {
        return Err(format!("unknown_session:{session_id}"));
    };
    if session.persistent.is_none() {
        return Err(format!("session_not_persistent:{session_id}"));
    }
    if !matches!(
        session.status.as_str(),
        "persistent_running" | "background_running"
    ) {
        return Err(format!("session_not_active:{session_id}"));
    }
    let report = perform_persistent_check_in(session, "manual_wake", false)?;
    if let Some(runtime) = session.persistent.as_mut() {
        runtime.next_check_in_ms =
            now_ms.saturating_add(runtime.config.check_in_interval_sec.saturating_mul(1000));
    }
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_wake",
        "session_id": session_id,
        "report": report,
    }))
}
