fn search_query_fetch_url_candidate_kind(query: &str) -> &'static str {
    search_query_fetch_url_candidate_with_kind(query)
        .map(|row| row.1)
        .unwrap_or("none")
}
