fn dashboard_outbox_health_tier(depth: usize, ready_ratio: f64, blocked_ratio: f64) -> &'static str {
    if depth == 0 {
        "empty"
    } else if ready_ratio >= 0.7 && blocked_ratio <= 0.3 {
        "healthy"
    } else if ready_ratio >= 0.4 {
        "degraded"
    } else {
        "blocked"
    }
}

fn dashboard_outbox_health_reason(depth: usize, ready_ratio: f64, blocked_ratio: f64) -> &'static str {
    if depth == 0 {
        "outbox_empty"
    } else if ready_ratio >= 0.7 && blocked_ratio <= 0.3 {
        "ready_ratio>=0.70_and_blocked_ratio<=0.30"
    } else if ready_ratio >= 0.4 {
        "ready_ratio>=0.40_with_some_cooldown_pressure"
    } else {
        "ready_ratio<0.40_or_cooldown_dominant"
    }
}

fn dashboard_outbox_queue_action_hint(health_tier: &str) -> &'static str {
    match health_tier {
        "empty" => "no_action",
        "healthy" => "drain_normally",
        "degraded" => "increase_flush_frequency_and_monitor_auth",
        _ => "resolve_auth_or_transport_blockers_then_force_flush",
    }
}

fn dashboard_outbox_health_score(
    depth: usize,
    ready_ratio: f64,
    blocked_ratio: f64,
    stale_ratio: f64,
) -> f64 {
    if depth == 0 {
        return 100.0;
    }
    let score = (ready_ratio * 0.65 + (1.0 - blocked_ratio) * 0.25 + (1.0 - stale_ratio) * 0.10)
        * 100.0;
    (score.clamp(0.0, 100.0) * 100.0).round() / 100.0
}

fn dashboard_outbox_retry_pressure_tier(
    depth: usize,
    due_within_60s: usize,
    due_within_300s: usize,
    due_within_900s: usize,
) -> &'static str {
    if depth == 0 {
        "none"
    } else if due_within_60s >= depth.saturating_div(2).max(1)
        || due_within_300s >= depth.saturating_mul(3).saturating_div(4).max(1)
    {
        "high"
    } else if due_within_300s > 0 || due_within_900s >= depth.saturating_div(2).max(1) {
        "medium"
    } else {
        "low"
    }
}

fn dashboard_outbox_pressure_score(
    depth: usize,
    blocked_ratio: f64,
    stale_ratio: f64,
    retry_pressure_tier: &str,
) -> f64 {
    if depth == 0 {
        return 0.0;
    }
    let retry_weight = match retry_pressure_tier {
        "high" => 1.0,
        "medium" => 0.6,
        "low" => 0.25,
        _ => 0.0,
    };
    let score = (blocked_ratio * 0.45 + stale_ratio * 0.35 + retry_weight * 0.20) * 100.0;
    (score.clamp(0.0, 100.0) * 100.0).round() / 100.0
}

fn dashboard_outbox_pressure_tier(pressure_score: f64) -> &'static str {
    if pressure_score >= 70.0 {
        "high"
    } else if pressure_score >= 35.0 {
        "medium"
    } else if pressure_score > 0.0 {
        "low"
    } else {
        "none"
    }
}

fn dashboard_outbox_pressure_action_hint(pressure_tier: &str) -> &'static str {
    match pressure_tier {
        "high" => "run_outbox_flush_and_triage_deadletter_immediately",
        "medium" => "increase_eval_drain_frequency_and_monitor_retry_backlog",
        "low" => "continue_normal_drain_with_periodic_health_checks",
        _ => "no_action",
    }
}

fn dashboard_outbox_pressure_priority(pressure_tier: &str) -> &'static str {
    match pressure_tier {
        "high" => "p0",
        "medium" => "p1",
        "low" => "p2",
        _ => "none",
    }
}

fn dashboard_outbox_pressure_recommended_lane(pressure_tier: &str) -> &'static str {
    match pressure_tier {
        "high" => "dashboard.troubleshooting.outbox.flush",
        "medium" => "dashboard.troubleshooting.eval.drain",
        "low" => "dashboard.troubleshooting.summary",
        _ => "none",
    }
}

fn dashboard_outbox_pressure_runbook_id(pressure_tier: &str) -> &'static str {
    match pressure_tier {
        "high" => "runbook.troubleshooting.queue_pressure.high",
        "medium" => "runbook.troubleshooting.queue_pressure.medium",
        "low" => "runbook.troubleshooting.queue_pressure.low",
        _ => "none",
    }
}

