fn search_query_fetch_url_candidate(query: &str) -> Option<String> {
    search_query_fetch_url_candidate_with_kind(query).map(|row| row.0)
}
