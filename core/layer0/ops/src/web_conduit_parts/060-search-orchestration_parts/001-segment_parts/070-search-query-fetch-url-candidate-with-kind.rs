fn search_query_fetch_url_candidate_with_kind(query: &str) -> Option<(String, &'static str)> {
    let cleaned = clean_text(query, 2_200);
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return None;
    }
    let direct_candidate = normalize_search_query_url_candidate(trimmed);
    if direct_candidate.starts_with("https://") && trimmed.trim_start().starts_with("//") {
        return Some((direct_candidate, "protocol_relative"));
    }
    if (direct_candidate.starts_with("http://") || direct_candidate.starts_with("https://"))
        && !direct_candidate.chars().any(|ch| ch.is_whitespace())
    {
        return Some((direct_candidate, "direct_url"));
    }
    if trimmed.contains(char::is_whitespace) {
        if let Some(candidate) = search_extract_inline_http_url(trimmed) {
            return Some((candidate, "inline_url"));
        }
    }
    if direct_candidate.starts_with("www.") && !direct_candidate.chars().any(|ch| ch.is_whitespace()) {
        return Some((format!("https://{}", direct_candidate), "www_domain"));
    }
    if search_query_looks_like_bare_domain(&direct_candidate) {
        return Some((format!("https://{}", direct_candidate), "bare_domain"));
    }
    if trimmed.starts_with('[') && trimmed.contains("](") && trimmed.ends_with(')') {
        if let (Some(open_idx), Some(close_idx)) = (trimmed.rfind('('), trimmed.rfind(')')) {
            if close_idx > open_idx + 1 {
                let candidate =
                    normalize_search_query_url_candidate(trimmed[open_idx + 1..close_idx].trim());
                if (candidate.starts_with("http://") || candidate.starts_with("https://"))
                    && !candidate.chars().any(|ch| ch.is_whitespace())
                {
                    return Some((candidate, "markdown_link"));
                }
            }
        }
    }
    None
}
