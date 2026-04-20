fn search_retry_decision_execution_guard_for_error(
    error: &str,
    retry_after_seconds: i64,
) -> &'static str {
    if search_retry_decision_manual_ack_required_for_error(error, retry_after_seconds) {
        "manual_gate_guard"
    } else if search_retry_next_action_kind_for_error(error, retry_after_seconds) == "deferred_retry" {
        "retry_window_guard"
    } else {
        "none"
    }
}
