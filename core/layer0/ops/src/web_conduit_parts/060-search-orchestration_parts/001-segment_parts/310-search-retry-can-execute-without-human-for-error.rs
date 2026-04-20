fn search_retry_can_execute_without_human_for_error(error: &str) -> bool {
    matches!(
        search_retry_execution_policy_for_error(error),
        "auto_retry" | "deferred_auto_retry"
    )
}
