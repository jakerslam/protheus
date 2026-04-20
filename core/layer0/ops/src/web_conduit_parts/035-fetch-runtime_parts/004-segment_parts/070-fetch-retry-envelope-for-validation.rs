fn fetch_retry_envelope_for_validation(error: &str, validation_route: &str) -> Value {
    let strategy = if error == "non_fetch_meta_query" || validation_route == "meta_query_blocked" {
        "answer_directly_without_web_fetch"
    } else if error == "unknown_fetch_provider" {
        "use_supported_provider_or_auto"
    } else if error == "fetch_url_required" {
        "provide_valid_http_or_https_url"
    } else if error == "fetch_url_invalid_scheme" {
        "provide_http_or_https_scheme"
    } else if error == "fetch_url_payload_dump_detected" || error == "fetch_url_shape_invalid" {
        "rewrite_fetch_input_as_url"
    } else {
        "adjust_request_and_retry"
    };
    fetch_retry_envelope_runtime(
        strategy,
        if error.is_empty() { validation_route } else { error },
        "web_fetch",
        0,
    )
}
