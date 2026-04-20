
#[test]
fn fetch_request_uses_payload_query_url_fallback_and_kind() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "payload": {
                "query": "Please fetch https://example.com/from-payload-query"
            },
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/from-payload-query")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("payload.query")
    );
    assert_eq!(
        out.get("requested_url_source_kind").and_then(Value::as_str),
        Some("payload_query_fallback")
    );
}

#[test]
fn fetch_request_uses_request_url_and_kind() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "request": {
                "url": "example.com/from-request-object"
            },
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/from-request-object")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("request.url")
    );
    assert_eq!(
        out.get("requested_url_source_kind").and_then(Value::as_str),
        Some("request_field")
    );
}

#[test]
fn search_query_source_supports_payload_request_query_alias() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "payload": {
                "request": {
                    "query": "example.com/payload-request-query-source"
                }
            }
        }),
    );
    assert_eq!(
        out.get("query_source").and_then(Value::as_str),
        Some("payload.request.query")
    );
    assert_eq!(
        out.get("query_source_kind").and_then(Value::as_str),
        Some("request_field")
    );
    assert_eq!(
        out.get("query_shape_fetch_url_candidate")
            .and_then(Value::as_str),
        Some("https://example.com/payload-request-query-source")
    );
}

#[test]
fn fetch_request_uses_request_query_fallback_and_kind() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "request": {
                "query": "Please fetch https://example.com/from-request-query"
            },
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/from-request-query")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("request.query")
    );
    assert_eq!(
        out.get("requested_url_source_kind").and_then(Value::as_str),
        Some("request_query_fallback")
    );
}

#[test]
fn fetch_request_uses_payload_request_query_fallback_and_kind() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "payload": {
                "request": {
                    "q": "https://example.com/from-payload-request-q"
                }
            },
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/from-payload-request-q")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("payload.request.q")
    );
    assert_eq!(
        out.get("requested_url_source_kind").and_then(Value::as_str),
        Some("request_query_fallback")
    );
}

#[test]
fn search_early_validation_exposes_query_source_confidence() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "query": "that was just a test"
        }),
    );
    assert_eq!(
        out.get("query_source_confidence").and_then(Value::as_str),
        Some("high")
    );
}

#[test]
fn search_query_source_supports_payload_request_array_row() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "payload": {
                "request": {
                    "queries": [
                        {"q": "example.com/payload-request-array-source"}
                    ]
                }
            }
        }),
    );
    assert_eq!(
        out.get("query_source").and_then(Value::as_str),
        Some("payload.request.queries[0].q")
    );
    assert_eq!(
        out.get("query_source_kind").and_then(Value::as_str),
        Some("request_array_field")
    );
    assert_eq!(
        out.get("query_source_confidence").and_then(Value::as_str),
        Some("medium")
    );
}

#[test]
fn fetch_request_uses_payload_urls_object_array_and_kind() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "payload": {
                "urls": [
                    {"url": "https://example.com/from-payload-urls-object-array"}
                ]
            },
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/from-payload-urls-object-array")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("payload.urls[0].url")
    );
    assert_eq!(
        out.get("requested_url_source_kind").and_then(Value::as_str),
        Some("payload_array_field")
    );
    assert_eq!(
        out.get("requested_url_source_confidence")
            .and_then(Value::as_str),
        Some("medium")
    );
}

#[test]
fn fetch_request_query_fallback_exposes_high_source_confidence() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "request": {
                "query": "Please fetch https://example.com/request-fallback-confidence"
            },
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url_source_kind").and_then(Value::as_str),
        Some("request_query_fallback")
    );
    assert_eq!(
        out.get("requested_url_source_confidence")
            .and_then(Value::as_str),
        Some("high")
    );
}
