fn search_retry_decision_retry_budget_class_for_error(
    error: &str,
    retry_after_seconds: i64,
) -> &'static str {
    if !search_retry_automation_safe_for_error(error) {
        return "manual_only";
    }
    match search_retry_window_class_for_error(error, retry_after_seconds) {
        "immediate" => "single_attempt",
        "short" => "bounded_backoff_short",
        "medium" => "bounded_backoff_medium",
        _ => "bounded_backoff_long",
    }
}
