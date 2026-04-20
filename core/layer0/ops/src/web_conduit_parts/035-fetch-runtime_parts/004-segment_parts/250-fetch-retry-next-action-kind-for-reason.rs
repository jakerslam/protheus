fn fetch_retry_next_action_kind_for_reason(reason: &str, retry_after_seconds: i64) -> &'static str {
    if !fetch_retry_can_execute_without_human_for_reason(reason) {
        "manual_gate"
    } else if fetch_retry_next_action_after_seconds_for_reason(reason, retry_after_seconds) > 0 {
        "deferred_retry"
    } else {
        "execute_now"
    }
}
