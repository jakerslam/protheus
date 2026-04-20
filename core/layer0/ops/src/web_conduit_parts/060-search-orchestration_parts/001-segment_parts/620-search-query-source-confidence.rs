fn search_query_source_confidence(source_kind: &str) -> &'static str {
    match source_kind {
        "none" => "none",
        "array_field" | "payload_array_field" | "request_array_field" => "medium",
        _ => "high",
    }
}
