fn search_retry_envelope_for_error(error: &str) -> Value {
    search_retry_envelope_runtime(
        search_retry_strategy_for_error(error),
        search_retry_reason_for_error(error),
        search_retry_lane_for_error(error),
        0,
    )
}
