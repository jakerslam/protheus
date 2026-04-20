fn search_retry_strategy_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url" {
        "use_web_fetch_route"
    } else if error == "non_search_meta_query" {
        "answer_directly_without_web_search"
    } else if error == "query_required" {
        "provide_query_text"
    } else if error == "conflicting_time_filters" {
        "remove_conflicting_time_filters"
    } else if error == "unknown_search_provider" {
        "use_supported_provider_or_auto"
    } else if error == "unsupported_search_filter" {
        "remove_unsupported_filter"
    } else if error == "query_shape_repetitive_loop" {
        "rewrite_without_repetition"
    } else {
        "rewrite_query_shape"
    }
}
