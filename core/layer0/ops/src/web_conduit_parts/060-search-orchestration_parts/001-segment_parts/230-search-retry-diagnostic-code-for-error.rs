fn search_retry_diagnostic_code_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url" {
        "search_retry_query_prefers_fetch_url"
    } else if error == "non_search_meta_query" {
        "search_retry_non_search_meta_query"
    } else if error == "query_required" {
        "search_retry_query_required"
    } else if error == "conflicting_time_filters" {
        "search_retry_conflicting_time_filters"
    } else if error == "unknown_search_provider" {
        "search_retry_unknown_search_provider"
    } else if error == "unsupported_search_filter" {
        "search_retry_unsupported_search_filter"
    } else if error == "web_search_duplicate_attempt_suppressed" {
        "search_retry_duplicate_attempt_suppressed"
    } else if error.starts_with("web_search_tool_surface_") {
        "search_retry_tool_surface"
    } else {
        "search_retry_request_contract_adjustment_required"
    }
}
