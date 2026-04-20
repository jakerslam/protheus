fn search_retry_readiness_state_for_error(error: &str, retry_after_seconds: i64) -> &'static str {
    if !search_retry_can_execute_without_human_for_error(error) {
        "manual_gate_pending"
    } else if search_retry_next_action_after_seconds_for_error(error, retry_after_seconds) > 0 {
        "deferred_retry_pending"
    } else {
        "ready_now"
    }
}
