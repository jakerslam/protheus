fn fetch_retry_decision_dispatch_mode_for_reason(
    reason: &str,
    retry_after_seconds: i64,
) -> &'static str {
    match fetch_retry_next_action_kind_for_reason(reason, retry_after_seconds) {
        "manual_gate" => "manual_review",
        "deferred_retry" => "scheduled_retry",
        _ => "immediate_execute",
    }
}
