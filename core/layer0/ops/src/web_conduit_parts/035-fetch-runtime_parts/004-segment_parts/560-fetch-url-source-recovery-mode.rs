fn fetch_url_source_recovery_mode(source: &str) -> &'static str {
    if source == "none" {
        "none"
    } else if source.starts_with("requested_url")
        || source.starts_with("url")
        || source.starts_with("target")
        || source.starts_with("link")
        || source.starts_with("href")
        || source.starts_with("uri")
    {
        "direct"
    } else {
        "derived"
    }
}
