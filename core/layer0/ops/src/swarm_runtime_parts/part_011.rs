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

    let confidence = leader_group.len() as f64 / reports.len() as f64;
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

    json!({
        "consensus_reached": confidence >= threshold,
        "confidence": confidence,
        "threshold": threshold,
        "sample_size": reports.len(),
        "agreement_count": leader_group.len(),
        "agreed_value": agreed_value,
        "outliers": outliers,
        "fields": fields,
    })
}

fn run_test_recursive(state: &mut SwarmState, argv: &[String]) -> Result<Value, String> {
    let levels = parse_u8_flag(argv, "levels", 5);
    let options = SpawnOptions {
        verify: true,
        timeout_ms: parse_u64_flag(argv, "timeout-ms", 1_000),
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

    let options = SpawnOptions {
        verify: true,
        timeout_ms: 1_000,
        metrics_detailed,
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

    let options = SpawnOptions {
        verify: true,
        timeout_ms: 1_000,
        metrics_detailed: true,
        simulate_unreachable: false,
        byzantine: false,
        corruption_type: "data_falsification".to_string(),
        token_budget: Some(budget),
        token_warning_threshold: warning_at,
        budget_exhaustion_action: exhaustion_action.clone(),
        adaptive_complexity,
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
            let mut should_finalize = false;
            let should_check_in = match state
                .sessions
                .get(&session_id)
                .and_then(|session| session.persistent.as_ref())
            {
                Some(runtime) => {
                    if now_ms >= runtime.deadline_ms {
                        should_finalize = true;
                        true
                    } else {
                        now_ms >= runtime.next_check_in_ms
                    }
                }
                None => false,
            };
            if !should_check_in {
                break;
            }

            let Some(session) = state.sessions.get_mut(&session_id) else {
                break;
            };
            let report = if should_finalize {
                let result = perform_persistent_check_in(session, "lifespan_expired", true)?;
                session.status = "completed".to_string();
                if let Some(runtime) = session.persistent.as_mut() {
                    runtime.terminated_at_ms = Some(now_ms);
                    runtime.terminated_reason = Some("lifespan_expired".to_string());
                }
                mark_service_instance_unhealthy(state, &session_id);
                settle_budget_reservation(state, &session_id);
                finalized_sessions.push(session_id.clone());
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
