fn search_retry_readiness_reason_for_error(error: &str, retry_after_seconds: i64) -> &'static str {
    match search_retry_next_action_kind_for_error(error, retry_after_seconds) {
        "manual_gate" => search_retry_manual_gate_reason_for_error(error),
        "deferred_retry" => {
            if retry_after_seconds.max(0) > 0 {
                "retry_after_pending"
            } else {
                "deferred_retry_pending"
            }
        }
        _ => "none",
    }
}
