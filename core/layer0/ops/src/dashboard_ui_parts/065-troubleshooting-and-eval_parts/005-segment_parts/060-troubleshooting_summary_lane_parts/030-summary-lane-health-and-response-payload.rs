    let lane_health = dashboard_troubleshooting_recent_lane_health(&filtered_entries);
    let latest_loop_level = filtered_entries
        .last()
        .and_then(|row| row.pointer("/loop_detection/level").and_then(Value::as_str))
        .unwrap_or("none");
    let tooling_contract = dashboard_troubleshooting_recent_tooling_contract(&filtered_entries);
    let tooling_gate_ok = tooling_contract
        .get("gate_health_ok")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let provider_resolution_ok = tooling_contract
        .get("provider_resolution_ok")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let tooling_watchdog_not_triggered = !tooling_contract
        .get("watchdog_triggered")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tooling_completion_signal_ok = tooling_contract
        .get("completion_signal_ok")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let tooling_manual_intervention_not_required = !tooling_contract
        .get("manual_intervention_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tooling_contract_version_supported = tooling_contract
        .get("contract_version")
        .and_then(Value::as_str)
        .is_some_and(|value| value == "v1");
    let tooling_next_action_routable = tooling_contract
        .get("next_action_routable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tooling_llm_reliability_not_low = tooling_contract
        .get("llm_reliability_not_low")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let tooling_hallucination_pattern_not_detected = !tooling_contract
        .get("hallucination_pattern_detected")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tooling_placeholder_output_not_detected = !tooling_contract
        .get("placeholder_output_detected")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tooling_final_response_contract_ok = tooling_contract
        .get("final_response_contract_ok")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let tooling_no_result_pattern_not_detected = !tooling_contract
        .get("no_result_pattern_detected")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tooling_answer_contract_ok = tooling_contract
        .get("answer_contract_ok")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let response_gate = tooling_contract.get("response_gate").unwrap_or(&Value::Null);
    let recent_recovery_hints =
        dashboard_troubleshooting_recent_recovery_hints(&lane_health, severity_tier);
    let recent_health_checks = dashboard_troubleshooting_recent_health_checks(
        &lane_health,
        latest_loop_level,
        entry_count,
        filtered_out_count,
        recent_entries.len(),
        stale_rate,
        outbox_queue_pressure_tier,
        tooling_gate_ok,
        provider_resolution_ok,
        tooling_watchdog_not_triggered,
        tooling_completion_signal_ok,
        tooling_manual_intervention_not_required,
        tooling_contract_version_supported,
        tooling_next_action_routable,
        tooling_llm_reliability_not_low,
        tooling_hallucination_pattern_not_detected,
        tooling_placeholder_output_not_detected,
        tooling_final_response_contract_ok,
        tooling_no_result_pattern_not_detected,
        tooling_answer_contract_ok,
        response_gate,
    );
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.summary".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_summary",
            "filters": {
                "applied": filters_applied,
                "classification": classification_filter,
                "error": error_filter,
                "no_match": no_filter_matches
            },
            "window": {
                "applied": window_filter_applied,
                "window_seconds": window_seconds,
                "since_epoch_s": effective_since_epoch,
                "filtered_out_count": filtered_out_count
            },
            "top_failure_cluster": {
                "top_error": top_error,
                "top_error_count": top_error_count,
                "top_error_share": top_error_share,
                "top_classification": top_class,
                "top_classification_count": top_class_count,
                "top_classification_share": top_classification_share,
                "severity_tier": severity_tier
            },
            "recent": {
                "entry_count": entry_count,
                "total_entry_count": recent_entries.len(),
                "failure_count": failure_count,
                "failure_rate": failure_rate,
                "stale_count": stale_count,
                "stale_rate": stale_rate,
                "lane_health": lane_health,
                "recovery_hints": recent_recovery_hints,
                "tooling_contract": tooling_contract,
                "checks": recent_health_checks,
                "error_histogram": error_hist,
                "classification_histogram": class_hist,
                "entries": recent
            },
            "queues": {
                "eval_depth": eval_queue.len(),
                "outbox_depth": outbox.len(),
                "deadletter_depth": deadletter.len(),
                "outbox_health": {
                    "ready_count": outbox_ready_count,
                    "ready_ratio": outbox_ready_ratio,
                    "blocked_ratio": outbox_blocked_ratio,
                    "fresh_count": outbox_fresh_count,
                    "fresh_ratio": outbox_fresh_ratio,
                    "aging_count": outbox_aging_count,
                    "aging_ratio": outbox_aging_ratio,
                    "stale_count": outbox_stale_count,
                    "stale_ratio": outbox_stale_ratio,
                    "oldest_age_seconds": outbox_oldest_age_seconds,
                    "age_p95_seconds": outbox_age_p95_seconds,
                    "health_score": outbox_health_score,
                    "health_tier": outbox_health_tier,
                    "health_reason": outbox_health_reason,
                    "queue_action_hint": outbox_queue_action_hint,
                    "retry_due_within_60s_count": outbox_retry_due_within_60s_count,
                    "retry_due_within_300s_count": outbox_retry_due_within_300s_count,
                    "retry_due_within_900s_count": outbox_retry_due_within_900s_count,
                    "retry_pressure_tier": outbox_retry_pressure_tier,
                    "queue_pressure_score": outbox_queue_pressure_score,
                    "queue_pressure_tier": outbox_queue_pressure_tier,
                    "queue_pressure_action_hint": outbox_queue_pressure_action_hint,
                    "queue_pressure_priority": outbox_queue_pressure_priority,
                    "queue_pressure_recommended_lane": outbox_queue_pressure_recommended_lane,
                    "queue_pressure_escalation_required": outbox_queue_pressure_escalation_required,
                    "queue_pressure_escalation_reason": outbox_queue_pressure_escalation_reason,
                    "queue_pressure_runbook_id": outbox_queue_pressure_runbook_id,
                    "queue_pressure_escalation_owner": outbox_queue_pressure_escalation_owner,
                    "queue_pressure_sla_minutes": outbox_queue_pressure_sla_minutes,
                    "queue_pressure_escalation_lane": outbox_queue_pressure_escalation_lane,
                    "queue_pressure_deadline_epoch_s": outbox_queue_pressure_deadline_epoch_s,
                    "queue_pressure_deadline_remaining_seconds": outbox_queue_pressure_deadline_remaining_seconds,
                    "queue_pressure_breach": outbox_queue_pressure_breach,
                    "queue_pressure_breach_reason": outbox_queue_pressure_breach_reason,
                    "queue_pressure_breach_detected_at_epoch_s": outbox_queue_pressure_breach_detected_at_epoch_s,
                    "queue_pressure_blocking_kind": outbox_queue_pressure_blocking_kind,
                    "queue_pressure_auto_retry_allowed": outbox_queue_pressure_auto_retry_allowed,
                    "queue_pressure_execution_policy": outbox_queue_pressure_execution_policy,
                    "queue_pressure_manual_gate_required": outbox_queue_pressure_manual_gate_required,
                    "queue_pressure_manual_gate_reason": outbox_queue_pressure_manual_gate_reason,
                    "queue_pressure_requeue_strategy": outbox_queue_pressure_requeue_strategy,
                    "queue_pressure_can_execute_without_human": outbox_queue_pressure_can_execute_without_human,
                    "queue_pressure_execution_window": outbox_queue_pressure_execution_window,
                    "queue_pressure_manual_gate_timeout_seconds": outbox_queue_pressure_manual_gate_timeout_seconds,
                    "queue_pressure_next_action_after_seconds": outbox_queue_pressure_next_action_after_seconds,
                    "queue_pressure_next_action_kind": outbox_queue_pressure_next_action_kind,
                    "queue_pressure_retry_window_class": outbox_queue_pressure_retry_window_class,
                    "queue_pressure_readiness_state": outbox_queue_pressure_readiness_state,
                    "queue_pressure_readiness_reason": outbox_queue_pressure_readiness_reason,
                    "queue_pressure_automation_safe": outbox_queue_pressure_automation_safe,
                    "queue_pressure_decision_route_hint": outbox_queue_pressure_decision_route_hint,
                    "queue_pressure_decision_urgency_tier": outbox_queue_pressure_decision_urgency_tier,
                    "queue_pressure_decision_retry_budget_class": outbox_queue_pressure_decision_retry_budget_class,
                    "queue_pressure_decision_lane_token": outbox_queue_pressure_decision_lane_token,
                    "queue_pressure_decision_dispatch_mode": outbox_queue_pressure_decision_dispatch_mode,
                    "queue_pressure_decision_manual_ack_required": outbox_queue_pressure_decision_manual_ack_required,
                    "queue_pressure_decision_execution_guard": outbox_queue_pressure_decision_execution_guard,
                    "queue_pressure_decision_followup_required": outbox_queue_pressure_decision_followup_required,
                    "queue_pressure_decision_vector_version": "v1",
                    "queue_pressure_decision_vector_key": outbox_queue_pressure_decision_vector_key,
                    "queue_pressure_decision_vector": dashboard_outbox_pressure_decision_vector(
                        outbox_queue_pressure_next_action_after_seconds,
                        outbox_queue_pressure_next_action_kind,
                        outbox_queue_pressure_retry_window_class,
                        outbox_queue_pressure_readiness_state,
                        outbox_queue_pressure_readiness_reason,
                        outbox_queue_pressure_automation_safe
                    ),
                    "queue_pressure_contract_version": dashboard_outbox_pressure_contract_version(),
                    "queue_pressure_contract_family": "dashboard_queue_pressure_contract_v1",
                    "queue_pressure_snapshot_epoch_s": now_epoch,
                    "queue_pressure_contract": dashboard_outbox_pressure_contract(
                        now_epoch,
                        outbox_queue_pressure_priority,
                        outbox_queue_pressure_action_hint,
                        outbox_queue_pressure_escalation_lane,
                        outbox_queue_pressure_runbook_id,
                        outbox_queue_pressure_escalation_owner,
                        outbox_queue_pressure_blocking_kind,
                        outbox_queue_pressure_auto_retry_allowed,
                        outbox_queue_pressure_execution_policy,
                        outbox_queue_pressure_manual_gate_required,
                        outbox_queue_pressure_manual_gate_reason,
                        outbox_queue_pressure_requeue_strategy,
                        outbox_queue_pressure_can_execute_without_human,
                        outbox_queue_pressure_execution_window,
                        outbox_queue_pressure_manual_gate_timeout_seconds,
                        outbox_queue_pressure_next_action_after_seconds,
                        outbox_queue_pressure_next_action_kind,
                        outbox_queue_pressure_retry_window_class,
                        outbox_queue_pressure_readiness_state,
                        outbox_queue_pressure_readiness_reason,
                        outbox_queue_pressure_automation_safe,
                        outbox_queue_pressure_decision_vector_key.as_str(),
                        outbox_queue_pressure_deadline_epoch_s,
                        outbox_queue_pressure_breach_reason
                    ),
                    "compaction_recommended": outbox_compaction_recommended,
                    "compaction_reason": outbox_compaction_reason
                },
                "outbox_error_histogram": dashboard_troubleshooting_outbox_reason_histogram(&outbox),
                "deadletter_reason_histogram": dashboard_troubleshooting_reason_histogram(&deadletter)
            },
            "recommendations": recommendations
        })),
    }
