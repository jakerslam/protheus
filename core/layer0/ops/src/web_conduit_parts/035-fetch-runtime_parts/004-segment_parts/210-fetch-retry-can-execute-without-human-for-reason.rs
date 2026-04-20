fn fetch_retry_can_execute_without_human_for_reason(reason: &str) -> bool {
    matches!(
        fetch_retry_execution_policy_for_reason(reason),
        "auto_retry" | "deferred_auto_retry"
    )
}
