fn fetch_retry_decision_urgency_tier_for_reason(reason: &str, retry_after_seconds: i64) -> &'static str {
    if !fetch_retry_automation_safe_for_reason(reason) {
        return "manual";
    }
    match fetch_retry_window_class_for_reason(reason, retry_after_seconds) {
        "immediate" => "high",
        "short" => "medium",
        "medium" => "low",
        _ => "deferred",
    }
}
