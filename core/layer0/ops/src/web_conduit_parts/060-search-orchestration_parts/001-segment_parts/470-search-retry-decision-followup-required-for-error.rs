fn search_retry_decision_followup_required_for_error(
    error: &str,
    retry_after_seconds: i64,
) -> bool {
    search_retry_next_action_kind_for_error(error, retry_after_seconds) != "execute_now"
}
