fn search_retry_operator_action_hint_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url" {
        "invoke_web_fetch_with_requested_url"
    } else if error == "non_search_meta_query" {
        "answer_without_web_search_or_set_force_web_search"
    } else if error == "query_required" {
        "provide_non_empty_query"
    } else if error == "conflicting_time_filters" {
        "remove_freshness_or_date_range_conflict"
    } else if error == "unknown_search_provider" {
        "set_provider_auto_or_supported_provider"
    } else if error == "unsupported_search_filter" {
        "remove_or_replace_unsupported_filter"
    } else if error == "web_search_duplicate_attempt_suppressed" {
        "adjust_query_or_wait_for_retry_window"
    } else if error.starts_with("web_search_tool_surface_") {
        "restore_web_tool_surface_and_retry"
    } else {
        "adjust_search_request_and_retry"
    }
}
