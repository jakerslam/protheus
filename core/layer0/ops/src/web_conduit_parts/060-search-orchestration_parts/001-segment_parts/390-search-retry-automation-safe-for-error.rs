fn search_retry_automation_safe_for_error(error: &str) -> bool {
    search_retry_auto_retry_allowed_for_error(error)
        && search_retry_can_execute_without_human_for_error(error)
}
