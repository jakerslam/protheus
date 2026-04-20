
#[test]
fn search_early_validation_blocks_conversational_test_prompt() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = api_search(tmp.path(), &json!({"query": "that was just a test"}));
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("non_search_meta_query")
    );
    assert_eq!(
        out.get("meta_query_blocked").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        out.get("cache_skip_reason").and_then(Value::as_str),
        Some("meta_query_blocked")
    );
    assert_eq!(
        out.get("tool_execution_attempted").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn search_early_validation_empty_query_marks_skipped_execution() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = api_search(tmp.path(), &json!({"query": ""}));
    assert_eq!(out.get("error").and_then(Value::as_str), Some("query_required"));
    assert_eq!(
        out.get("cache_status").and_then(Value::as_str),
        Some("skipped_validation")
    );
    assert_eq!(
        out.get("cache_store_allowed").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        out.get("cache_skip_reason").and_then(Value::as_str),
        Some("query_required")
    );
    assert_eq!(
        out.get("cache_write_attempted").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        out.get("tool_execution_attempted").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        out.get("tool_execution_skipped_reason")
            .and_then(Value::as_str),
        Some("query_required")
    );
    assert_eq!(
        out.get("providers_attempted")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(0)
    );
    assert_eq!(
        out.pointer("/tool_execution_gate/reason")
            .and_then(Value::as_str),
        Some("query_required")
    );
    assert_eq!(
        out.pointer("/provider_resolution/reason")
            .and_then(Value::as_str),
        Some("query_required")
    );
    assert_eq!(
        out.pointer("/provider_resolution/tool_surface_health/status")
            .and_then(Value::as_str),
        Some("not_evaluated")
    );
    assert_eq!(
        out.pointer("/provider_health/status")
            .and_then(Value::as_str),
        Some("not_evaluated")
    );
    assert_eq!(
        out.get("tool_surface_status").and_then(Value::as_str),
        Some("not_evaluated")
    );
    assert_eq!(
        out.get("tool_surface_ready").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn search_query_shape_error_flags_payload_dump_tokens() {
    assert_eq!(
        search_query_shape_error_code(
            "```text\n[PATCH v2] diff --git a/x b/x\ninput specification\nsample output\n```"
        ),
        "query_payload_dump_detected"
    );
}

#[test]
fn search_query_shape_error_flags_json_blob_payload() {
    assert_eq!(
        search_query_shape_error_code("{\"query\":\"top agent frameworks\",\"source\":\"web\"}"),
        "query_payload_dump_detected"
    );
}

#[test]
fn search_query_shape_error_flags_direct_url_as_fetch_preferred() {
    assert_eq!(
        search_query_shape_error_code("https://example.com/research"),
        "query_prefers_fetch_url"
    );
}

#[test]
fn search_query_shape_contract_reports_invisible_unicode_stripping() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = api_search(
        tmp.path(),
        &json!({"query":"https://exa\u{200B}mple.com/docs"}),
    );
    assert_eq!(
        out.pointer("/query_shape/invisible_unicode_stripped")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        out.pointer("/query_shape/invisible_unicode_removed_count")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            >= 1,
        true
    );
}

#[test]
fn search_early_validation_blocks_direct_url_query_with_fetch_route_hint() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = api_search(tmp.path(), &json!({"query":"https://example.com/docs"}));
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("query_prefers_fetch_url")
    );
    assert_eq!(
        out.get("query_shape_route_hint").and_then(Value::as_str),
        Some("web_fetch")
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
        out.pointer("/suggested_next_action/payload/requested_url")
            .and_then(Value::as_str),
        Some("https://example.com/docs")
    );
}

#[test]
fn search_early_validation_blocks_query_shape_invalid_before_provider_validation() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let long_query = "top agent frameworks ".repeat(40);
    let out = api_search(
        tmp.path(),
        &json!({
            "query": long_query,
            "provider": "definitely-not-a-provider"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("query_shape_invalid")
    );
    assert_eq!(
        out.get("query_shape_blocked").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        out.get("provider").and_then(Value::as_str),
        Some("none")
    );
    assert_eq!(
        out.get("tool_execution_attempted").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn search_early_validation_shape_override_allows_next_validation_phase() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let long_query = "top agent frameworks ".repeat(40);
    let out = api_search(
        tmp.path(),
        &json!({
            "query": long_query,
            "provider": "definitely-not-a-provider",
            "allow_query_blob_search": true
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("unknown_search_provider")
    );
}
