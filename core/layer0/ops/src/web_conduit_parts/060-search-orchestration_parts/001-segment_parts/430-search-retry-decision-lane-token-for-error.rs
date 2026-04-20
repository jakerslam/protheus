fn search_retry_decision_lane_token_for_error(error: &str, retry_after_seconds: i64) -> String {
    let route_hint = search_retry_decision_route_hint_for_error(error, retry_after_seconds);
    let urgency_tier = search_retry_decision_urgency_tier_for_error(error, retry_after_seconds);
    format!("{}::{}", route_hint, urgency_tier)
}
