fn search_query_shape_contract(
    query: &str,
    reason: &str,
    override_used: bool,
    override_source: &str,
) -> Value {
    let cleaned = clean_text(query, 2_200);
    let stripped = search_strip_invisible_unicode(&cleaned);
    let invisible_unicode_removed_count =
        cleaned.chars().count().saturating_sub(stripped.chars().count()) as i64;
    let invisible_unicode_stripped = invisible_unicode_removed_count > 0;
    let fetch_url_candidate = search_query_fetch_url_candidate(&stripped).unwrap_or_default();
    let fetch_url_candidate_kind = search_query_fetch_url_candidate_kind(&stripped);
    json!({
        "blocked": reason != "none" && !override_used,
        "error": reason,
        "category": search_query_shape_category(reason),
        "recommended_action": search_query_shape_recommended_action(reason),
        "route_hint": search_query_shape_route_hint(reason),
        "suggested_next_action": search_query_shape_suggested_next_action(query, reason),
        "fetch_url_candidate": fetch_url_candidate,
        "fetch_url_candidate_kind": fetch_url_candidate_kind,
        "invisible_unicode_stripped": invisible_unicode_stripped,
        "invisible_unicode_removed_count": invisible_unicode_removed_count,
        "override_used": override_used,
        "override_source": override_source,
        "stats": search_query_shape_stats(&stripped)
    })
}
