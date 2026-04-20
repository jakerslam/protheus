fn fetch_retry_decision_retry_budget_class_for_reason(
    reason: &str,
    retry_after_seconds: i64,
) -> &'static str {
    if !fetch_retry_automation_safe_for_reason(reason) {
        return "manual_only";
    }
    match fetch_retry_window_class_for_reason(reason, retry_after_seconds) {
        "immediate" => "single_attempt",
        "short" => "bounded_backoff_short",
        "medium" => "bounded_backoff_medium",
        _ => "bounded_backoff_long",
    }
}
