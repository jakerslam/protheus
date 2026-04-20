fn search_query_source_lineage(source: &str, source_kind: &str, source_confidence: &str) -> Value {
    let normalized_source = clean_text(source, 180);
    let source_lane = if normalized_source.starts_with("payload.request.") {
        "payload_request"
    } else if normalized_source.starts_with("request.") {
        "request"
    } else if normalized_source.starts_with("payload.") {
        "payload"
    } else if normalized_source == "none" {
        "none"
    } else {
        "direct"
    };
    let path_depth = if normalized_source.is_empty() || normalized_source == "none" {
        0usize
    } else {
        normalized_source.split('.').count()
    };
    json!({
        "source": normalized_source,
        "kind": source_kind,
        "confidence": source_confidence,
        "lane": source_lane,
        "is_request_wrapped": source_lane == "request" || source_lane == "payload_request",
        "is_payload_wrapped": source_lane == "payload" || source_lane == "payload_request",
        "is_array_source": source.contains('['),
        "path_depth": path_depth
    })
}
