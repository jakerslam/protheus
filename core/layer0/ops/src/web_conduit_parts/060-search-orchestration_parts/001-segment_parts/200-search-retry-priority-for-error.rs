fn search_retry_priority_for_error(error: &str) -> &'static str {
    if error == "query_required" || error.starts_with("web_search_tool_surface_") {
        "high"
    } else if error == "non_search_meta_query" {
        "low"
    } else {
        "medium"
    }
}
