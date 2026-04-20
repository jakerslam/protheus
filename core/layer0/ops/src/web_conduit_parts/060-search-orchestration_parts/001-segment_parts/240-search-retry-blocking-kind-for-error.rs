fn search_retry_blocking_kind_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url"
        || error == "query_required"
        || error == "conflicting_time_filters"
        || error == "unsupported_search_filter"
    {
        "input_adjustment_required"
    } else if error == "non_search_meta_query" {
        "direct_answer_required"
    } else if error == "unknown_search_provider" {
        "provider_configuration_required"
    } else if error == "web_search_duplicate_attempt_suppressed" {
        "cooldown_required"
    } else if error.starts_with("web_search_tool_surface_") {
        "tool_surface_restore_required"
    } else {
        "none"
    }
}
