{
    let limit = dashboard_payload_usize(payload, "limit", 20, 1, 200);
    let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
    let window_seconds = dashboard_summary_window_seconds(payload);
    let explicit_since_epoch = dashboard_payload_i64_with_bounds(
        payload,
        "since_epoch_s",
        0,
        0,
        i64::MAX,
    );
    let effective_since_epoch = if explicit_since_epoch > 0 {
        explicit_since_epoch
    } else if window_seconds > 0 {
        now_epoch.saturating_sub(window_seconds)
    } else {
        0
    };
    let window_filter_applied = effective_since_epoch > 0;
    let recent_entries = dashboard_troubleshooting_read_recent_entries(root);
    let classification_filter = dashboard_payload_first_string_filter(
        payload,
        &["classification_filter", "class_filter", "classifications"],
        12,
        80,
    );
    let error_filter =
        dashboard_payload_first_string_filter(payload, &["error_filter", "errors", "error_codes"], 12, 120);
    let filters_applied = !classification_filter.is_empty() || !error_filter.is_empty();
    let filtered_entries = recent_entries
        .iter()
        .filter(|row| {
            let error_code = clean_text(
                row.pointer("/workflow/error_code")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            )
            .to_ascii_lowercase();
            let class = clean_text(
                row.pointer("/workflow/classification")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                80,
            )
            .to_ascii_lowercase();
            let class_match = classification_filter.is_empty()
                || classification_filter
                    .iter()
                    .any(|value| dashboard_troubleshooting_filter_match(&class, value));
            let error_match = error_filter.is_empty()
                || error_filter
                    .iter()
                    .any(|value| dashboard_troubleshooting_filter_match(&error_code, value));
            let window_match = if window_filter_applied {
                let epoch = dashboard_troubleshooting_epoch_hint(row);
                epoch > 0 && epoch >= effective_since_epoch
            } else {
                true
            };
            class_match && error_match && window_match
        })
        .cloned()
        .collect::<Vec<_>>();
    let entry_count = filtered_entries.len();
    let filtered_out_count = recent_entries.len().saturating_sub(filtered_entries.len());
    let failure_count = filtered_entries
        .iter()
        .filter(|row| dashboard_troubleshooting_exchange_failed(row))
        .count();
    let stale_count = filtered_entries
        .iter()
        .filter(|row| row.get("stale").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let failure_rate = if entry_count == 0 {
        0.0
    } else {
        ((failure_count as f64) / (entry_count as f64) * 10_000.0).round() / 10_000.0
    };
    let stale_rate = if entry_count == 0 {
        0.0
    } else {
        ((stale_count as f64) / (entry_count as f64) * 10_000.0).round() / 10_000.0
    };
    let mut error_counts = HashMap::<String, i64>::new();
    let mut class_counts = HashMap::<String, i64>::new();
    for row in &filtered_entries {
        let error_code = clean_text(
            row.pointer("/workflow/error_code")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        )
        .to_ascii_lowercase();
        if !error_code.is_empty() {
            *error_counts.entry(error_code).or_insert(0) += 1;
        }
        let class = clean_text(
            row.pointer("/workflow/classification")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            80,
        )
        .to_ascii_lowercase();
        *class_counts.entry(class).or_insert(0) += 1;
    }
    let error_hist = dashboard_troubleshooting_sorted_histogram(error_counts, "error");
    let class_hist = dashboard_troubleshooting_sorted_histogram(class_counts, "classification");
    let top_error = error_hist
        .first()
        .and_then(|row| row.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    let top_class = class_hist
        .first()
        .and_then(|row| row.get("classification"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    let recommendations = dashboard_troubleshooting_eval_recommendations(top_error, top_class);
    let outbox = dashboard_troubleshooting_read_issue_outbox(root);
    let deadletter = dashboard_troubleshooting_read_deadletter_all(root);
    let eval_queue = dashboard_troubleshooting_read_eval_queue(root);
    let outbox_ready_count = outbox
        .iter()
        .filter(|row| {
            row.get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                <= now_epoch
        })
        .count();
    let outbox_ready_ratio = if outbox.is_empty() {
        0.0
    } else {
        ((outbox_ready_count as f64) / (outbox.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let outbox_blocked_ratio = if outbox.is_empty() {
        0.0
    } else {
        (((outbox.len().saturating_sub(outbox_ready_count)) as f64) / (outbox.len() as f64) * 10_000.0)
            .round()
            / 10_000.0
    };
    let outbox_stale_count = outbox
        .iter()
        .filter(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) > 900
        })
        .count();
    let outbox_fresh_count = outbox
        .iter()
        .filter(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) <= 300
        })
        .count();
    let outbox_aging_count = outbox
        .iter()
        .filter(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) > 300 && (now_epoch - epoch) <= 900
        })
        .count();
    let outbox_fresh_ratio = if outbox.is_empty() {
        0.0
    } else {
        ((outbox_fresh_count as f64) / (outbox.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let outbox_aging_ratio = if outbox.is_empty() {
        0.0
    } else {
        ((outbox_aging_count as f64) / (outbox.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let outbox_stale_ratio = if outbox.is_empty() {
        0.0
    } else {
        ((outbox_stale_count as f64) / (outbox.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let outbox_oldest_epoch = outbox
        .iter()
        .map(dashboard_troubleshooting_epoch_hint)
        .filter(|epoch| *epoch > 0)
        .min()
        .unwrap_or(0);
    let outbox_oldest_age_seconds = if outbox_oldest_epoch > 0 && outbox_oldest_epoch <= now_epoch {
        now_epoch - outbox_oldest_epoch
    } else {
        0
    };
    let outbox_age_p95_seconds = dashboard_troubleshooting_age_p95_seconds(&outbox, now_epoch);
    let outbox_retry_due_within_60s_count = outbox
        .iter()
        .filter(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            epoch > now_epoch && epoch <= now_epoch + 60
        })
        .count();
    let outbox_retry_due_within_300s_count = outbox
        .iter()
        .filter(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            epoch > now_epoch && epoch <= now_epoch + 300
        })
        .count();
    let outbox_retry_due_within_900s_count = outbox
        .iter()
        .filter(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            epoch > now_epoch && epoch <= now_epoch + 900
        })
        .count();
    let outbox_health_tier =
        dashboard_outbox_health_tier(outbox.len(), outbox_ready_ratio, outbox_blocked_ratio);
    let outbox_health_reason =
        dashboard_outbox_health_reason(outbox.len(), outbox_ready_ratio, outbox_blocked_ratio);
    let outbox_queue_action_hint = dashboard_outbox_queue_action_hint(outbox_health_tier);
    let outbox_health_score = dashboard_outbox_health_score(
        outbox.len(),
        outbox_ready_ratio,
        outbox_blocked_ratio,
        outbox_stale_ratio,
    );
    let outbox_retry_pressure_tier = dashboard_outbox_retry_pressure_tier(
        outbox.len(),
        outbox_retry_due_within_60s_count,
        outbox_retry_due_within_300s_count,
        outbox_retry_due_within_900s_count,
    );
    let outbox_queue_pressure_score = dashboard_outbox_pressure_score(
        outbox.len(),
        outbox_blocked_ratio,
        outbox_stale_ratio,
        outbox_retry_pressure_tier,
    );
    let outbox_queue_pressure_tier =
        dashboard_outbox_pressure_tier(outbox_queue_pressure_score);
    let outbox_queue_pressure_action_hint =
        dashboard_outbox_pressure_action_hint(outbox_queue_pressure_tier);
    let outbox_queue_pressure_priority =
        dashboard_outbox_pressure_priority(outbox_queue_pressure_tier);
    let outbox_queue_pressure_recommended_lane =
        dashboard_outbox_pressure_recommended_lane(outbox_queue_pressure_tier);
    let outbox_queue_pressure_escalation_required = outbox_queue_pressure_priority == "p0";
    let outbox_queue_pressure_escalation_reason = if outbox_queue_pressure_escalation_required {
        "high_queue_pressure"
    } else {
        "none"
    };
    let outbox_queue_pressure_runbook_id =
        dashboard_outbox_pressure_runbook_id(outbox_queue_pressure_tier);
    let outbox_queue_pressure_escalation_owner =
        dashboard_outbox_pressure_escalation_owner(outbox_queue_pressure_tier);
    let outbox_queue_pressure_sla_minutes =
        dashboard_outbox_pressure_sla_minutes(outbox_queue_pressure_tier);
    let outbox_queue_pressure_escalation_lane =
        dashboard_outbox_pressure_escalation_lane(outbox_queue_pressure_tier);
    let (
        outbox_queue_pressure_deadline_epoch_s,
        outbox_queue_pressure_breach,
        outbox_queue_pressure_breach_reason,
    ) = dashboard_outbox_pressure_deadline_breach(
        now_epoch,
        outbox_queue_pressure_tier,
        outbox_oldest_age_seconds,
        outbox_blocked_ratio,
        outbox_queue_pressure_sla_minutes,
        "outbox_oldest_age_seconds>1800",
        "outbox_blocked_ratio>=0.70",
    );
    let outbox_queue_pressure_deadline_remaining_seconds =
        dashboard_outbox_pressure_deadline_remaining_seconds(
            now_epoch,
            outbox_queue_pressure_deadline_epoch_s,
        );
    let outbox_queue_pressure_breach_detected_at_epoch_s =
        dashboard_outbox_pressure_breach_detected_at_epoch_s(
            now_epoch,
            outbox_queue_pressure_breach,
        );
    let outbox_queue_pressure_blocking_kind = dashboard_outbox_pressure_blocking_kind(
        outbox_queue_pressure_tier,
        outbox_queue_pressure_breach,
    );
    let outbox_queue_pressure_auto_retry_allowed =
        dashboard_outbox_pressure_auto_retry_allowed(outbox_queue_pressure_blocking_kind);
    let outbox_queue_pressure_execution_policy = dashboard_outbox_pressure_execution_policy(
        outbox_queue_pressure_blocking_kind,
        outbox_queue_pressure_auto_retry_allowed,
    );
    let outbox_queue_pressure_manual_gate_required =
        dashboard_outbox_pressure_manual_gate_required(outbox_queue_pressure_blocking_kind);
    let outbox_queue_pressure_manual_gate_reason =
        dashboard_outbox_pressure_manual_gate_reason(outbox_queue_pressure_blocking_kind);
    let outbox_queue_pressure_requeue_strategy =
        dashboard_outbox_pressure_requeue_strategy(outbox_queue_pressure_execution_policy);
    let outbox_queue_pressure_can_execute_without_human =
        dashboard_outbox_pressure_can_execute_without_human(outbox_queue_pressure_execution_policy);
    let outbox_queue_pressure_execution_window =
        dashboard_outbox_pressure_execution_window(outbox_queue_pressure_requeue_strategy);
    let outbox_queue_pressure_manual_gate_timeout_seconds =
        dashboard_outbox_pressure_manual_gate_timeout_seconds(outbox_queue_pressure_manual_gate_reason);
    let outbox_queue_pressure_next_action_after_seconds =
        dashboard_outbox_pressure_next_action_after_seconds(
            outbox_queue_pressure_execution_window,
            outbox_queue_pressure_manual_gate_timeout_seconds,
        );
    let outbox_queue_pressure_readiness_state = dashboard_outbox_pressure_readiness_state(
        outbox_queue_pressure_can_execute_without_human,
        outbox_queue_pressure_next_action_after_seconds,
    );
    let outbox_queue_pressure_next_action_kind = dashboard_outbox_pressure_next_action_kind(
        outbox_queue_pressure_can_execute_without_human,
        outbox_queue_pressure_next_action_after_seconds,
    );
    let outbox_queue_pressure_readiness_reason = dashboard_outbox_pressure_readiness_reason(
        outbox_queue_pressure_next_action_kind,
        outbox_queue_pressure_manual_gate_reason,
        outbox_queue_pressure_blocking_kind,
    );
    let outbox_queue_pressure_retry_window_class = dashboard_outbox_pressure_retry_window_class(
        outbox_queue_pressure_next_action_after_seconds,
    );
    let outbox_queue_pressure_automation_safe = dashboard_outbox_pressure_automation_safe(
        outbox_queue_pressure_auto_retry_allowed,
        outbox_queue_pressure_can_execute_without_human,
    );
    let outbox_queue_pressure_decision_vector_key = dashboard_outbox_pressure_decision_vector_key(
        outbox_queue_pressure_next_action_after_seconds,
        outbox_queue_pressure_next_action_kind,
        outbox_queue_pressure_retry_window_class,
        outbox_queue_pressure_readiness_state,
        outbox_queue_pressure_readiness_reason,
        outbox_queue_pressure_automation_safe,
    );
    let outbox_queue_pressure_decision_route_hint =
        dashboard_outbox_pressure_decision_route_hint(outbox_queue_pressure_next_action_kind);
    let outbox_queue_pressure_decision_urgency_tier =
        dashboard_outbox_pressure_decision_urgency_tier(
            outbox_queue_pressure_retry_window_class,
            outbox_queue_pressure_automation_safe,
        );
    let outbox_queue_pressure_decision_retry_budget_class =
        dashboard_outbox_pressure_decision_retry_budget_class(
            outbox_queue_pressure_retry_window_class,
            outbox_queue_pressure_automation_safe,
        );
    let outbox_queue_pressure_decision_lane_token =
        dashboard_outbox_pressure_decision_lane_token(
            outbox_queue_pressure_decision_route_hint,
            outbox_queue_pressure_decision_urgency_tier,
        );
    let outbox_queue_pressure_decision_dispatch_mode =
        dashboard_outbox_pressure_decision_dispatch_mode(outbox_queue_pressure_next_action_kind);
    let outbox_queue_pressure_decision_manual_ack_required =
        dashboard_outbox_pressure_decision_manual_ack_required(
            outbox_queue_pressure_next_action_kind,
            outbox_queue_pressure_automation_safe,
        );
    let outbox_queue_pressure_decision_execution_guard =
        dashboard_outbox_pressure_decision_execution_guard(
            outbox_queue_pressure_next_action_kind,
            outbox_queue_pressure_automation_safe,
        );
    let outbox_queue_pressure_decision_followup_required =
        dashboard_outbox_pressure_decision_followup_required(outbox_queue_pressure_next_action_kind);
    let outbox_compaction_recommended =
        outbox_stale_ratio >= 0.4 || outbox_oldest_age_seconds > 86_400;
    let outbox_compaction_reason = if outbox.is_empty() {
        "none"
    } else if outbox_stale_ratio >= 0.4 {
        "stale_ratio>=0.40"
    } else if outbox_oldest_age_seconds > 86_400 {
        "oldest_age_seconds>86400"
    } else {
        "none"
    };
    let mut recent = filtered_entries.clone();
    if recent.len() > limit {
        recent = recent.split_off(recent.len().saturating_sub(limit));
    }
    let no_filter_matches = filters_applied && filtered_entries.is_empty();
    let top_error_count = error_hist
        .first()
        .and_then(|row| row.get("count"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let top_class_count = class_hist
        .first()
        .and_then(|row| row.get("count"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let top_error_share = if entry_count == 0 {
        0.0
    } else {
        ((top_error_count as f64) / (entry_count as f64) * 10_000.0).round() / 10_000.0
    };
    let top_classification_share = if entry_count == 0 {
        0.0
    } else {
        ((top_class_count as f64) / (entry_count as f64) * 10_000.0).round() / 10_000.0
    };
    let severity_tier = if entry_count == 0 {
        "none"
    } else if failure_rate >= 0.6 || top_error_share >= 0.6 || top_classification_share >= 0.6 {
        "high"
    } else if failure_rate >= 0.3 || top_error_share >= 0.3 || top_classification_share >= 0.3 {
        "medium"
    } else {
        "low"
    };
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
}
