fn search_retry_decision_dispatch_mode_for_error(
    error: &str,
    retry_after_seconds: i64,
) -> &'static str {
    match search_retry_next_action_kind_for_error(error, retry_after_seconds) {
        "manual_gate" => "manual_review",
        "deferred_retry" => "scheduled_retry",
        _ => "immediate_execute",
    }
}
