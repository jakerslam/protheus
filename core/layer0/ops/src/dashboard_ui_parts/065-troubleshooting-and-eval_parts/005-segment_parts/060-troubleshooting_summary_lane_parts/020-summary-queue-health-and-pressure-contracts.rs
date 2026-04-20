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
