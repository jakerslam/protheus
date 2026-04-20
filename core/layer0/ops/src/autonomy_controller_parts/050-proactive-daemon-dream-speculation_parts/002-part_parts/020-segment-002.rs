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
