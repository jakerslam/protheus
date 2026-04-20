fn fetch_url_shape_category(reason: &str) -> &'static str {
    match reason {
        "fetch_url_required" => "missing_input",
        "fetch_url_payload_dump_detected" => "payload_dump",
        "fetch_url_invalid_scheme" => "invalid_scheme",
        "fetch_url_shape_invalid" => "invalid_shape",
        _ => "none",
    }
}
