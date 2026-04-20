fn search_retry_category_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url" || error == "query_required" {
        "input_contract"
    } else if error == "non_search_meta_query" {
        "intent_contract"
    } else if error == "conflicting_time_filters" || error == "unsupported_search_filter" {
        "filter_contract"
    } else if error == "unknown_search_provider" {
        "provider_contract"
    } else if error == "web_search_duplicate_attempt_suppressed" {
        "replay_guard"
    } else if error.starts_with("web_search_tool_surface_") {
        "tool_surface"
    } else {
        "request_contract"
    }
}
