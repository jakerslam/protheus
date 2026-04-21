
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
