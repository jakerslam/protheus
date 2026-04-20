fn search_extract_inline_http_url(raw: &str) -> Option<String> {
    for token in clean_text(raw, 2_200).split_whitespace() {
        let normalized = normalize_search_query_url_candidate(token);
        if (normalized.starts_with("http://") || normalized.starts_with("https://"))
            && !normalized.chars().any(|ch| ch.is_whitespace())
        {
            return Some(normalized);
        }
    }
    None
}
