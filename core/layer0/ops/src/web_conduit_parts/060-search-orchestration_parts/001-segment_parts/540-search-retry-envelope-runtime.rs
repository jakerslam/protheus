fn search_retry_envelope_runtime(
    strategy: &str,
    reason: &str,
    lane: &str,
    retry_after_seconds: i64,
) -> Value {
    let decision_vector = search_retry_decision_vector_for_error(reason, retry_after_seconds);
    let next_action_after_seconds = decision_vector
        .get("next_action_after_seconds")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let next_action_kind = decision_vector
        .get("next_action_kind")
        .and_then(Value::as_str)
        .unwrap_or("execute_now");
    let retry_window_class = decision_vector
        .get("retry_window_class")
        .and_then(Value::as_str)
        .unwrap_or("immediate");
    let readiness_state = decision_vector
        .get("readiness_state")
        .and_then(Value::as_str)
        .unwrap_or("ready_now");
    let readiness_reason = decision_vector
        .get("readiness_reason")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let automation_safe = decision_vector
        .get("automation_safe")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let decision_vector_key = decision_vector
        .get("decision_vector_key")
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_string();
    let decision_route_hint = decision_vector
        .get("route_hint")
        .and_then(Value::as_str)
        .unwrap_or("auto_execute_lane");
    let decision_urgency_tier = decision_vector
        .get("urgency_tier")
        .and_then(Value::as_str)
        .unwrap_or("deferred");
    let decision_retry_budget_class = decision_vector
        .get("retry_budget_class")
        .and_then(Value::as_str)
        .unwrap_or("manual_only");
    let decision_lane_token = decision_vector
        .get("lane_token")
        .and_then(Value::as_str)
        .unwrap_or("auto_execute_lane::deferred")
        .to_string();
    let decision_dispatch_mode = decision_vector
        .get("dispatch_mode")
        .and_then(Value::as_str)
        .unwrap_or("immediate_execute");
    let decision_manual_ack_required = decision_vector
        .get("manual_ack_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let decision_execution_guard = decision_vector
        .get("execution_guard")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let decision_followup_required = decision_vector
        .get("followup_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let decision_vector_version = decision_vector
        .get("decision_vector_version")
        .and_then(Value::as_str)
        .unwrap_or("v1");
    json!({
        "recommended": true,
        "retryable": true,
        "idempotent": true,
        "contract_family": "web_retry_contract_v1",
        "strategy": strategy,
        "lane": lane,
        "reason": reason,
        "category": search_retry_category_for_error(reason),
        "recovery_mode": search_retry_recovery_mode_for_error(reason),
        "priority": search_retry_priority_for_error(reason),
        "operator_action_hint": search_retry_operator_action_hint_for_error(reason),
        "operator_owner": search_retry_operator_owner_for_error(reason),
        "diagnostic_code": search_retry_diagnostic_code_for_error(reason),
        "blocking_kind": search_retry_blocking_kind_for_error(reason),
        "auto_retry_allowed": search_retry_auto_retry_allowed_for_error(reason),
        "escalation_lane": search_retry_escalation_lane_for_error(reason),
        "requires_manual_confirmation": search_retry_requires_manual_confirmation_for_error(reason),
        "execution_policy": search_retry_execution_policy_for_error(reason),
        "manual_gate_reason": search_retry_manual_gate_reason_for_error(reason),
        "requeue_strategy": search_retry_requeue_strategy_for_error(reason),
        "can_execute_without_human": search_retry_can_execute_without_human_for_error(reason),
        "execution_window": search_retry_execution_window_for_error(reason, retry_after_seconds),
        "manual_gate_timeout_seconds": search_retry_manual_gate_timeout_seconds_for_error(reason),
        "next_action_after_seconds": next_action_after_seconds,
        "next_action_kind": next_action_kind,
        "retry_window_class": retry_window_class,
        "readiness_state": readiness_state,
        "readiness_reason": readiness_reason,
        "automation_safe": automation_safe,
        "decision_route_hint": decision_route_hint,
        "decision_urgency_tier": decision_urgency_tier,
        "decision_retry_budget_class": decision_retry_budget_class,
        "decision_lane_token": decision_lane_token,
        "decision_dispatch_mode": decision_dispatch_mode,
        "decision_manual_ack_required": decision_manual_ack_required,
        "decision_execution_guard": decision_execution_guard,
        "decision_followup_required": decision_followup_required,
        "decision_vector_version": decision_vector_version,
        "decision_vector_key": decision_vector_key,
        "decision_vector": decision_vector,
        "contract_version": "v1",
        "retry_after_seconds": retry_after_seconds.max(0)
    })
}
