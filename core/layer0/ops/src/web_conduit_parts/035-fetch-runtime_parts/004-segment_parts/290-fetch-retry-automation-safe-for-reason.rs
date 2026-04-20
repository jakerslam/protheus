fn fetch_retry_automation_safe_for_reason(reason: &str) -> bool {
    fetch_retry_auto_retry_allowed_for_reason(reason)
        && fetch_retry_can_execute_without_human_for_reason(reason)
}