fn dashboard_outbox_pressure_escalation_owner(pressure_tier: &str) -> &'static str {
    match pressure_tier {
        "high" => "ops_oncall",
        "medium" => "runtime_owner",
        "low" => "none",
        _ => "none",
    }
}

fn dashboard_outbox_pressure_sla_minutes(pressure_tier: &str) -> i64 {
    match pressure_tier {
        "high" => 5,
        "medium" => 15,
        "low" => 60,
        _ => 0,
    }
}

fn dashboard_outbox_pressure_escalation_lane(pressure_tier: &str) -> &'static str {
    match pressure_tier {
        "high" => "dashboard.troubleshooting.outbox.flush",
        "medium" => "dashboard.troubleshooting.eval.drain",
        _ => "none",
    }
}

fn dashboard_outbox_pressure_deadline_breach(
    now_epoch: i64,
    pressure_tier: &str,
    oldest_age_seconds: i64,
    blocked_ratio: f64,
    sla_minutes: i64,
    oldest_age_breach_reason: &'static str,
    blocked_ratio_breach_reason: &'static str,
) -> (i64, bool, &'static str) {
    let deadline_epoch_s = if sla_minutes > 0 {
        now_epoch.saturating_add(sla_minutes.saturating_mul(60))
    } else {
        0
    };
    if pressure_tier != "high" {
        return (deadline_epoch_s, false, "none");
    }
    if oldest_age_seconds > 1800 {
        return (deadline_epoch_s, true, oldest_age_breach_reason);
    }
    if blocked_ratio >= 0.7 {
        return (deadline_epoch_s, true, blocked_ratio_breach_reason);
    }
    (deadline_epoch_s, false, "none")
}

fn dashboard_outbox_pressure_deadline_remaining_seconds(
    now_epoch: i64,
    deadline_epoch_s: i64,
) -> i64 {
    if deadline_epoch_s <= 0 || deadline_epoch_s <= now_epoch {
        0
    } else {
        deadline_epoch_s.saturating_sub(now_epoch)
    }
}

fn dashboard_outbox_pressure_breach_detected_at_epoch_s(now_epoch: i64, breach: bool) -> i64 {
    if breach {
        now_epoch
    } else {
        0
    }
}

fn dashboard_outbox_pressure_contract_version() -> &'static str {
    "v1"
}

fn dashboard_outbox_pressure_blocking_kind(tier: &str, breach: bool) -> &'static str {
    if breach || tier == "critical" {
        "manual_intervention_required"
    } else if tier == "high" {
        "operator_review_required"
    } else {
        "none"
    }
}

fn dashboard_outbox_pressure_auto_retry_allowed(blocking_kind: &str) -> bool {
    blocking_kind == "none"
}

fn dashboard_outbox_pressure_execution_policy(
    blocking_kind: &str,
    auto_retry_allowed: bool,
) -> &'static str {
    if auto_retry_allowed {
        "auto_retry"
    } else if blocking_kind == "none" {
        "deferred_auto_retry"
    } else {
        "manual_gate_required"
    }
}

fn dashboard_outbox_pressure_manual_gate_required(blocking_kind: &str) -> bool {
    blocking_kind != "none"
}

fn dashboard_outbox_pressure_manual_gate_reason(blocking_kind: &str) -> &'static str {
    match blocking_kind {
        "manual_intervention_required" => "manual_intervention_required",
        "operator_review_required" => "operator_review_required",
        _ => "none",
    }
}

fn dashboard_outbox_pressure_requeue_strategy(execution_policy: &str) -> &'static str {
    match execution_policy {
        "auto_retry" => "immediate",
        "deferred_auto_retry" => "deferred",
        _ => "manual",
    }
}

fn dashboard_outbox_pressure_can_execute_without_human(execution_policy: &str) -> bool {
    matches!(execution_policy, "auto_retry" | "deferred_auto_retry")
}

fn dashboard_outbox_pressure_execution_window(requeue_strategy: &str) -> &'static str {
    match requeue_strategy {
        "immediate" => "now",
        "deferred" => "deferred",
        _ => "after_manual_gate",
    }
}

fn dashboard_outbox_pressure_manual_gate_timeout_seconds(manual_gate_reason: &str) -> i64 {
    match manual_gate_reason {
        "manual_intervention_required" => 1200,
        "operator_review_required" => 1800,
        _ => 0,
    }
}

fn dashboard_outbox_pressure_next_action_after_seconds(
    execution_window: &str,
    manual_gate_timeout_seconds: i64,
) -> i64 {
    match execution_window {
        "now" => 0,
        "deferred" => 60,
        _ => manual_gate_timeout_seconds.max(0),
    }
}

