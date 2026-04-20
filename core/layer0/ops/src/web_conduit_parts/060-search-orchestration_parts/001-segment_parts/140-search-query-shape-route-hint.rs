fn search_query_shape_route_hint(reason: &str) -> &'static str {
    if reason == "query_prefers_fetch_url" {
        "web_fetch"
    } else {
        "web_search"
    }
}
