fn search_retry_lane_for_error(error: &str) -> &'static str {
    if error == "query_prefers_fetch_url" {
        "web_fetch"
    } else {
        "web_search"
    }
}
