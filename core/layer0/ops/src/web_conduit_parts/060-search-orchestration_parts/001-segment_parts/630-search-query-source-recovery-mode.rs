fn search_query_source_recovery_mode(source: &str) -> &'static str {
    if source == "none" {
        "none"
    } else if source.starts_with("query")
        || source.starts_with("q")
        || source.starts_with("search_query")
        || source.starts_with("searchQuery")
        || source.starts_with("prompt")
    {
        "direct"
    } else {
        "derived"
    }
}
