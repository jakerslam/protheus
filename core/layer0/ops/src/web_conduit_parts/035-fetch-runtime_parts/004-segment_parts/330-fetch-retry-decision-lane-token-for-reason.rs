fn fetch_retry_decision_lane_token_for_reason(reason: &str, retry_after_seconds: i64) -> String {
    let route_hint = fetch_retry_decision_route_hint_for_reason(reason, retry_after_seconds);
    let urgency_tier = fetch_retry_decision_urgency_tier_for_reason(reason, retry_after_seconds);
    format!("{}::{}", route_hint, urgency_tier)
}
