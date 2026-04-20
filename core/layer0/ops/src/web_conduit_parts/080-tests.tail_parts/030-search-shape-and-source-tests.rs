
#[test]
fn search_shape_block_response_carries_override_source_and_stats() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let long_query = "top agent frameworks ".repeat(40);
    let out = api_search(tmp.path(), &json!({"query": long_query}));
    assert_eq!(
        out.get("query_shape_blocked").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        out.get("query_shape_override_source")
            .and_then(Value::as_str),
        Some("none")
    );
    assert_eq!(
        out.get("query_shape_override_used").and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        out.pointer("/query_shape_stats/char_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
    );
    assert_eq!(
        out.get("query_shape_category").and_then(Value::as_str),
        Some("invalid_shape")
    );
    assert!(
        out.get("query_shape_recommended_action")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("concise")
    );
}

#[test]
fn search_query_shape_override_can_be_enabled_by_policy() {
    let policy = json!({
        "web_conduit": {
            "search_policy": {
                "allow_query_shape_override": true
            }
        }
    });
    assert!(search_query_shape_override(&policy, &json!({})));
}

#[test]
fn search_query_shape_override_source_detects_request() {
    assert_eq!(
        search_query_shape_override_source(&json!({}), &json!({"allow_query_shape_override": true})),
        "request"
    );
}

#[test]
fn fetch_url_shape_error_reports_invalid_scheme() {
    assert_eq!(
        fetch_url_shape_error_code("ftp://example.com/archive"),
        "fetch_url_invalid_scheme"
    );
}

#[test]
fn fetch_url_shape_error_flags_json_blob_payload() {
    assert_eq!(
        fetch_url_shape_error_code("{\"requested_url\":\"https://example.com\"}"),
        "fetch_url_payload_dump_detected"
    );
}

#[test]
fn fetch_url_shape_error_flags_whitespace_url() {
    assert_eq!(
        fetch_url_shape_error_code("https://example.com/some path"),
        "fetch_url_shape_invalid"
    );
}

#[test]
fn fetch_url_shape_contract_reports_invisible_unicode_stripping() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({"requested_url":"ftp://exa\u{200B}mple.com"}),
    );
    assert_eq!(
        out.pointer("/fetch_url_shape/invisible_unicode_stripped")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        out.pointer("/fetch_url_shape/invisible_unicode_removed_count")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            >= 1,
        true
    );
}

#[test]
fn fetch_url_shape_override_can_be_enabled_by_policy() {
    let policy = json!({
        "web_conduit": {
            "fetch_policy": {
                "allow_fetch_url_shape_override": true
            }
        }
    });
    assert!(fetch_url_shape_override(&policy, &json!({})));
}

#[test]
fn fetch_url_shape_override_source_detects_request() {
    assert_eq!(
        fetch_url_shape_override_source(&json!({}), &json!({"force_web_fetch": true})),
        "request"
    );
}

#[test]
fn fetch_shape_block_response_carries_override_source_and_stats() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "url": "ftp://example.com/archive"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("fetch_url_invalid_scheme")
    );
    assert_eq!(
        out.get("fetch_url_shape_override_source")
            .and_then(Value::as_str),
        Some("none")
    );
    assert_eq!(
        out.get("fetch_url_shape_override_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        out.pointer("/fetch_url_shape_stats/char_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
    );
    assert_eq!(
        out.get("fetch_url_shape_category").and_then(Value::as_str),
        Some("invalid_scheme")
    );
    assert_eq!(
        out.pointer("/retry/strategy").and_then(Value::as_str),
        Some("provide_http_or_https_scheme")
    );
    assert_eq!(
        out.pointer("/retry/reason").and_then(Value::as_str),
        Some("fetch_url_invalid_scheme")
    );
    assert_eq!(
        out.pointer("/retry/contract_version").and_then(Value::as_str),
        Some("v1")
    );
    assert_eq!(
        out.pointer("/retry/lane").and_then(Value::as_str),
        Some("web_fetch")
    );
    assert!(
        out.get("fetch_url_shape_recommended_action")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("http")
    );
}

#[test]
fn fetch_normalizes_wrapped_url_before_provider_validation() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "url": "<https://example.com/path?q=1>",
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("unknown_fetch_provider")
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/path?q=1")
    );
    assert_eq!(
        out.get("requested_url_input").and_then(Value::as_str),
        Some("<https://example.com/path?q=1>")
    );
}

#[test]
fn fetch_normalization_strips_trailing_punctuation_before_provider_validation() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "url": "\"https://example.com/path?q=1),\"",
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("unknown_fetch_provider")
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/path?q=1")
    );
    assert_eq!(
        out.get("requested_url_input").and_then(Value::as_str),
        Some("\"https://example.com/path?q=1),\"")
    );
    assert_eq!(
        out.pointer("/fetch_url_shape/route_hint").and_then(Value::as_str),
        Some("web_fetch")
    );
    assert_eq!(
        out.pointer("/fetch_url_shape/normalization_changed")
            .and_then(Value::as_bool),
        Some(true)
    );
}
