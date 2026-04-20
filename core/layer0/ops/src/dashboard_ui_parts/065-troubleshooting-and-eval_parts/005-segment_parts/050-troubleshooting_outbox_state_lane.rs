fn dashboard_troubleshooting_outbox_state_lane(root: &Path, payload: &Value) -> LaneResult {
    let limit = dashboard_payload_usize(payload, "limit", 20, 1, 200);
    let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
    let rows = dashboard_troubleshooting_read_issue_outbox(root);
    let ready_count = rows
        .iter()
        .filter(|row| {
            row.get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                <= now_epoch
        })
        .count();
    let cooldown_blocked_count = rows.len().saturating_sub(ready_count);
    let next_retry_after_epoch_s = rows
        .iter()
        .filter_map(|row| row.get("next_retry_after_epoch_s").and_then(Value::as_i64))
        .filter(|epoch| *epoch > now_epoch)
        .min()
        .unwrap_or(0);
    let next_retry_after_seconds = if next_retry_after_epoch_s > now_epoch {
        next_retry_after_epoch_s - now_epoch
    } else {
        0
    };
    let mut display_rows = rows.clone();
    if display_rows.len() > limit {
        display_rows = display_rows.split_off(display_rows.len().saturating_sub(limit));
    }
    let mut sequence_cursor = 0i64;
    let display_rows = display_rows
        .into_iter()
        .map(|row| {
            sequence_cursor += 1;
            let epoch = dashboard_troubleshooting_epoch_hint(&row);
            let age_seconds = if epoch > 0 && epoch <= now_epoch {
                now_epoch - epoch
            } else {
                0
            };
            let stale = age_seconds > 900;
            let freshness_tier = if age_seconds <= 300 {
                "fresh"
            } else if age_seconds <= 900 {
                "aging"
            } else {
                "stale"
            };
            let mut out = row.clone();
            if let Some(obj) = out.as_object_mut() {
                obj.insert("source_sequence".to_string(), json!(sequence_cursor));
                obj.insert("age_seconds".to_string(), json!(age_seconds));
                obj.insert("stale".to_string(), json!(stale));
                obj.insert("freshness_tier".to_string(), json!(freshness_tier));
                obj.insert("source".to_string(), json!("issue_outbox"));
            }
            out
        })
        .collect::<Vec<_>>();
    let oldest_epoch = rows
        .iter()
        .map(dashboard_troubleshooting_epoch_hint)
        .filter(|epoch| *epoch > 0)
        .min()
        .unwrap_or(0);
    let oldest_age_seconds = if oldest_epoch > 0 && oldest_epoch <= now_epoch {
        now_epoch - oldest_epoch
    } else {
        0
    };
    let age_p95_seconds = dashboard_troubleshooting_age_p95_seconds(&rows, now_epoch);
    let max_attempts_observed = rows
        .iter()
        .filter_map(|row| row.get("attempts").and_then(Value::as_i64))
        .max()
        .unwrap_or(0);
    let blocked_ratio = if rows.is_empty() {
        0.0
    } else {
        ((cooldown_blocked_count as f64) / (rows.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let oldest_item_id = rows
        .iter()
        .filter_map(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            if epoch <= 0 {
                return None;
            }
            Some((
                epoch,
                clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
            ))
        })
        .min_by_key(|(epoch, _)| *epoch)
        .map(|(_, id)| id)
        .unwrap_or_default();
    let next_retry_item_id = rows
        .iter()
        .filter_map(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            if epoch <= now_epoch {
                return None;
            }
            Some((
                epoch,
                clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
            ))
        })
        .min_by_key(|(epoch, _)| *epoch)
        .map(|(_, id)| id)
        .unwrap_or_default();
    let retry_due_within_60s_count = rows
        .iter()
        .filter(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            epoch > now_epoch && epoch <= now_epoch + 60
        })
        .count();
    let retry_due_within_300s_count = rows
        .iter()
        .filter(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            epoch > now_epoch && epoch <= now_epoch + 300
        })
        .count();
    let retry_due_within_900s_count = rows
        .iter()
        .filter(|row| {
            let epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            epoch > now_epoch && epoch <= now_epoch + 900
        })
        .count();
    let fresh_count = rows
        .iter()
        .filter(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) <= 300
        })
        .count();
    let aging_count = rows
        .iter()
        .filter(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) > 300 && (now_epoch - epoch) <= 900
        })
        .count();
    let stale_count = rows
        .iter()
        .filter(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) > 900
        })
        .count();
    let stale_oldest_age_seconds = rows
        .iter()
        .filter_map(|row| {
            let epoch = dashboard_troubleshooting_epoch_hint(row);
            if epoch > 0 && epoch <= now_epoch && (now_epoch - epoch) > 900 {
                Some(now_epoch - epoch)
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0);
    let fresh_ratio = if rows.is_empty() {
        0.0
    } else {
        ((fresh_count as f64) / (rows.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let aging_ratio = if rows.is_empty() {
        0.0
    } else {
        ((aging_count as f64) / (rows.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let stale_ratio = if rows.is_empty() {
        0.0
    } else {
        ((stale_count as f64) / (rows.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let ready_ratio = if rows.is_empty() {
        0.0
    } else {
        ((ready_count as f64) / (rows.len() as f64) * 10_000.0).round() / 10_000.0
    };
    let health_tier = dashboard_outbox_health_tier(rows.len(), ready_ratio, blocked_ratio);
    let health_reason = dashboard_outbox_health_reason(rows.len(), ready_ratio, blocked_ratio);
    let queue_action_hint = dashboard_outbox_queue_action_hint(health_tier);
    let health_score =
        dashboard_outbox_health_score(rows.len(), ready_ratio, blocked_ratio, stale_ratio);
    let retry_pressure_tier = dashboard_outbox_retry_pressure_tier(
        rows.len(),
        retry_due_within_60s_count,
        retry_due_within_300s_count,
        retry_due_within_900s_count,
    );
    let queue_pressure_score = dashboard_outbox_pressure_score(
        rows.len(),
        blocked_ratio,
        stale_ratio,
        retry_pressure_tier,
    );
    let queue_pressure_tier = dashboard_outbox_pressure_tier(queue_pressure_score);
    let queue_pressure_action_hint =
        dashboard_outbox_pressure_action_hint(queue_pressure_tier);
    let queue_pressure_priority = dashboard_outbox_pressure_priority(queue_pressure_tier);
    let queue_pressure_recommended_lane =
        dashboard_outbox_pressure_recommended_lane(queue_pressure_tier);
    let queue_pressure_escalation_required = queue_pressure_priority == "p0";
    let queue_pressure_escalation_reason = if queue_pressure_escalation_required {
        "high_queue_pressure"
    } else {
        "none"
    };
    let queue_pressure_runbook_id = dashboard_outbox_pressure_runbook_id(queue_pressure_tier);
    let queue_pressure_escalation_owner =
        dashboard_outbox_pressure_escalation_owner(queue_pressure_tier);
    let queue_pressure_sla_minutes =
        dashboard_outbox_pressure_sla_minutes(queue_pressure_tier);
    let queue_pressure_escalation_lane =
        dashboard_outbox_pressure_escalation_lane(queue_pressure_tier);
    let (queue_pressure_deadline_epoch_s, queue_pressure_breach, queue_pressure_breach_reason) =
        dashboard_outbox_pressure_deadline_breach(
            now_epoch,
            queue_pressure_tier,
            stale_oldest_age_seconds,
            blocked_ratio,
            queue_pressure_sla_minutes,
            "stale_oldest_age_seconds>1800",
            "blocked_ratio>=0.70",
        );
    let queue_pressure_deadline_remaining_seconds =
        dashboard_outbox_pressure_deadline_remaining_seconds(
            now_epoch,
            queue_pressure_deadline_epoch_s,
        );
    let queue_pressure_breach_detected_at_epoch_s =
        dashboard_outbox_pressure_breach_detected_at_epoch_s(now_epoch, queue_pressure_breach);
    let queue_pressure_blocking_kind =
        dashboard_outbox_pressure_blocking_kind(queue_pressure_tier, queue_pressure_breach);
    let queue_pressure_auto_retry_allowed =
        dashboard_outbox_pressure_auto_retry_allowed(queue_pressure_blocking_kind);
    let queue_pressure_execution_policy = dashboard_outbox_pressure_execution_policy(
        queue_pressure_blocking_kind,
        queue_pressure_auto_retry_allowed,
    );
    let queue_pressure_manual_gate_required =
        dashboard_outbox_pressure_manual_gate_required(queue_pressure_blocking_kind);
    let queue_pressure_manual_gate_reason =
        dashboard_outbox_pressure_manual_gate_reason(queue_pressure_blocking_kind);
    let queue_pressure_requeue_strategy =
        dashboard_outbox_pressure_requeue_strategy(queue_pressure_execution_policy);
    let queue_pressure_can_execute_without_human =
        dashboard_outbox_pressure_can_execute_without_human(queue_pressure_execution_policy);
    let queue_pressure_execution_window =
        dashboard_outbox_pressure_execution_window(queue_pressure_requeue_strategy);
    let queue_pressure_manual_gate_timeout_seconds =
        dashboard_outbox_pressure_manual_gate_timeout_seconds(queue_pressure_manual_gate_reason);
    let queue_pressure_next_action_after_seconds = dashboard_outbox_pressure_next_action_after_seconds(
        queue_pressure_execution_window,
        queue_pressure_manual_gate_timeout_seconds,
    );
    let queue_pressure_readiness_state = dashboard_outbox_pressure_readiness_state(
        queue_pressure_can_execute_without_human,
        queue_pressure_next_action_after_seconds,
    );
    let queue_pressure_next_action_kind = dashboard_outbox_pressure_next_action_kind(
        queue_pressure_can_execute_without_human,
        queue_pressure_next_action_after_seconds,
    );
    let queue_pressure_readiness_reason = dashboard_outbox_pressure_readiness_reason(
        queue_pressure_next_action_kind,
        queue_pressure_manual_gate_reason,
        queue_pressure_blocking_kind,
    );
    let queue_pressure_retry_window_class =
        dashboard_outbox_pressure_retry_window_class(queue_pressure_next_action_after_seconds);
    let queue_pressure_automation_safe = dashboard_outbox_pressure_automation_safe(
        queue_pressure_auto_retry_allowed,
        queue_pressure_can_execute_without_human,
    );
    let queue_pressure_decision_vector_key = dashboard_outbox_pressure_decision_vector_key(
        queue_pressure_next_action_after_seconds,
        queue_pressure_next_action_kind,
        queue_pressure_retry_window_class,
        queue_pressure_readiness_state,
        queue_pressure_readiness_reason,
        queue_pressure_automation_safe,
    );
    let queue_pressure_decision_route_hint =
        dashboard_outbox_pressure_decision_route_hint(queue_pressure_next_action_kind);
    let queue_pressure_decision_urgency_tier =
        dashboard_outbox_pressure_decision_urgency_tier(
            queue_pressure_retry_window_class,
            queue_pressure_automation_safe,
        );
    let queue_pressure_decision_retry_budget_class =
        dashboard_outbox_pressure_decision_retry_budget_class(
            queue_pressure_retry_window_class,
            queue_pressure_automation_safe,
        );
    let queue_pressure_decision_lane_token = dashboard_outbox_pressure_decision_lane_token(
        queue_pressure_decision_route_hint,
        queue_pressure_decision_urgency_tier,
    );
    let queue_pressure_decision_dispatch_mode =
        dashboard_outbox_pressure_decision_dispatch_mode(queue_pressure_next_action_kind);
    let queue_pressure_decision_manual_ack_required =
        dashboard_outbox_pressure_decision_manual_ack_required(
            queue_pressure_next_action_kind,
            queue_pressure_automation_safe,
        );
    let queue_pressure_decision_execution_guard =
        dashboard_outbox_pressure_decision_execution_guard(
            queue_pressure_next_action_kind,
            queue_pressure_automation_safe,
        );
    let queue_pressure_decision_followup_required =
        dashboard_outbox_pressure_decision_followup_required(queue_pressure_next_action_kind);
    let compaction_recommended = stale_ratio >= 0.4 || oldest_age_seconds > 86_400;
    let compaction_reason = if rows.is_empty() {
        "none"
    } else if stale_ratio >= 0.4 {
        "stale_ratio>=0.40"
    } else if oldest_age_seconds > 86_400 {
        "oldest_age_seconds>86400"
    } else {
        "none"
    };
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.outbox.state".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_outbox_state",
            "depth": rows.len(),
            "ready_count": ready_count,
            "ready_ratio": ready_ratio,
            "cooldown_blocked_count": cooldown_blocked_count,
            "blocked_ratio": blocked_ratio,
            "health_tier": health_tier,
            "health_reason": health_reason,
            "oldest_age_seconds": oldest_age_seconds,
            "age_p95_seconds": age_p95_seconds,
            "oldest_item_id": oldest_item_id,
            "max_attempts_observed": max_attempts_observed,
            "next_retry_after_epoch_s": next_retry_after_epoch_s,
            "next_retry_after_seconds": next_retry_after_seconds,
            "next_retry_item_id": next_retry_item_id,
            "retry_due_within_60s_count": retry_due_within_60s_count,
            "retry_due_within_300s_count": retry_due_within_300s_count,
            "retry_due_within_900s_count": retry_due_within_900s_count,
            "fresh_count": fresh_count,
            "fresh_ratio": fresh_ratio,
            "aging_count": aging_count,
            "aging_ratio": aging_ratio,
            "stale_count": stale_count,
            "stale_ratio": stale_ratio,
            "stale_oldest_age_seconds": stale_oldest_age_seconds,
            "health_score": health_score,
            "queue_action_hint": queue_action_hint,
            "retry_pressure_tier": retry_pressure_tier,
            "queue_pressure_score": queue_pressure_score,
            "queue_pressure_tier": queue_pressure_tier,
            "queue_pressure_action_hint": queue_pressure_action_hint,
            "queue_pressure_priority": queue_pressure_priority,
            "queue_pressure_recommended_lane": queue_pressure_recommended_lane,
            "queue_pressure_escalation_required": queue_pressure_escalation_required,
            "queue_pressure_escalation_reason": queue_pressure_escalation_reason,
            "queue_pressure_runbook_id": queue_pressure_runbook_id,
            "queue_pressure_escalation_owner": queue_pressure_escalation_owner,
            "queue_pressure_sla_minutes": queue_pressure_sla_minutes,
            "queue_pressure_escalation_lane": queue_pressure_escalation_lane,
            "queue_pressure_deadline_epoch_s": queue_pressure_deadline_epoch_s,
            "queue_pressure_deadline_remaining_seconds": queue_pressure_deadline_remaining_seconds,
            "queue_pressure_breach": queue_pressure_breach,
            "queue_pressure_breach_reason": queue_pressure_breach_reason,
            "queue_pressure_breach_detected_at_epoch_s": queue_pressure_breach_detected_at_epoch_s,
            "queue_pressure_blocking_kind": queue_pressure_blocking_kind,
            "queue_pressure_auto_retry_allowed": queue_pressure_auto_retry_allowed,
            "queue_pressure_execution_policy": queue_pressure_execution_policy,
            "queue_pressure_manual_gate_required": queue_pressure_manual_gate_required,
            "queue_pressure_manual_gate_reason": queue_pressure_manual_gate_reason,
            "queue_pressure_requeue_strategy": queue_pressure_requeue_strategy,
            "queue_pressure_can_execute_without_human": queue_pressure_can_execute_without_human,
            "queue_pressure_execution_window": queue_pressure_execution_window,
            "queue_pressure_manual_gate_timeout_seconds": queue_pressure_manual_gate_timeout_seconds,
            "queue_pressure_next_action_after_seconds": queue_pressure_next_action_after_seconds,
            "queue_pressure_next_action_kind": queue_pressure_next_action_kind,
            "queue_pressure_retry_window_class": queue_pressure_retry_window_class,
            "queue_pressure_readiness_state": queue_pressure_readiness_state,
            "queue_pressure_readiness_reason": queue_pressure_readiness_reason,
            "queue_pressure_automation_safe": queue_pressure_automation_safe,
            "queue_pressure_decision_route_hint": queue_pressure_decision_route_hint,
            "queue_pressure_decision_urgency_tier": queue_pressure_decision_urgency_tier,
            "queue_pressure_decision_retry_budget_class": queue_pressure_decision_retry_budget_class,
            "queue_pressure_decision_lane_token": queue_pressure_decision_lane_token,
            "queue_pressure_decision_dispatch_mode": queue_pressure_decision_dispatch_mode,
            "queue_pressure_decision_manual_ack_required": queue_pressure_decision_manual_ack_required,
            "queue_pressure_decision_execution_guard": queue_pressure_decision_execution_guard,
            "queue_pressure_decision_followup_required": queue_pressure_decision_followup_required,
            "queue_pressure_decision_vector_version": "v1",
            "queue_pressure_decision_vector_key": queue_pressure_decision_vector_key,
            "queue_pressure_decision_vector": dashboard_outbox_pressure_decision_vector(
                queue_pressure_next_action_after_seconds,
                queue_pressure_next_action_kind,
                queue_pressure_retry_window_class,
                queue_pressure_readiness_state,
                queue_pressure_readiness_reason,
                queue_pressure_automation_safe
            ),
            "queue_pressure_contract_version": dashboard_outbox_pressure_contract_version(),
            "queue_pressure_contract_family": "dashboard_queue_pressure_contract_v1",
            "queue_pressure_snapshot_epoch_s": now_epoch,
            "queue_pressure_contract": dashboard_outbox_pressure_contract(
                now_epoch,
                queue_pressure_priority,
                queue_pressure_action_hint,
                queue_pressure_escalation_lane,
                queue_pressure_runbook_id,
                queue_pressure_escalation_owner,
                queue_pressure_blocking_kind,
                queue_pressure_auto_retry_allowed,
                queue_pressure_execution_policy,
                queue_pressure_manual_gate_required,
                queue_pressure_manual_gate_reason,
                queue_pressure_requeue_strategy,
                queue_pressure_can_execute_without_human,
                queue_pressure_execution_window,
                queue_pressure_manual_gate_timeout_seconds,
                queue_pressure_next_action_after_seconds,
                queue_pressure_next_action_kind,
                queue_pressure_retry_window_class,
                queue_pressure_readiness_state,
                queue_pressure_readiness_reason,
                queue_pressure_automation_safe,
                queue_pressure_decision_vector_key.as_str(),
                queue_pressure_deadline_epoch_s,
                queue_pressure_breach_reason
            ),
            "compaction_recommended": compaction_recommended,
            "compaction_reason": compaction_reason,
            "error_histogram": dashboard_troubleshooting_outbox_reason_histogram(&rows),
            "items": display_rows
        })),
    }
}
