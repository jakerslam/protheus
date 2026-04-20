fn chat_ui_web_result_matches_query(query: &str, output: &str) -> bool {
    let query_terms = chat_ui_query_alignment_terms(query, 16);
    if query_terms.len() < 2 {
        return true;
    }
    let lowered = clean(output, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let matched = query_terms
        .iter()
        .filter(|term| lowered.contains(term.as_str()))
        .count();
    let required_hits = 2.min(query_terms.len());
    if matched >= required_hits {
        return true;
    }
    let ratio = (matched as f64) / (query_terms.len() as f64);
    let ratio_floor = if query_terms.len() >= 6 { 0.40 } else { 0.34 };
    ratio >= ratio_floor
}
