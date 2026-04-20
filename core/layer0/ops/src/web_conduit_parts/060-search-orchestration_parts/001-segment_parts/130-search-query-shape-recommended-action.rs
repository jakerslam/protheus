fn search_query_shape_recommended_action(reason: &str) -> &'static str {
    match reason {
        "query_required" => "provide a concise search query describing what to find",
        "query_payload_dump_detected" => {
            "replace pasted logs/pages with a short web intent and 2-8 focused keywords"
        }
        "query_prefers_fetch_url" => {
            "input is a direct URL; use web fetch action for page retrieval instead of search"
        }
        "query_shape_repetitive_loop" => {
            "query appears repetitive; replace repeated terms with 2-8 specific keywords"
        }
        "query_shape_invalid" => {
            "rewrite query as one concise sentence (recommended <= 300 chars)"
        }
        _ => "none",
    }
}
