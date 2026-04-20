fn fetch_retry_decision_route_hint_for_reason(reason: &str, retry_after_seconds: i64) -> &'static str {
    match fetch_retry_next_action_kind_for_reason(reason, retry_after_seconds) {
        "manual_gate" => "manual_review_lane",
        "deferred_retry" => "deferred_retry_lane",
        _ => "auto_execute_lane",
    }
}
