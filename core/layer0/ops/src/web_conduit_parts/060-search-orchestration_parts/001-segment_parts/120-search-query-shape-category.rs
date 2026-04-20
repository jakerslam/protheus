fn search_query_shape_category(reason: &str) -> &'static str {
    match reason {
        "query_required" => "missing_input",
        "query_payload_dump_detected" => "payload_dump",
        "query_prefers_fetch_url" => "prefers_fetch",
        "query_shape_repetitive_loop" => "repetition_loop",
        "query_shape_invalid" => "invalid_shape",
        _ => "none",
    }
}
