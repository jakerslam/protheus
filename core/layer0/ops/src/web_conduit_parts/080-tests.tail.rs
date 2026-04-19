#[test]
fn render_bing_rss_payload_filters_domains_and_builds_content() {
    let body = r#"
    <rss><channel>
      <item>
        <title>Main Result</title>
        <link>https://example.com/main</link>
        <description>Main description text</description>
      </item>
      <item>
        <title>Other Result</title>
        <link>https://other.com/page</link>
        <description>Other description text</description>
      </item>
    </channel></rss>
    "#;
    let rendered = render_bing_rss_payload(body, &vec!["example.com".to_string()], true, 8, 12_000);
    assert_eq!(rendered.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        rendered.get("provider_raw_count").and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        rendered
            .get("provider_filtered_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    let links = rendered
        .get("links")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].as_str(), Some("https://example.com/main"));
}

#[test]
fn payload_challenge_detector_flags_duckduckgo_challenge_dump() {
    let payload = json!({
        "summary": "DuckDuckGo challenge",
        "content": "Unfortunately, bots use DuckDuckGo too. Please complete the following challenge. Select all squares containing a duck."
    });
    assert!(payload_looks_like_search_challenge(&payload));
}

#[test]
fn payload_low_signal_detector_flags_duckduckgo_chrome_summary() {
    let payload = json!({
        "summary": "latest technology news today at DuckDuckGo All Regions Argentina Australia Safe Search Any Time",
        "content": ""
    });
    assert!(payload_looks_low_signal_search(&payload));
}

#[test]
fn payload_low_signal_detector_flags_source_scaffold_summary() {
    let payload = json!({
        "summary": "Key findings for \"Infring AI vs competitors\": - Potential sources: hai.stanford.edu, artificialanalysis.ai.",
        "content": ""
    });
    assert!(payload_looks_low_signal_search(&payload));
}

#[test]
fn challenge_like_failure_detector_requires_only_low_signal_or_challenge_errors() {
    let out = json!({
        "ok": false,
        "error": "search_providers_exhausted"
    });
    let provider_errors = vec![
        json!({"provider": "duckduckgo", "challenge": true, "low_signal": false}),
        json!({"provider": "duckduckgo_lite", "challenge": false, "low_signal": true}),
    ];
    assert!(search_failure_is_challenge_like(&out, provider_errors.as_slice()));
}

#[test]
fn challenge_like_failure_detector_rejects_mixed_error_causes() {
    let out = json!({
        "ok": false,
        "error": "search_providers_exhausted"
    });
    let provider_errors = vec![
        json!({"provider": "duckduckgo", "challenge": true, "low_signal": false}),
        json!({"provider": "bing_rss", "challenge": false, "low_signal": false}),
    ];
    assert!(!search_failure_is_challenge_like(
        &out,
        provider_errors.as_slice()
    ));
}

#[test]
fn meta_query_detector_flags_workflow_diagnostic_prompt() {
    assert!(search_query_is_meta_diagnostic(
        "was this a bad web request or training data hallucination again"
    ));
}

#[test]
fn meta_query_detector_allows_normal_research_prompt() {
    assert!(!search_query_is_meta_diagnostic(
        "top ai agent frameworks official docs"
    ));
}

#[test]
fn meta_query_detector_allows_single_meta_term_research_query() {
    assert!(!search_query_is_meta_diagnostic(
        "hallucination mitigation techniques for llm responses"
    ));
}

#[test]
fn meta_query_detector_allows_explicit_search_intent_with_meta_phrase() {
    assert!(!search_query_is_meta_diagnostic(
        "did you try it? search for current top ai agent frameworks"
    ));
}

#[test]
fn meta_query_detector_does_not_treat_topic_noun_as_search_intent() {
    assert!(search_query_is_meta_diagnostic(
        "why did my last prompt about agent frameworks hallucination fail"
    ));
}

#[test]
fn meta_query_detector_flags_short_conversational_test_prompt() {
    assert!(search_query_is_meta_diagnostic("that was just a test"));
}

