
#[test]
fn search_meta_query_early_response_includes_query_source_kind() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "query": "that was just a test"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("non_search_meta_query")
    );
    assert_eq!(
        out.get("query_source").and_then(Value::as_str),
        Some("query")
    );
    assert_eq!(
        out.get("query_source_kind").and_then(Value::as_str),
        Some("direct_field")
    );
    assert_eq!(
        out.get("query_shape_route_hint").and_then(Value::as_str),
        Some("web_search")
    );
    assert_eq!(
        out.pointer("/retry/strategy").and_then(Value::as_str),
        Some("answer_directly_without_web_search")
    );
    assert_eq!(
        out.pointer("/retry/reason").and_then(Value::as_str),
        Some("non_search_meta_query")
    );
    assert_eq!(
        out.pointer("/retry/contract_version").and_then(Value::as_str),
        Some("v1")
    );
    assert_eq!(
        out.pointer("/retry/lane").and_then(Value::as_str),
        Some("web_search")
    );
}

#[test]
fn search_query_source_supports_request_object_alias() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "request": {
                "query": "example.com/request-object-source"
            }
        }),
    );
    assert_eq!(
        out.get("query_source").and_then(Value::as_str),
        Some("request.query")
    );
    assert_eq!(
        out.get("query_source_kind").and_then(Value::as_str),
        Some("direct_field")
    );
    assert_eq!(
        out.get("query_shape_fetch_url_candidate")
            .and_then(Value::as_str),
        Some("https://example.com/request-object-source")
    );
}

#[test]
fn fetch_request_uses_payload_urls_array_and_kind() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "payload": {
                "urls": ["https://example.com/from-payload-urls-array"]
            },
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/from-payload-urls-array")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("payload.urls[0]")
    );
    assert_eq!(
        out.get("requested_url_source_kind").and_then(Value::as_str),
        Some("payload_array_field")
    );
}

#[test]
fn fetch_request_uses_payload_request_urls_array_and_kind() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "payload": {
                "request": {
                    "urls": ["https://example.com/from-payload-request-urls-array"]
                }
            },
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/from-payload-request-urls-array")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("payload.request.urls[0]")
    );
    assert_eq!(
        out.get("requested_url_source_kind").and_then(Value::as_str),
        Some("request_array_field")
    );
}

#[test]
fn fetch_request_uses_request_data_message_text_fallback_and_source() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "request": {
                "data": {
                    "message": "please fetch https://example.com/from-request-data-message"
                }
            },
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/from-request-data-message")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("request.data.message")
    );
    assert_eq!(
        out.get("requested_url_source_kind").and_then(Value::as_str),
        Some("request_field")
    );
}

#[test]
fn fetch_request_uses_payload_request_data_prompt_text_fallback_and_source() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "payload": {
                "request": {
                    "data": {
                        "prompt": "Open https://example.com/from-payload-request-data-prompt"
                    }
                }
            },
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/from-payload-request-data-prompt")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("payload.request.data.prompt")
    );
    assert_eq!(
        out.get("requested_url_source_kind").and_then(Value::as_str),
        Some("request_field")
    );
}
