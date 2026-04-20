fn search_retry_decision_urgency_tier_for_error(error: &str, retry_after_seconds: i64) -> &'static str {
    if !search_retry_automation_safe_for_error(error) {
        return "manual";
    }
    match search_retry_window_class_for_error(error, retry_after_seconds) {
        "immediate" => "high",
        "short" => "medium",
        "medium" => "low",
        _ => "deferred",
    }
}
