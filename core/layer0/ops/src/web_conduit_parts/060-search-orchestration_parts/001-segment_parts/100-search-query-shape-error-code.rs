fn search_query_shape_error_code(query: &str) -> &'static str {
    let lowered = search_strip_invisible_unicode(&clean_text(query, 1_200)).to_ascii_lowercase();
    let trimmed = lowered.trim();
    if trimmed.is_empty() {
        return "query_required";
    }
    if trimmed.contains("<html")
        || trimmed.contains("</html>")
        || trimmed.contains("<body")
        || trimmed.contains("sample input:")
        || trimmed.contains("sample output:")
    {
        return "query_payload_dump_detected";
    }
    if (trimmed.starts_with('{') && trimmed.contains(':'))
        || (trimmed.starts_with('[') && trimmed.contains('{'))
        || trimmed.starts_with("\"query\"")
    {
        return "query_payload_dump_detected";
    }
    if lowered.contains("```")
        || lowered.contains("diff --git")
        || lowered.contains("[patch v")
        || lowered.contains("input specification")
        || lowered.contains("sample output")
        || lowered.contains("you are an expert")
    {
        return "query_payload_dump_detected";
    }
    if search_query_fetch_url_candidate(trimmed).is_some() {
        return "query_prefers_fetch_url";
    }
    let line_count = lowered.lines().count();
    if line_count > 8 || lowered.len() > 520 {
        return "query_shape_invalid";
    }
    let mut total_terms = 0usize;
    let mut unique = Vec::<&str>::new();
    for token in lowered.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        let trimmed = token.trim();
        if trimmed.len() < 2 {
            continue;
        }
        total_terms += 1;
        if !unique.iter().any(|existing| *existing == trimmed) {
            unique.push(trimmed);
        }
    }
    if total_terms >= 7 && unique.len() <= 1 {
        return "query_shape_invalid";
    }
    let repetition_ratio = search_query_repetition_ratio(trimmed);
    if total_terms >= 8 && repetition_ratio >= 0.65 {
        return "query_shape_repetitive_loop";
    }
    let url_count = trimmed.match_indices("http://").count() + trimmed.match_indices("https://").count();
    if url_count > 1 {
        return "query_shape_invalid";
    }
    "none"
}
