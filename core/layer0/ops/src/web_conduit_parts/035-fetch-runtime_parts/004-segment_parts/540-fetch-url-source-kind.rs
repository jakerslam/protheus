fn fetch_url_source_kind(source: &str) -> &'static str {
    if source == "none" {
        "none"
    } else if source.starts_with("payload.request.") && (source.ends_with("[0]") || source.contains("[0].")) {
        "request_array_field"
    } else if source.starts_with("request.") && (source.ends_with("[0]") || source.contains("[0].")) {
        "request_array_field"
    } else if source == "request.query"
        || source == "request.q"
        || source == "request.search_query"
        || source == "request.searchQuery"
        || source == "payload.request.query"
        || source == "payload.request.q"
        || source == "payload.request.search_query"
        || source == "payload.request.searchQuery"
    {
        "request_query_fallback"
    } else if source.starts_with("payload.") && (source.ends_with("[0]") || source.contains("[0].")) {
        "payload_array_field"
    } else if source.ends_with("[0]") || source.contains("[0].") {
        "array_field"
    } else if source == "query" || source == "q" {
        "query_fallback"
    } else if source == "message"
        || source == "text"
        || source == "input"
        || source == "prompt"
        || source == "question"
    {
        "query_fallback"
    } else if source == "request.message"
        || source == "request.text"
        || source == "request.input"
        || source == "request.prompt"
        || source == "request.question"
        || source == "payload.request.message"
        || source == "payload.request.text"
        || source == "payload.request.input"
        || source == "payload.request.prompt"
        || source == "payload.request.question"
    {
        "request_query_fallback"
    } else if source == "payload.query"
        || source == "payload.q"
        || source == "payload.search_query"
        || source == "payload.searchQuery"
        || source == "payload.message"
        || source == "payload.text"
        || source == "payload.input"
        || source == "payload.prompt"
        || source == "payload.question"
    {
        "payload_query_fallback"
    } else if source.starts_with("request.") || source.starts_with("payload.request.") {
        "request_field"
    } else if source.starts_with("payload.") {
        "payload_field"
    } else {
        "direct_field"
    }
}
