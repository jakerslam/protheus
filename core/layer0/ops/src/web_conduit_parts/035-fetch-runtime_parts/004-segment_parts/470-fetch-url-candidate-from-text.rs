fn fetch_url_candidate_from_text(raw: &str) -> Option<String> {
    let trimmed = clean_text(raw, 2_200).trim().to_string();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = normalize_fetch_requested_url_input(&trimmed);
    if (normalized.starts_with("http://") || normalized.starts_with("https://"))
        && !normalized.contains(char::is_whitespace)
    {
        return Some(normalized);
    }
    if normalized.starts_with("www.") && !normalized.contains(char::is_whitespace) {
        return Some(format!("https://{}", normalized));
    }
    None
}
