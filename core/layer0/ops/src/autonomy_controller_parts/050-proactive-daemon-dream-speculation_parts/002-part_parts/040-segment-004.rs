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
