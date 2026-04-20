fn fetch_retry_readiness_reason_for_reason(reason: &str, retry_after_seconds: i64) -> &'static str {
    match fetch_retry_next_action_kind_for_reason(reason, retry_after_seconds) {
        "manual_gate" => fetch_retry_manual_gate_reason_for_reason(reason),
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
