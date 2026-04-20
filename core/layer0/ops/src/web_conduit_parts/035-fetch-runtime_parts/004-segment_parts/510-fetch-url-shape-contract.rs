fn fetch_url_shape_contract(
    requested_url_input: &str,
    normalized_requested_url: &str,
    reason: &str,
    override_used: bool,
    override_source: &str,
) -> Value {
    let input_cleaned = clean_text(requested_url_input, 2_400);
    let input_trimmed = input_cleaned.trim().to_string();
    let stripped = fetch_strip_invisible_unicode(&input_cleaned);
    let invisible_unicode_removed_count =
        input_cleaned.chars().count().saturating_sub(stripped.chars().count()) as i64;
    let invisible_unicode_stripped = invisible_unicode_removed_count > 0;
    let normalized_changed = input_trimmed != normalized_requested_url;
    json!({
        "blocked": reason != "none" && !override_used,
        "error": reason,
        "category": fetch_url_shape_category(reason),
        "recommended_action": fetch_url_shape_recommended_action(reason),
        "route_hint": fetch_url_shape_route_hint(reason),
        "normalized_requested_url": normalized_requested_url,
        "normalization_changed": normalized_changed,
        "invisible_unicode_stripped": invisible_unicode_stripped,
        "invisible_unicode_removed_count": invisible_unicode_removed_count,
        "override_used": override_used,
        "override_source": override_source,
        "stats": fetch_url_shape_stats(&fetch_strip_invisible_unicode(normalized_requested_url)),
        "input_char_count": input_cleaned.len()
    })
}
