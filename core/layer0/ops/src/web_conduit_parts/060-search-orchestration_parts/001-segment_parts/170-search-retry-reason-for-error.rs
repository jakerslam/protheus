fn search_retry_reason_for_error(error: &str) -> &'static str {
    match error {
        "query_prefers_fetch_url" => "query_prefers_fetch_url",
        "non_search_meta_query" => "non_search_meta_query",
        "query_required" => "query_required",
        "conflicting_time_filters" => "conflicting_time_filters",
        "unknown_search_provider" => "unknown_search_provider",
        "unsupported_search_filter" => "unsupported_search_filter",
        "query_shape_repetitive_loop" => "query_shape_repetitive_loop",
        _ => "request_contract_adjustment_required",
    }
}
