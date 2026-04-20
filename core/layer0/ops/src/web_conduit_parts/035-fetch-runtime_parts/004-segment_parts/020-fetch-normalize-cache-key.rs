fn fetch_normalize_cache_key(raw: &str) -> String {
    fetch_strip_invisible_unicode(&clean_text(raw, 2_200))
        .trim()
        .to_ascii_lowercase()
}
