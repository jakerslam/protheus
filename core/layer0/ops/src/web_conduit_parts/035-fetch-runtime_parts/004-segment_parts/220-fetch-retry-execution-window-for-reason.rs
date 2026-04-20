fn fetch_retry_execution_window_for_reason(reason: &str, retry_after_seconds: i64) -> &'static str {
    match fetch_retry_requeue_strategy_for_reason(reason) {
        "immediate" => "now",
        "deferred" => {
            if retry_after_seconds.max(0) > 0 {
                "after_retry_after"
            } else {
                "deferred"
            }
        }
        _ => "after_manual_gate",
    }
}
