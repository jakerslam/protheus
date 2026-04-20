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
