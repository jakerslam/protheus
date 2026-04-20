fn search_query_repetition_ratio(raw: &str) -> f64 {
    let lowered = clean_text(raw, 1_200).to_ascii_lowercase();
    let mut tokens = Vec::<String>::new();
    for token in lowered.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        let trimmed = token.trim();
        if trimmed.len() >= 2 {
            tokens.push(trimmed.to_string());
        }
    }
    if tokens.len() < 6 {
        return 0.0;
    }
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for token in tokens {
        let current = counts.get(&token).copied().unwrap_or(0);
        counts.insert(token, current.saturating_add(1));
    }
    let total = counts.values().sum::<usize>();
    if total == 0 {
        return 0.0;
    }
    let max_count = counts.values().copied().max().unwrap_or(0);
    (max_count as f64) / (total as f64)
}
