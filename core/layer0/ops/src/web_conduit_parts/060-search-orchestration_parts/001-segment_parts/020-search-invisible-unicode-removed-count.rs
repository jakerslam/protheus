fn search_invisible_unicode_removed_count(raw: &str) -> usize {
    let stripped = search_strip_invisible_unicode(raw);
    raw.chars().count().saturating_sub(stripped.chars().count())
}
