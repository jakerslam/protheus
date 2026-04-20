fn fetch_url_shape_stats(raw_requested_url: &str) -> Value {
    let lowered =
        fetch_strip_invisible_unicode(&clean_text(raw_requested_url, 2_400)).to_ascii_lowercase();
    let mut total_terms = 0usize;
    let mut unique = Vec::<String>::new();
    for token in lowered.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }
        total_terms += 1;
        if !unique.iter().any(|existing| existing == trimmed) {
            unique.push(trimmed.to_string());
        }
    }
    let host_candidate = lowered
        .split("://")
        .nth(1)
        .unwrap_or(&lowered)
        .split('/')
        .next()
        .unwrap_or("")
        .to_string();
    json!({
        "line_count": lowered.lines().count(),
        "char_count": lowered.len(),
        "total_terms": total_terms,
        "unique_terms": unique.len(),
        "host_candidate": host_candidate,
        "path_present": lowered.contains('/'),
    })
}
