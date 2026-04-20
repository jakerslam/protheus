
#[test]
fn search_query_shape_supports_bare_domain_candidates() {
    assert_eq!(
        search_query_shape_error_code("example.com/research/agents"),
        "query_prefers_fetch_url"
    );
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "query": "example.com/research/agents"
        }),
    );
    assert_eq!(
        out.pointer("/query_shape/fetch_url_candidate_kind")
            .and_then(Value::as_str),
        Some("bare_domain")
    );
    assert_eq!(
        out.pointer("/query_shape/fetch_url_candidate")
            .and_then(Value::as_str),
        Some("https://example.com/research/agents")
    );
}

#[test]
fn search_query_shape_supports_protocol_relative_candidates() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "query": "//example.com/protocol-relative"
        }),
    );
    assert_eq!(
        out.get("query_shape_error").and_then(Value::as_str),
        Some("query_prefers_fetch_url")
    );
    assert_eq!(
        out.get("query_shape_fetch_url_candidate")
            .and_then(Value::as_str),
        Some("https://example.com/protocol-relative")
    );
    assert_eq!(
        out.get("query_shape_fetch_url_candidate_kind")
            .and_then(Value::as_str),
        Some("protocol_relative")
    );
}

#[test]
fn fetch_normalizes_bare_domain_in_url_field() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "url": "example.com/path?a=1",
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("unknown_fetch_provider")
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/path?a=1")
    );
}

#[test]
fn fetch_request_uses_target_url_fallback_when_url_missing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "target_url": "example.com/from-target",
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("unknown_fetch_provider")
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/from-target")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("target_url")
    );
}

#[test]
fn search_query_shape_exposes_top_level_candidate_metadata() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "query": "example.com/insights"
        }),
    );
    assert_eq!(
        out.get("query_shape_fetch_url_candidate")
            .and_then(Value::as_str),
        Some("https://example.com/insights")
    );
    assert_eq!(
        out.get("query_shape_fetch_url_candidate_kind")
            .and_then(Value::as_str),
        Some("bare_domain")
    );
    assert_eq!(out.get("query_source").and_then(Value::as_str), Some("query"));
}

#[test]
fn fetch_request_uses_uri_field_and_reports_source() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "uri": "example.com/via-uri",
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/via-uri")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("uri")
    );
}

#[test]
fn search_query_allows_payload_query_source() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "payload": {
                "query": "example.com/payload-source"
            }
        }),
    );
    assert_eq!(out.get("query_source").and_then(Value::as_str), Some("payload.query"));
    assert_eq!(
        out.get("query_shape_fetch_url_candidate")
            .and_then(Value::as_str),
        Some("https://example.com/payload-source")
    );
}

#[test]
fn fetch_request_uses_payload_target_url_and_reports_source() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "payload": {
                "targetUrl": "example.com/via-payload-target-url"
            },
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/via-payload-target-url")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("payload.targetUrl")
    );
}

#[test]
fn search_query_source_supports_object_query_array_rows() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "queries": [
                {"query": "example.com/object-query-array"}
            ]
        }),
    );
    assert_eq!(
        out.get("query_source").and_then(Value::as_str),
        Some("queries[0].query")
    );
    assert_eq!(
        out.get("query_source_kind").and_then(Value::as_str),
        Some("array_field")
    );
    assert_eq!(
        out.get("query_shape_fetch_url_candidate")
            .and_then(Value::as_str),
        Some("https://example.com/object-query-array")
    );
}

#[test]
fn search_query_source_supports_payload_object_query_array_rows() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "payload": {
                "search_queries": [
                    {"q": "example.com/payload-object-array"}
                ]
            }
        }),
    );
    assert_eq!(
        out.get("query_source").and_then(Value::as_str),
        Some("payload.search_queries[0].q")
    );
    assert_eq!(
        out.get("query_source_kind").and_then(Value::as_str),
        Some("payload_array_field")
    );
    assert_eq!(
        out.get("query_shape_fetch_url_candidate")
            .and_then(Value::as_str),
        Some("https://example.com/payload-object-array")
    );
}
