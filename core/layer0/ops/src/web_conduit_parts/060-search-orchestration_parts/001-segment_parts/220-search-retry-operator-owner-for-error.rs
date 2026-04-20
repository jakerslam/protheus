fn search_retry_operator_owner_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url"
        || error == "non_search_meta_query"
        || error == "query_required"
        || error == "conflicting_time_filters"
        || error == "unsupported_search_filter"
    {
        "user"
    } else if error == "unknown_search_provider" {
        "operator"
    } else if error == "web_search_duplicate_attempt_suppressed"
        || error.starts_with("web_search_tool_surface_")
    {
        "system_operator"
    } else {
        "operator"
    }
}
