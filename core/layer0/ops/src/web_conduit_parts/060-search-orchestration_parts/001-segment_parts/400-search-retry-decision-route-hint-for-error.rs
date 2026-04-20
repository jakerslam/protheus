fn search_retry_decision_route_hint_for_error(error: &str, retry_after_seconds: i64) -> &'static str {
    match search_retry_next_action_kind_for_error(error, retry_after_seconds) {
        "manual_gate" => "manual_review_lane",
        "deferred_retry" => "deferred_retry_lane",
        _ => "auto_execute_lane",
    }
}
