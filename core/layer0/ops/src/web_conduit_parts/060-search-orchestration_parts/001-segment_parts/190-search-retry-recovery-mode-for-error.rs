fn search_retry_recovery_mode_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url" {
        "reroute_fetch"
    } else if error == "non_search_meta_query" {
        "answer_directly"
    } else if error == "conflicting_time_filters" || error == "unsupported_search_filter" {
        "adjust_filters"
    } else if error == "unknown_search_provider" {
        "switch_provider"
    } else if error == "web_search_duplicate_attempt_suppressed" {
        "adjust_query_or_provider"
    } else if error.starts_with("web_search_tool_surface_") {
        "restore_tool_surface"
    } else {
        "adjust_request"
    }
}
