fn fetch_retry_next_action_after_seconds_for_reason(reason: &str, retry_after_seconds: i64) -> i64 {
    match fetch_retry_execution_window_for_reason(reason, retry_after_seconds) {
        "now" => 0,
        "after_retry_after" => retry_after_seconds.max(0),
        "deferred" => 60,
        _ => fetch_retry_manual_gate_timeout_seconds_for_reason(reason),
    }
}
