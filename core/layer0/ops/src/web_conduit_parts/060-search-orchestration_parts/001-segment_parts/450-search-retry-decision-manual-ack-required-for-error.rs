fn search_retry_decision_manual_ack_required_for_error(
    error: &str,
    retry_after_seconds: i64,
) -> bool {
    !search_retry_automation_safe_for_error(error)
        || search_retry_next_action_kind_for_error(error, retry_after_seconds) == "manual_gate"
}
