fn run_proactive_daemon_daemon(root: &Path, argv: &[String]) -> i32 {
    let strict = parse_bool(parse_flag(argv, "strict").as_deref(), true);
    if let Some(mut denied) = conduit_guard(argv, strict) {
        return emit_receipt(root, &mut denied);
    }
    let action = clean_id(
        parse_flag(argv, "action").or_else(|| parse_positional(argv, 1)),
        "status",
    );
    let auto = parse_bool(parse_flag(argv, "auto").as_deref(), false);
    let force_cycle = parse_bool(parse_flag(argv, "force").as_deref(), false);
    let tick_ms = parse_u64(parse_flag(argv, "tick-ms").as_deref(), 5000, 1000, 60_000);
    let jitter_ms = parse_u64(
        parse_flag(argv, "jitter-ms").as_deref(),
        400,
        0,
        tick_ms.min(5_000),
    );
    let window_sec = parse_u64(parse_flag(argv, "window-sec").as_deref(), 900, 10, 86_400);
    let max_messages = parse_u64(parse_flag(argv, "max-proactive").as_deref(), 2, 1, 64);
    let blocking_budget_ms = parse_u64(
        parse_flag(argv, "block-budget-ms").as_deref(),
        15_000,
        50,
        120_000,
    );
    let dream_idle_ms = parse_u64(
        parse_flag(argv, "dream-idle-ms").as_deref(),
        6 * 60 * 60 * 1000,
        60_000,
        30 * 24 * 60 * 60 * 1000,
    );
    let dream_max_without_ms = parse_u64(
        parse_flag(argv, "dream-max-without-ms").as_deref(),
        24 * 60 * 60 * 1000,
        60_000,
        60 * 24 * 60 * 60 * 1000,
    );
    let policy_tier = clean(
        parse_flag(argv, "policy-tier")
            .unwrap_or_else(|| {
                std::env::var("PROACTIVE_DAEMON_POLICY_TIER")
                    .unwrap_or_else(|_| "observe".to_string())
            }),
        32,
    );
    let enabled_tool_surfaces = parse_tool_surfaces(
        parse_flag(argv, "tool-surfaces").or_else(|| parse_flag(argv, "tools")),
    );
    let brief_mode = parse_bool(parse_flag(argv, "brief").as_deref(), true);
    let now_ms = now_epoch_ms();

    let mut state = read_json(&proactive_daemon_state_path(root))
        .unwrap_or_else(proactive_daemon_default_state);
    ensure_proactive_daemon_state_shape(&mut state);
    state["heartbeat"]["tick_ms"] = json!(tick_ms);
    state["heartbeat"]["jitter_ms"] = json!(jitter_ms);
    state["proactive"]["window_sec"] = json!(window_sec);
    state["proactive"]["max_messages"] = json!(max_messages);
    state["proactive"]["brief_mode"] = json!(brief_mode);
    state["budgets"]["blocking_ms"] = json!(blocking_budget_ms);
    state["tool_surfaces"]["policy_tier"] = json!(policy_tier);
    state["tool_surfaces"]["enabled"] = Value::Array(
        enabled_tool_surfaces
            .iter()
            .cloned()
            .map(Value::String)
            .collect(),
    );
    state["dream"]["max_idle_ms"] = json!(dream_idle_ms);
    state["dream"]["max_without_dream_ms"] = json!(dream_max_without_ms);
    rollover_proactive_window(&mut state, now_ms);
    purge_expired_isolation_quarantine(&mut state, now_ms);

    let mut cycle_log_row = Value::Null;
    match action.as_str() {
        "pause" => {
            state["paused"] = json!(true);
            state["last_decision"] = json!("paused");
        }
        "resume" => {
            state["paused"] = json!(false);
            state["last_decision"] = json!("resumed");
        }
        "cycle" | "run" => {
            if !state
                .get("paused")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                let next_tick_after = state
                    .pointer("/heartbeat/next_tick_after_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                if !force_cycle && next_tick_after > now_ms {
                    state["last_decision"] = json!("tick_deferred");
                    state["tick_deferred_reason"] = json!("heartbeat_not_due");
                } else {
                    let swarm = read_json(&root.join("local/state/ops/swarm_runtime/latest.json"))
                        .unwrap_or_else(|| json!({}));
                    let dead_letters = swarm
                        .get("dead_letters")
                        .and_then(Value::as_array)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    let sessions = swarm
                        .get("sessions")
                        .and_then(Value::as_object)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    let mut intents = vec![];
                    let mut recurring_patterns = Vec::<String>::new();
                    let mut latest_hand_activity_ms = 0u64;
                    let mut latest_hand_for_dream = "hand-default".to_string();
                    if dead_letters > 0 {
                        intents.push(json!({"kind":"reliability","task":"sweep_dead_letters","priority":"medium","count":dead_letters}));
                        recurring_patterns.push("dead_letter_recurrence".to_string());
                    }
                    if sessions > 2000 {
                        intents.push(json!({"kind":"capacity","task":"autoscale_review","priority":"high","session_count":sessions}));
                        recurring_patterns.push("session_pressure_recurrence".to_string());
                    }
                    for hand_file in std::fs::read_dir(state_root(root).join("hands"))
                        .ok()
                        .into_iter()
                        .flat_map(|it| it.flatten())
                    {
                        let hand = read_json(&hand_file.path()).unwrap_or_else(|| json!({}));
                        let hand_id = clean_id(
                            hand.get("hand_id")
                                .and_then(Value::as_str)
                                .map(|v| v.to_string()),
                            "hand-default",
                        );
                        let activity_ms = value_epoch_ms(hand.get("updated_at"))
                            .or_else(|| value_epoch_ms(hand.get("last_cycle_at")))
                            .or_else(|| file_modified_epoch_ms(&hand_file.path()))
                            .unwrap_or(0);
                        if activity_ms >= latest_hand_activity_ms {
                            latest_hand_activity_ms = activity_ms;
                            latest_hand_for_dream = hand_id.clone();
                        }
                        let core_count = hand
                            .pointer("/memory/core")
                            .and_then(Value::as_array)
                            .map(|v| v.len())
                            .unwrap_or(0);
                        if core_count >= 96 {
                            intents.push(json!({"kind":"memory","task":"compact_hand_memory","hand_id":hand_id,"mode":"reactive","priority":"medium"}));
                        }
                    }
                    let last_dream_at_ms = state
                        .pointer("/dream/last_dream_at_ms")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let inactivity_elapsed_ms = if latest_hand_activity_ms == 0 {
                        u64::MAX
                    } else {
                        now_ms.saturating_sub(latest_hand_activity_ms)
                    };
                    let since_last_dream_ms = if last_dream_at_ms == 0 {
                        u64::MAX
                    } else {
                        now_ms.saturating_sub(last_dream_at_ms)
                    };
                    let dream_reason =
                        if latest_hand_activity_ms > 0 && inactivity_elapsed_ms >= dream_idle_ms {
                            Some("inactivity")
                        } else if since_last_dream_ms >= dream_max_without_ms {
                            Some("stale_without_dream")
                        } else {
                            None
                        };
                    if let Some(reason) = dream_reason {
                        intents.push(json!({
                            "kind":"memory",
                            "task":"dream_consolidation",
                            "priority":"medium",
                            "hand_id": latest_hand_for_dream,
                            "reason": reason,
                            "inactivity_ms": inactivity_elapsed_ms,
                            "since_last_dream_ms": since_last_dream_ms
                        }));
                    }
                    if !recurring_patterns.is_empty() {
                        intents.push(json!({
                            "kind": "maintenance",
                            "task": "pattern_log",
                            "priority": "low",
                            "patterns": recurring_patterns
                        }));
                    }
                    if auto {
                        for tool in &enabled_tool_surfaces {
                            if tool_surface_allowed(&policy_tier, tool.as_str()) {
                                intents.push(json!({
                                    "kind": "outbound",
                                    "task": tool,
                                    "priority": "low",
                                    "transport": "conduit",
                                    "policy_tier": policy_tier
                                }));
                            }
                        }
                    }
                    if intents.is_empty() {
                        intents.push(
                            json!({"kind":"maintenance","task":"pattern_log","priority":"low"}),
                        );
                    }
                    let mut executed = vec![];
                    let mut deferred = vec![];
                    let mut blocking_used_ms = 0u64;
                    let mut sent_in_window = state
                        .pointer("/proactive/sent_in_window")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    for intent in intents.iter() {
                        let estimate_ms = intent_estimated_blocking_ms(intent);
                        if auto {
                            let task = intent.get("task").and_then(Value::as_str).unwrap_or("");
                            if task_isolation_active(&state, task, now_ms) {
                                let recovery = record_recovery_strategy(
                                    &mut state,
                                    intent,
                                    "isolation_quarantine",
                                    now_ms,
                                );
                                deferred.push(json!({
                                    "intent": intent,
                                    "reason":"isolation_quarantine",
                                    "recovery": recovery
                                }));
                                continue;
                            }
                            if sent_in_window >= max_messages {
                                let recovery =
                                    record_recovery_strategy(&mut state, intent, "rate_limit", now_ms);
                                deferred.push(json!({"intent": intent, "reason":"rate_limit", "recovery": recovery}));
                                continue;
                            }
                            if blocking_used_ms.saturating_add(estimate_ms) > blocking_budget_ms {
                                let recovery = record_recovery_strategy(
                                    &mut state,
                                    intent,
                                    "blocking_budget",
                                    now_ms,
                                );
                                deferred.push(json!({
                                    "intent": intent,
                                    "reason":"blocking_budget",
                                    "recovery": recovery
                                }));
                                continue;
                            }
                            if task == "compact_hand_memory" {
                                if let Some(hand_id) = intent.get("hand_id").and_then(Value::as_str)
                                {
                                    let compact_result = compact_hand_memory(
                                        root,
                                        hand_id,
                                        "reactive",
                                        PROACTIVE_DAEMON_REACTIVE_COMPACTION_PRESSURE_RATIO,
                                        None,
                                    );
                                    if compact_result.is_err() {
                                        let isolation = record_isolation_event(
                                            &mut state,
                                            intent,
                                            "compact_failed",
                                            now_ms,
                                        );
                                        let recovery = record_recovery_strategy(
                                            &mut state,
                                            intent,
                                            "compact_failed",
                                            now_ms,
                                        );
                                        deferred.push(
                                            json!({
                                                "intent": intent,
                                                "reason":"compact_failed",
                                                "failure_isolation": isolation,
                                                "recovery": recovery
                                            }),
                                        );
                                        continue;
                                    }
                                    mark_recovery_success(&mut state, task);
                                }
                            } else if task == "dream_consolidation" {
                                let hand_id = intent
                                    .get("hand_id")
                                    .and_then(Value::as_str)
                                    .unwrap_or("hand-default");
                                match run_dream_consolidation_for_hand(root, hand_id) {
                                    Ok(event) => {
                                        let (cleanup_code, cleanup_payload) =
                                            crate::spine::execute_sleep_cleanup(
                                                root,
                                                true,
                                                false,
                                                "autonomy_dream",
                                            );
                                        state["dream"]["last_dream_at_ms"] = json!(now_ms);
                                        state["dream"]["last_dream_reason"] =
                                            intent.get("reason").cloned().unwrap_or(Value::Null);
                                        state["dream"]["last_dream_hand_id"] = json!(hand_id);
                                        state["dream"]["last_cleanup_ok"] = json!(
                                            cleanup_code == 0
                                                && cleanup_payload
                                                    .get("ok")
                                                    .and_then(Value::as_bool)
                                                    .unwrap_or(false)
                                        );
                                        sent_in_window = sent_in_window.saturating_add(1);
                                        blocking_used_ms =
                                            blocking_used_ms.saturating_add(estimate_ms);
                                        executed.push(json!({
                                            "intent": intent,
                                            "estimated_blocking_ms": estimate_ms,
                                            "dream_event": event,
                                            "cleanup": cleanup_payload
                                        }));
                                        mark_recovery_success(&mut state, task);
                                        continue;
                                    }
                                    Err(_) => {
                                        let isolation = record_isolation_event(
                                            &mut state,
                                            intent,
                                            "dream_failed",
                                            now_ms,
                                        );
                                        let recovery = record_recovery_strategy(
                                            &mut state,
                                            intent,
                                            "dream_failed",
                                            now_ms,
                                        );
                                        deferred.push(
                                            json!({
                                                "intent": intent,
                                                "reason":"dream_failed",
                                                "failure_isolation": isolation,
                                                "recovery": recovery
                                            }),
                                        );
                                        continue;
                                    }
                                }
                            } else if task == "pattern_log" {
                                let patterns = intent
                                    .get("patterns")
                                    .and_then(Value::as_array)
                                    .cloned()
                                    .unwrap_or_default()
                                    .iter()
                                    .filter_map(Value::as_str)
                                    .map(|v| clean(v, 80))
                                    .filter(|v| !v.is_empty())
                                    .collect::<Vec<_>>();
                                if deferred
                                    .iter()
                                    .any(|row| {
                                        matches!(
                                            row.get("reason").and_then(Value::as_str),
                                            Some("blocking_budget" | "rate_limit")
                                        )
                                    })
                                {
                                    if !patterns.iter().any(|row| row == "deferred_budget_recurrence")
                                    {
                                        let mut next = patterns.clone();
                                        next.push("deferred_budget_recurrence".to_string());
                                        let summary = update_pattern_log_state(
                                            &mut state,
                                            now_iso().as_str(),
                                            &next,
                                            json!({"dead_letters": dead_letters, "sessions": sessions}),
                                        );
                                        executed.push(json!({
                                            "intent": intent,
                                            "estimated_blocking_ms": estimate_ms,
                                            "pattern_summary": summary
                                        }));
                                        mark_recovery_success(&mut state, task);
                                        continue;
                                    }
                                }
                                let summary = update_pattern_log_state(
                                    &mut state,
                                    now_iso().as_str(),
                                    &patterns,
                                    json!({"dead_letters": dead_letters, "sessions": sessions}),
                                );
                                executed.push(json!({
                                    "intent": intent,
                                    "estimated_blocking_ms": estimate_ms,
                                    "pattern_summary": summary
                                }));
                                mark_recovery_success(&mut state, task);
                                continue;
                            } else if matches!(task, "subscribe_pr" | "push_notification" | "send_user_file")
                            {
                                match append_tool_surface_receipt(
                                    root,
                                    task,
                                    &policy_tier,
                                    intent,
                                    strict,
                                ) {
                                    Ok(receipt) => {
                                        let prev_written = state
                                            .pointer("/tool_surfaces/receipts_written")
                                            .and_then(Value::as_u64)
                                            .unwrap_or(0);
                                        state["tool_surfaces"]["receipts_written"] =
                                            json!(prev_written.saturating_add(1));
                                        state["tool_surfaces"]["last_receipt_id"] = receipt
                                            .get("receipt_id")
                                            .cloned()
                                            .unwrap_or(Value::Null);
                                        sent_in_window = sent_in_window.saturating_add(1);
                                        blocking_used_ms =
                                            blocking_used_ms.saturating_add(estimate_ms);
                                        executed.push(json!({
                                            "intent": intent,
                                            "estimated_blocking_ms": estimate_ms,
                                            "tool_surface_receipt": receipt
                                        }));
                                        mark_recovery_success(&mut state, task);
                                        continue;
                                    }
                                    Err(_) => {
                                        let isolation = record_isolation_event(
                                            &mut state,
                                            intent,
                                            "tool_surface_failed",
                                            now_ms,
                                        );
                                        let recovery = record_recovery_strategy(
                                            &mut state,
                                            intent,
                                            "tool_surface_failed",
                                            now_ms,
                                        );
                                        deferred.push(
                                            json!({
                                                "intent": intent,
                                                "reason":"tool_surface_failed",
                                                "failure_isolation": isolation,
                                                "recovery": recovery
                                            }),
                                        );
                                        continue;
                                    }
                                }
                            }
                            sent_in_window = sent_in_window.saturating_add(1);
                            blocking_used_ms = blocking_used_ms.saturating_add(estimate_ms);
                            let mut execution = json!({
                                "intent": intent,
                                "estimated_blocking_ms": estimate_ms
                            });
                            if task == "compact_hand_memory" {
                                execution["pressure_ratio"] =
                                    json!(PROACTIVE_DAEMON_REACTIVE_COMPACTION_PRESSURE_RATIO);
                            }
                            executed.push(execution);
                            mark_recovery_success(&mut state, task);
                        }
                    }
                    state["last_intents"] = Value::Array(intents.clone());
                    state["last_executed_intents"] = Value::Array(executed.clone());
                    state["last_deferred_intents"] = Value::Array(deferred.clone());
                    state["proactive"]["sent_in_window"] = json!(sent_in_window);

                    let cycles = state.get("cycles").and_then(Value::as_u64).unwrap_or(0) + 1;
                    state["cycles"] = json!(cycles);
                    state["last_cycle_at"] = json!(now_iso());
                    state["heartbeat"]["last_tick_ms"] = json!(now_ms);
                    let jitter_offset = deterministic_jitter_ms(cycles, jitter_ms);
                    state["heartbeat"]["next_tick_after_ms"] =
                        json!(now_ms.saturating_add(tick_ms).saturating_add(jitter_offset));
                    state["last_decision"] = if auto {
                        json!("cycle_executed_auto")
                    } else {
                        json!("cycle_executed_intent_only")
                    };
                    state["last_blocking_budget_used_ms"] = json!(blocking_used_ms);
                    cycle_log_row = json!({
                        "type": "proactive_daemon_tick",
                        "ts": now_iso(),
                        "action": action,
                        "auto": auto,
                        "brief_mode": brief_mode,
                        "intents": intents,
                        "executed": executed,
                        "deferred": deferred,
                        "blocking_budget_ms": blocking_budget_ms,
                        "blocking_used_ms": blocking_used_ms,
                        "window_sec": window_sec,
                        "max_proactive": max_messages,
                        "policy_tier": policy_tier,
                        "tool_surfaces": enabled_tool_surfaces,
                        "pattern_log": state.get("pattern_log").cloned().unwrap_or(Value::Null),
                        "failure_isolation": state.get("failure_isolation").cloned().unwrap_or(Value::Null),
                        "recovery_matrix": state.get("recovery_matrix").cloned().unwrap_or(Value::Null),
                        "state_hash": receipt_hash(&state)
                    });
                }
            } else {
                state["last_decision"] = json!("paused_skip");
            }
        }
        _ => {}
    }
    state["updated_at"] = json!(now_iso());
    if cycle_log_row != Value::Null {
        if let Err(err) = append_proactive_daemon_log(root, &cycle_log_row, strict) {
            let mut out = cli_error_receipt(argv, &format!("proactive_daemon_log_failed:{err}"), 2);
            out["type"] = json!("autonomy_proactive_daemon");
            return emit_receipt(root, &mut out);
        }
    }
    if let Err(err) = persist_proactive_daemon_state(root, &mut state, strict) {
        let mut out = cli_error_receipt(
            argv,
            &format!("proactive_daemon_state_persist_failed:{err}"),
            2,
        );
        out["type"] = json!("autonomy_proactive_daemon");
        return emit_receipt(root, &mut out);
    }
    let mut out = json!({
        "ok": true,
        "type": "autonomy_proactive_daemon",
        "lane": LANE_ID,
        "strict": strict,
        "action": action,
        "state": state,
        "policy": {
            "tick_ms": tick_ms,
            "jitter_ms": jitter_ms,
            "window_sec": window_sec,
            "max_proactive": max_messages,
            "blocking_budget_ms": blocking_budget_ms,
            "brief_mode": brief_mode,
            "dream_idle_ms": dream_idle_ms,
            "dream_max_without_ms": dream_max_without_ms,
            "policy_tier": policy_tier,
            "tool_surfaces": enabled_tool_surfaces
        },
        "claim_evidence": [
            {"id":"V6-AUTONOMY-003.1","claim":"proactive_daemon_background_daemon_tracks_runtime_state_and_receipts_actions"},
            {"id":"V6-AUTONOMY-003.2","claim":"proactive_daemon_generates_proactive_micro_tasks_with_policy_bounded_auto_execution"},
            {"id":"V6-AUTONOMY-004","claim":"proactive_daemon_tick_heartbeat_rate_limits_blocking_budget_and_append_only_daily_logs_enforce_proactive_safety"},
            {"id":"V6-AUTONOMY-004.1","claim":"proactive_daemon_isolates_failed_intent_contexts_with_quarantine_receipts_without_poisoning_other_intents"},
            {"id":"V6-AUTONOMY-004.2","claim":"proactive_daemon_uses_deterministic_recovery_strategy_matrix_retry_rollback_resync_escalate_by_failure_class"},
            {"id":"V6-AUTONOMY-004.3","claim":"proactive_daemon_logs_recurring_failure_and_latency_patterns_as_bounded_optimization_hints"},
            {"id":"V6-AUTONOMY-006","claim":"proactive_daemon_supports_policy_tiered_conduit_tool_surfaces_for_subscribe_pr_push_notification_and_send_user_file"}
        ]
    });
    emit_receipt(root, &mut out)
}
