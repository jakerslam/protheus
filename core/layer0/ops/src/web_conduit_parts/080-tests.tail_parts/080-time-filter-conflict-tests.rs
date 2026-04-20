
#[test]
fn search_conflicting_time_filters_response_includes_query_shape_contract() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "query": "example.com/conflicting-time-filters",
            "freshness": "week",
            "date_after": "2026-04-10"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("conflicting_time_filters")
    );
    assert_eq!(
        out.get("query_shape_route_hint").and_then(Value::as_str),
        Some("web_fetch")
    );
    assert_eq!(
        out.get("query_shape_fetch_url_candidate")
            .and_then(Value::as_str),
        Some("https://example.com/conflicting-time-filters")
    );
    assert_eq!(
        out.pointer("/query_shape/route_hint").and_then(Value::as_str),
        Some("web_fetch")
    );
    assert_eq!(
        out.pointer("/suggested_next_action/action")
            .and_then(Value::as_str),
        Some("web_conduit_fetch")
    );
    assert_eq!(
        out.get("tool_execution_attempted").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        out.pointer("/tool_execution_gate/reason")
            .and_then(Value::as_str),
        Some("conflicting_time_filters")
    );
    assert_eq!(
        out.get("meta_query_blocked").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        out.get("cache_status").and_then(Value::as_str),
        Some("skipped_validation")
    );
    assert_eq!(
        out.pointer("/retry/strategy").and_then(Value::as_str),
        Some("remove_conflicting_time_filters")
    );
    assert_eq!(
        out.pointer("/retry/reason").and_then(Value::as_str),
        Some("conflicting_time_filters")
    );
    assert_eq!(
        out.pointer("/retry/contract_version").and_then(Value::as_str),
        Some("v1")
    );
    assert_eq!(
        out.pointer("/retry/contract_family").and_then(Value::as_str),
        Some("web_retry_contract_v1")
    );
    assert_eq!(
        out.pointer("/retry/recovery_mode").and_then(Value::as_str),
        Some("adjust_filters")
    );
    assert_eq!(
        out.pointer("/retry/priority").and_then(Value::as_str),
        Some("medium")
    );
    assert_eq!(
        out.pointer("/retry/operator_action_hint")
            .and_then(Value::as_str),
        Some("remove_freshness_or_date_range_conflict")
    );
    assert_eq!(
        out.pointer("/retry/operator_owner").and_then(Value::as_str),
        Some("user")
    );
    assert_eq!(
        out.pointer("/retry/diagnostic_code").and_then(Value::as_str),
        Some("search_retry_conflicting_time_filters")
    );
    assert_eq!(
        out.pointer("/retry/blocking_kind").and_then(Value::as_str),
        Some("input_adjustment_required")
    );
    assert_eq!(
        out.pointer("/retry/auto_retry_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        out.pointer("/retry/retryable").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        out.pointer("/retry/idempotent").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        out.pointer("/retry/category").and_then(Value::as_str),
        Some("validation")
    );
    assert_eq!(
        out.pointer("/retry/lane").and_then(Value::as_str),
        Some("web_search")
    );
}

#[test]
fn fetch_request_uses_request_body_data_text_fallback_and_source() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "request": {
                "body": {
                    "data": {
                        "text": "fetch https://example.com/from-request-body-data-text"
                    }
                }
            },
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/from-request-body-data-text")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("request.body.data.text")
    );
}

#[test]
fn fetch_request_uses_payload_request_body_data_question_fallback_and_source() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "payload": {
                "request": {
                    "body": {
                        "data": {
                            "question": "can you open https://example.com/from-payload-request-body-data-question"
                        }
                    }
                }
            },
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/from-payload-request-body-data-question")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("payload.request.body.data.question")
    );
}
