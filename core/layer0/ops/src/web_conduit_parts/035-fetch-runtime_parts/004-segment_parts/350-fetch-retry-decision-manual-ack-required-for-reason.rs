fn fetch_retry_decision_manual_ack_required_for_reason(
    reason: &str,
    retry_after_seconds: i64,
) -> bool {
    !fetch_retry_automation_safe_for_reason(reason)
        || fetch_retry_next_action_kind_for_reason(reason, retry_after_seconds) == "manual_gate"
}
