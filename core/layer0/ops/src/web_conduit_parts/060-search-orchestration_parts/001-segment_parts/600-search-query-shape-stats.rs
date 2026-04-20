fn search_query_shape_stats(query: &str) -> Value {
    let cleaned = clean_text(query, 1_200).to_ascii_lowercase();
    let mut total_terms = 0usize;
    let mut unique = Vec::<String>::new();
    let mut term_counts = std::collections::BTreeMap::<String, usize>::new();
    for token in cleaned.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }
        total_terms += 1;
        if !unique.iter().any(|existing| existing == trimmed) {
            unique.push(trimmed.to_string());
        }
        let current = term_counts.get(trimmed).copied().unwrap_or(0);
        term_counts.insert(trimmed.to_string(), current.saturating_add(1));
    }
    let (dominant_term, dominant_term_count) = term_counts
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(term, count)| (term.clone(), *count))
        .unwrap_or_else(|| (String::new(), 0usize));
    let repetition_ratio = if total_terms == 0 {
        0.0
    } else {
        (dominant_term_count as f64) / (total_terms as f64)
    };
    let fetch_url_candidate = search_query_fetch_url_candidate(query).unwrap_or_default();
    let fetch_url_candidate_kind = search_query_fetch_url_candidate_kind(query);
    json!({
        "line_count": cleaned.lines().count(),
        "char_count": cleaned.len(),
        "total_terms": total_terms,
        "unique_terms": unique.len(),
        "repetition_ratio": repetition_ratio,
        "dominant_term": dominant_term,
        "dominant_term_count": dominant_term_count,
        "url_candidate_detected": !fetch_url_candidate.is_empty(),
        "url_candidate": fetch_url_candidate,
        "url_candidate_kind": fetch_url_candidate_kind,
    })
}
