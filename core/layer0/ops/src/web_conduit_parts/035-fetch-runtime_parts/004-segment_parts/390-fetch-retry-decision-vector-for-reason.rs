fn fetch_retry_decision_vector_for_reason(reason: &str, retry_after_seconds: i64) -> Value {
    let next_action_after_seconds =
        fetch_retry_next_action_after_seconds_for_reason(reason, retry_after_seconds).max(0);
    let next_action_kind = fetch_retry_next_action_kind_for_reason(reason, retry_after_seconds);
    let retry_window_class = fetch_retry_window_class_for_reason(reason, retry_after_seconds);
    let readiness_state = fetch_retry_readiness_state_for_reason(reason, retry_after_seconds);
    let readiness_reason = fetch_retry_readiness_reason_for_reason(reason, retry_after_seconds);
    let automation_safe = fetch_retry_automation_safe_for_reason(reason);
    let route_hint = fetch_retry_decision_route_hint_for_reason(reason, retry_after_seconds);
    let urgency_tier = fetch_retry_decision_urgency_tier_for_reason(reason, retry_after_seconds);
    let retry_budget_class =
        fetch_retry_decision_retry_budget_class_for_reason(reason, retry_after_seconds);
    let lane_token = fetch_retry_decision_lane_token_for_reason(reason, retry_after_seconds);
    let dispatch_mode = fetch_retry_decision_dispatch_mode_for_reason(reason, retry_after_seconds);
    let manual_ack_required =
        fetch_retry_decision_manual_ack_required_for_reason(reason, retry_after_seconds);
    let execution_guard =
        fetch_retry_decision_execution_guard_for_reason(reason, retry_after_seconds);
    let followup_required =
        fetch_retry_decision_followup_required_for_reason(reason, retry_after_seconds);
    let decision_vector_key =
        fetch_retry_decision_vector_key_for_reason(reason, retry_after_seconds);
    json!({
        "next_action_after_seconds": next_action_after_seconds,
        "next_action_kind": next_action_kind,
        "retry_window_class": retry_window_class,
        "readiness_state": readiness_state,
        "readiness_reason": readiness_reason,
        "automation_safe": automation_safe,
        "route_hint": route_hint,
        "urgency_tier": urgency_tier,
        "retry_budget_class": retry_budget_class,
        "lane_token": lane_token,
        "dispatch_mode": dispatch_mode,
        "manual_ack_required": manual_ack_required,
        "execution_guard": execution_guard,
        "followup_required": followup_required,
        "decision_vector_version": "v1",
        "decision_vector_key": decision_vector_key
    })
}