#[test]
fn search_early_validation_blocks_meta_query() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = api_search(
        tmp.path(),
        &json!({
            "query": "did you do the web request or was that a hallucination"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("non_search_meta_query")
    );
    assert_eq!(
        out.get("meta_query_blocked").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        out.get("type").and_then(Value::as_str),
        Some("web_conduit_search")
    );
    assert_eq!(out.get("provider").and_then(Value::as_str), Some("none"));
    assert_eq!(
        out.get("cache_status").and_then(Value::as_str),
        Some("blocked_meta_query")
    );
    assert_eq!(
        out.get("cache_store_allowed").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        out.get("cache_skip_reason").and_then(Value::as_str),
        Some("meta_query_blocked")
    );
    assert_eq!(
        out.get("cache_write_attempted").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(out.get("provider_hint").and_then(Value::as_str), Some("auto"));
    assert_eq!(
        out.get("override_hint").and_then(Value::as_str),
        Some("force_web_search=true|force_web_lookup=true")
    );
    assert_eq!(
        out.get("tool_execution_attempted").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        out.get("tool_execution_skipped_reason")
            .and_then(Value::as_str),
        Some("meta_query_blocked")
    );
    assert_eq!(
        out.get("validation_route").and_then(Value::as_str),
        Some("meta_query_blocked")
    );
    assert_eq!(
        out.get("providers_attempted")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(0)
    );
    assert_eq!(
        out.pointer("/tool_execution_gate/should_execute")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        out.pointer("/tool_execution_gate/reason")
            .and_then(Value::as_str),
        Some("meta_query_blocked")
    );
    assert_eq!(
        out.get("provider_chain")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(0)
    );
    assert_eq!(
        out.pointer("/provider_resolution/reason")
            .and_then(Value::as_str),
        Some("meta_query_blocked")
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
    assert_eq!(out.get("provider_hint").and_then(Value::as_str), Some("auto"));
}

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
}

#[test]
fn search_early_validation_allows_meta_query_when_override_enabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = search_early_validation_response(
        tmp.path(),
        &json!({"allow_meta_query_search": true}),
        "did you do the web request or was that a hallucination",
    );
    assert!(out.is_none());
}

#[test]
fn search_early_validation_allows_meta_query_when_force_web_search_enabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = search_early_validation_response(
        tmp.path(),
        &json!({"force_web_search": true}),
        "did you do the web request or was that a hallucination",
    );
    assert!(out.is_none());
}

#[test]
fn search_early_validation_allows_meta_query_when_force_web_search_string_enabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = search_early_validation_response(
        tmp.path(),
        &json!({"forceWebSearch": "true"}),
        "did you do the web request or was that a hallucination",
    );
    assert!(out.is_none());
}

#[test]
fn search_early_validation_allows_meta_query_when_force_web_lookup_enabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = search_early_validation_response(
        tmp.path(),
        &json!({"force_web_lookup": 1}),
        "did you do the web request or was that a hallucination",
    );
    assert!(out.is_none());
}

#[test]
fn search_early_validation_allows_meta_query_when_nested_force_lookup_enabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = search_early_validation_response(
        tmp.path(),
        &json!({"searchPolicy": {"forceWebLookup": "yes"}}),
        "did you do the web request or was that a hallucination",
    );
    assert!(out.is_none());
}

#[test]
fn search_uses_cached_response_when_available() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let request = json!({
        "query": "agent reliability benchmark",
        "summary_only": true
    });
    let query = clean_text(
        request
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        600,
    );
    let allowed_domains =
        normalize_allowed_domains(request.get("allowed_domains").unwrap_or(&Value::Null));
    let exclude_subdomains = request
        .get("exclude_subdomains")
        .or_else(|| request.get("exact_domain_only"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let top_k = 8usize;
    let summary_only = true;
    let scoped_query = scoped_search_query(&query, &allowed_domains, exclude_subdomains);
    let (policy, _) = load_policy(tmp.path());
    let provider_chain = crate::web_conduit_provider_runtime::provider_chain_from_request(
        "auto", &request, &policy,
    );
    let key = crate::web_conduit_provider_runtime::search_cache_key(
        &query,
        &scoped_query,
        &allowed_domains,
        exclude_subdomains,
        top_k,
        summary_only,
        &provider_chain,
    );
    crate::web_conduit_provider_runtime::store_search_cache(
        tmp.path(),
        &key,
        &json!({
            "ok": true,
            "summary": "cached search summary",
            "content": "",
            "provider": "duckduckgo"
        }),
        "ok",
        None,
    );

    let out = api_search(tmp.path(), &request);
    assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        out.get("summary").and_then(Value::as_str),
        Some("cached search summary")
    );
    assert_eq!(out.get("cache_status").and_then(Value::as_str), Some("hit"));
}
