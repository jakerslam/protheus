fn fetch_url_shape_recommended_action(reason: &str) -> &'static str {
    match reason {
        "fetch_url_required" => "provide an http(s) URL to fetch",
        "fetch_url_payload_dump_detected" => {
            "replace pasted text with a single http(s) URL; keep diagnostics in normal chat"
        }
        "fetch_url_invalid_scheme" => "use a URL starting with http:// or https://",
        "fetch_url_shape_invalid" => {
            "submit one concise URL only (no spaces/newlines/payload wrappers)"
        }
        _ => "none",
    }
}
