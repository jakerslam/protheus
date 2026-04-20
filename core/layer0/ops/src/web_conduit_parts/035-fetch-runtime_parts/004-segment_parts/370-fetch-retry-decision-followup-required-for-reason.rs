fn fetch_retry_decision_followup_required_for_reason(
    reason: &str,
    retry_after_seconds: i64,
) -> bool {
    fetch_retry_next_action_kind_for_reason(reason, retry_after_seconds) != "execute_now"
}
