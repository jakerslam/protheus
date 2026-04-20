fn fetch_url_shape_route_hint(reason: &str) -> &'static str {
    if reason == "none" {
        "web_fetch"
    } else if reason == "fetch_url_payload_dump_detected" {
        "chat_or_web_search"
    } else {
        "web_fetch"
    }
}
