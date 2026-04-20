fn search_retry_next_action_kind_for_error(error: &str, retry_after_seconds: i64) -> &'static str {
    if !search_retry_can_execute_without_human_for_error(error) {
        "manual_gate"
    } else if search_retry_next_action_after_seconds_for_error(error, retry_after_seconds) > 0 {
        "deferred_retry"
    } else {
        "execute_now"
    }
}
