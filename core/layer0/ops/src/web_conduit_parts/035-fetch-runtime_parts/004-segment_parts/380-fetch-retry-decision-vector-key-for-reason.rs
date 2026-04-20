fn fetch_retry_decision_vector_key_for_reason(reason: &str, retry_after_seconds: i64) -> String {
    let next_action_after_seconds =
        fetch_retry_next_action_after_seconds_for_reason(reason, retry_after_seconds).max(0);
    let next_action_kind = fetch_retry_next_action_kind_for_reason(reason, retry_after_seconds);
    let retry_window_class = fetch_retry_window_class_for_reason(reason, retry_after_seconds);
    let readiness_state = fetch_retry_readiness_state_for_reason(reason, retry_after_seconds);
    let readiness_reason = fetch_retry_readiness_reason_for_reason(reason, retry_after_seconds);
    let automation_safe = if fetch_retry_automation_safe_for_reason(reason) {
        "1"
    } else {
        "0"
    };
    format!(
        "{}|{}|{}|{}|{}|{}",
        next_action_kind,
        retry_window_class,
        readiness_state,
        readiness_reason,
        automation_safe,
        next_action_after_seconds
    )
}
