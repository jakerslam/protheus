fn search_query_source_kind(source: &str) -> &'static str {
    if source == "none" {
        "none"
    } else if source.starts_with("payload.request.") && source.contains('[') {
        "request_array_field"
    } else if source.starts_with("request.") && source.contains('[') {
        "request_array_field"
    } else if source.starts_with("payload.request.") || source.starts_with("request.") {
        "request_field"
    } else if source.starts_with("payload.") && source.contains('[') {
        "payload_array_field"
    } else if source.contains('[') {
        "array_field"
    } else if source.starts_with("payload.") {
        "payload_field"
    } else {
        "direct_field"
    }
}
