fn search_retry_decision_vector_key_for_error(error: &str, retry_after_seconds: i64) -> String {
    let next_action_after_seconds =
        search_retry_next_action_after_seconds_for_error(error, retry_after_seconds).max(0);
    let next_action_kind = search_retry_next_action_kind_for_error(error, retry_after_seconds);
    let retry_window_class = search_retry_window_class_for_error(error, retry_after_seconds);
    let readiness_state = search_retry_readiness_state_for_error(error, retry_after_seconds);
    let readiness_reason = search_retry_readiness_reason_for_error(error, retry_after_seconds);
    let automation_safe = if search_retry_automation_safe_for_error(error) {
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
