fn search_query_looks_like_bare_domain(raw: &str) -> bool {
    let candidate = clean_text(raw, 2_100).trim().to_ascii_lowercase();
    if candidate.is_empty() || candidate.contains(char::is_whitespace) {
        return false;
    }
    if candidate.starts_with("http://") || candidate.starts_with("https://") {
        return false;
    }
    let host = candidate
        .split('/')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("")
        .split('#')
        .next()
        .unwrap_or("");
    if host.is_empty() || !host.contains('.') {
        return false;
    }
    let labels = host.split('.').collect::<Vec<_>>();
    if labels.len() < 2 {
        return false;
    }
    let tld = labels.last().copied().unwrap_or("");
    if tld.len() < 2 || !tld.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return false;
    }
    labels.iter().all(|label| {
        !label.is_empty()
            && label.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    })
}
