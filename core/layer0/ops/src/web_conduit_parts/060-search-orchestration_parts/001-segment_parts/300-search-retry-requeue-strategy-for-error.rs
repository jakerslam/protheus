fn search_retry_requeue_strategy_for_error(error: &str) -> &'static str {
    match search_retry_execution_policy_for_error(error) {
        "auto_retry" => "immediate",
        "deferred_auto_retry" => "deferred",
        _ => "manual",
    }
}
