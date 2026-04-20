fn fetch_retry_requeue_strategy_for_reason(reason: &str) -> &'static str {
    match fetch_retry_execution_policy_for_reason(reason) {
        "auto_retry" => "immediate",
        "deferred_auto_retry" => "deferred",
        _ => "manual",
    }
}