fn dashboard_outbox_pressure_readiness_state(
    can_execute_without_human: bool,
    next_action_after_seconds: i64,
) -> &'static str {
    if !can_execute_without_human {
        "manual_gate_pending"
    } else if next_action_after_seconds > 0 {
        "deferred_retry_pending"
    } else {
        "ready_now"
    }
}

fn dashboard_outbox_pressure_next_action_kind(
    can_execute_without_human: bool,
    next_action_after_seconds: i64,
) -> &'static str {
    if !can_execute_without_human {
        "manual_gate"
    } else if next_action_after_seconds > 0 {
        "deferred_retry"
    } else {
        "execute_now"
    }
}

fn dashboard_outbox_pressure_readiness_reason(
    next_action_kind: &str,
    manual_gate_reason: &str,
    blocking_kind: &str,
) -> &'static str {
    match next_action_kind {
        "manual_gate" => match manual_gate_reason {
            "manual_intervention_required" => "manual_intervention_required",
            "operator_review_required" => "operator_review_required",
            _ => "manual_gate_required",
        },
        "deferred_retry" => {
            if blocking_kind == "cooldown_required" {
                "retry_after_pending"
            } else {
                "deferred_retry_pending"
            }
        }
        _ => "none",
    }
}

fn dashboard_outbox_pressure_retry_window_class(next_action_after_seconds: i64) -> &'static str {
    if next_action_after_seconds <= 0 {
        "immediate"
    } else if next_action_after_seconds <= 60 {
        "short"
    } else if next_action_after_seconds <= 900 {
        "medium"
    } else {
        "long"
    }
}

fn dashboard_outbox_pressure_automation_safe(
    auto_retry_allowed: bool,
    can_execute_without_human: bool,
) -> bool {
    auto_retry_allowed && can_execute_without_human
}

fn dashboard_outbox_pressure_decision_vector_key(
    next_action_after_seconds: i64,
    next_action_kind: &str,
    retry_window_class: &str,
    readiness_state: &str,
    readiness_reason: &str,
    automation_safe: bool,
) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        clean_text(next_action_kind, 80),
        clean_text(retry_window_class, 40),
        clean_text(readiness_state, 80),
        clean_text(readiness_reason, 80),
        if automation_safe { 1 } else { 0 },
        next_action_after_seconds.max(0)
    )
}

fn dashboard_outbox_pressure_decision_vector(
    next_action_after_seconds: i64,
    next_action_kind: &str,
    retry_window_class: &str,
    readiness_state: &str,
    readiness_reason: &str,
    automation_safe: bool,
) -> Value {
    let route_hint = dashboard_outbox_pressure_decision_route_hint(next_action_kind);
    let urgency_tier =
        dashboard_outbox_pressure_decision_urgency_tier(retry_window_class, automation_safe);
    let retry_budget_class =
        dashboard_outbox_pressure_decision_retry_budget_class(retry_window_class, automation_safe);
    let lane_token = dashboard_outbox_pressure_decision_lane_token(route_hint, urgency_tier);
    let dispatch_mode = dashboard_outbox_pressure_decision_dispatch_mode(next_action_kind);
    let manual_ack_required =
        dashboard_outbox_pressure_decision_manual_ack_required(next_action_kind, automation_safe);
    let execution_guard =
        dashboard_outbox_pressure_decision_execution_guard(next_action_kind, automation_safe);
    let followup_required = dashboard_outbox_pressure_decision_followup_required(next_action_kind);
    let decision_vector_key = dashboard_outbox_pressure_decision_vector_key(
        next_action_after_seconds,
        next_action_kind,
        retry_window_class,
        readiness_state,
        readiness_reason,
        automation_safe,
    );
    json!({
        "next_action_after_seconds": next_action_after_seconds.max(0),
        "next_action_kind": clean_text(next_action_kind, 80),
        "retry_window_class": clean_text(retry_window_class, 40),
        "readiness_state": clean_text(readiness_state, 80),
        "readiness_reason": clean_text(readiness_reason, 80),
        "automation_safe": automation_safe,
        "route_hint": clean_text(route_hint, 80),
        "urgency_tier": clean_text(urgency_tier, 40),
        "retry_budget_class": clean_text(retry_budget_class, 80),
        "lane_token": lane_token,
        "dispatch_mode": clean_text(dispatch_mode, 80),
        "manual_ack_required": manual_ack_required,
        "execution_guard": clean_text(execution_guard, 80),
        "followup_required": followup_required,
        "decision_vector_version": "v1",
        "decision_vector_key": decision_vector_key
    })
}
