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

#[test]
fn search_query_shape_error_flags_markdown_link_as_fetch_intent() {
    assert_eq!(
        search_query_shape_error_code("[official docs](https://example.com/agents)"),
        "query_prefers_fetch_url"
    );
}

#[test]
fn search_query_shape_suggested_next_action_extracts_markdown_link_url() {
    let action = search_query_shape_suggested_next_action(
        "[official docs](https://example.com/agents?x=1)",
        "query_prefers_fetch_url",
    );
    assert_eq!(
        action.pointer("/action").and_then(Value::as_str),
        Some("web_conduit_fetch")
    );
    assert_eq!(
        action
            .pointer("/payload/requested_url")
            .and_then(Value::as_str),
        Some("https://example.com/agents?x=1")
    );
}

#[test]
fn api_search_blocks_markdown_link_query_with_fetch_route_hint() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = api_search(
        tmp.path(),
        &json!({
            "query": "[OpenAI docs](https://platform.openai.com/docs)"
        }),
    );
    assert_eq!(
        out.get("query_shape_error").and_then(Value::as_str),
        Some("query_prefers_fetch_url")
    );
    assert_eq!(
        out.get("query_shape_route_hint").and_then(Value::as_str),
        Some("web_fetch")
    );
    assert_eq!(
        out.pointer("/suggested_next_action/payload/requested_url")
            .and_then(Value::as_str),
        Some("https://platform.openai.com/docs")
    );
    assert_eq!(
        out.pointer("/retry/strategy").and_then(Value::as_str),
        Some("use_web_fetch_route")
    );
    assert_eq!(
        out.pointer("/retry/reason").and_then(Value::as_str),
        Some("query_prefers_fetch_url")
    );
    assert_eq!(
        out.pointer("/retry/contract_version").and_then(Value::as_str),
        Some("v1")
    );
    assert_eq!(
        out.pointer("/retry/lane").and_then(Value::as_str),
        Some("web_fetch")
    );
}

#[test]
fn search_query_shape_extracts_wrapped_url_candidate() {
    let action = search_query_shape_suggested_next_action(
        "\"<https://example.com/research?q=agent&amp;mode=1>,\"",
        "query_prefers_fetch_url",
    );
    assert_eq!(
        action
            .pointer("/payload/requested_url")
            .and_then(Value::as_str),
        Some("https://example.com/research?q=agent&mode=1")
    );
}

#[test]
fn search_query_shape_supports_www_candidate() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "query": "www.example.com/docs"
        }),
    );
    assert_eq!(
        out.get("query_shape_error").and_then(Value::as_str),
        Some("query_prefers_fetch_url")
    );
    assert_eq!(
        out.pointer("/query_shape/fetch_url_candidate")
            .and_then(Value::as_str),
        Some("https://www.example.com/docs")
    );
}

#[test]
fn fetch_request_uses_query_url_when_url_field_missing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "query": "[read this](https://example.com/path?from=query)",
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("unknown_fetch_provider")
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/path?from=query")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("query")
    );
}

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

#[test]
fn search_unknown_provider_fail_closed_includes_retry_strategy() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "query": "agent frameworks docs",
            "provider": "definitely-not-a-provider"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("unknown_search_provider")
    );
    assert_eq!(
        out.pointer("/retry/recommended").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        out.pointer("/retry/strategy").and_then(Value::as_str),
        Some("use_supported_provider_or_auto")
    );
    assert_eq!(
        out.pointer("/retry/reason").and_then(Value::as_str),
        Some("unknown_search_provider")
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
        Some("switch_provider")
    );
    assert_eq!(
        out.pointer("/retry/priority").and_then(Value::as_str),
        Some("medium")
    );
    assert_eq!(
        out.pointer("/retry/operator_action_hint")
            .and_then(Value::as_str),
        Some("set_provider_auto_or_supported_provider")
    );
    assert_eq!(
        out.pointer("/retry/operator_owner").and_then(Value::as_str),
        Some("operator")
    );
    assert_eq!(
        out.pointer("/retry/diagnostic_code").and_then(Value::as_str),
        Some("search_retry_unknown_search_provider")
    );
    assert_eq!(
        out.pointer("/retry/blocking_kind").and_then(Value::as_str),
        Some("provider_configuration_required")
    );
    assert_eq!(
        out.pointer("/retry/auto_retry_allowed")
            .and_then(Value::as_bool),
        Some(true)
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
fn search_conflicting_time_filters_includes_retry_strategy() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "query": "agent frameworks",
            "freshness": "week",
            "date_before": "2026-04-10"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("conflicting_time_filters")
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
fn fetch_unknown_provider_fail_closed_includes_retry_strategy() {
    let out = execute_fetch_request(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "url": "https://example.com",
            "provider": "definitely-not-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("unknown_fetch_provider")
    );
    assert_eq!(
        out.pointer("/retry/recommended").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        out.pointer("/retry/strategy").and_then(Value::as_str),
        Some("use_supported_provider_or_auto")
    );
    assert_eq!(
        out.pointer("/retry/reason").and_then(Value::as_str),
        Some("unknown_fetch_provider")
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
        Some("switch_provider")
    );
    assert_eq!(
        out.pointer("/retry/priority").and_then(Value::as_str),
        Some("medium")
    );
    assert_eq!(
        out.pointer("/retry/operator_action_hint")
            .and_then(Value::as_str),
        Some("set_fetch_provider_auto_or_supported_provider")
    );
    assert_eq!(
        out.pointer("/retry/operator_owner").and_then(Value::as_str),
        Some("operator")
    );
    assert_eq!(
        out.pointer("/retry/diagnostic_code").and_then(Value::as_str),
        Some("fetch_retry_unknown_fetch_provider")
    );
    assert_eq!(
        out.pointer("/retry/blocking_kind").and_then(Value::as_str),
        Some("provider_configuration_required")
    );
    assert_eq!(
        out.pointer("/retry/auto_retry_allowed")
            .and_then(Value::as_bool),
        Some(true)
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
        Some("web_fetch")
    );
}

#[test]
fn search_retry_envelope_helper_supports_unsupported_filter_contract() {
    let retry = search_retry_envelope_for_error("unsupported_search_filter");
    assert_eq!(
        retry.get("strategy").and_then(Value::as_str),
        Some("remove_unsupported_filter")
    );
    assert_eq!(
        retry.get("reason").and_then(Value::as_str),
        Some("unsupported_search_filter")
    );
    assert_eq!(
        retry.get("contract_version").and_then(Value::as_str),
        Some("v1")
    );
    assert_eq!(
        retry.get("contract_family").and_then(Value::as_str),
        Some("web_retry_contract_v1")
    );
    assert_eq!(
        retry.get("recovery_mode").and_then(Value::as_str),
        Some("adjust_filters")
    );
    assert_eq!(
        retry.get("priority").and_then(Value::as_str),
        Some("medium")
    );
    assert_eq!(
        retry.get("operator_action_hint").and_then(Value::as_str),
        Some("remove_or_replace_unsupported_filter")
    );
    assert_eq!(
        retry.get("operator_owner").and_then(Value::as_str),
        Some("user")
    );
    assert_eq!(
        retry.get("diagnostic_code").and_then(Value::as_str),
        Some("search_retry_unsupported_search_filter")
    );
    assert_eq!(
        retry.get("blocking_kind").and_then(Value::as_str),
        Some("input_adjustment_required")
    );
    assert_eq!(
        retry.get("auto_retry_allowed").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        retry.get("escalation_lane").and_then(Value::as_str),
        Some("user_input")
    );
    assert_eq!(
        retry
            .get("requires_manual_confirmation")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        retry.get("execution_policy").and_then(Value::as_str),
        Some("manual_gate_required")
    );
    assert_eq!(
        retry.get("manual_gate_reason").and_then(Value::as_str),
        Some("input_adjustment_required")
    );
    assert_eq!(
        retry.get("requeue_strategy").and_then(Value::as_str),
        Some("manual")
    );
    assert_eq!(
        retry.get("can_execute_without_human").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        retry.get("execution_window").and_then(Value::as_str),
        Some("after_manual_gate")
    );
    assert_eq!(
        retry
            .get("manual_gate_timeout_seconds")
            .and_then(Value::as_i64),
        Some(1800)
    );
    assert_eq!(
        retry.get("next_action_after_seconds").and_then(Value::as_i64),
        Some(1800)
    );
    assert_eq!(
        retry.get("readiness_state").and_then(Value::as_str),
        Some("manual_gate_pending")
    );
    assert_eq!(retry.get("retryable").and_then(Value::as_bool), Some(true));
    assert_eq!(retry.get("idempotent").and_then(Value::as_bool), Some(true));
    assert_eq!(
        retry.get("category").and_then(Value::as_str),
        Some("validation")
    );
    assert_eq!(retry.get("lane").and_then(Value::as_str), Some("web_search"));
}

#[test]
fn fetch_retry_envelope_runtime_helper_pins_contract_and_reason() {
    let retry = fetch_retry_envelope_runtime(
        "change_query_or_provider",
        "web_fetch_duplicate_attempt_suppressed",
        "web_fetch",
        15,
    );
    assert_eq!(
        retry.get("strategy").and_then(Value::as_str),
        Some("change_query_or_provider")
    );
    assert_eq!(
        retry.get("reason").and_then(Value::as_str),
        Some("web_fetch_duplicate_attempt_suppressed")
    );
    assert_eq!(
        retry.get("contract_version").and_then(Value::as_str),
        Some("v1")
    );
    assert_eq!(
        retry.get("contract_family").and_then(Value::as_str),
        Some("web_retry_contract_v1")
    );
    assert_eq!(
        retry.get("recovery_mode").and_then(Value::as_str),
        Some("adjust_query_or_provider")
    );
    assert_eq!(
        retry.get("priority").and_then(Value::as_str),
        Some("medium")
    );
    assert_eq!(
        retry.get("operator_action_hint").and_then(Value::as_str),
        Some("adjust_query_or_wait_for_retry_window")
    );
    assert_eq!(
        retry.get("operator_owner").and_then(Value::as_str),
        Some("system_operator")
    );
    assert_eq!(
        retry.get("diagnostic_code").and_then(Value::as_str),
        Some("fetch_retry_duplicate_attempt_suppressed")
    );
    assert_eq!(
        retry.get("blocking_kind").and_then(Value::as_str),
        Some("cooldown_required")
    );
    assert_eq!(
        retry.get("auto_retry_allowed").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        retry.get("escalation_lane").and_then(Value::as_str),
        Some("automation")
    );
    assert_eq!(
        retry
            .get("requires_manual_confirmation")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        retry.get("execution_policy").and_then(Value::as_str),
        Some("deferred_auto_retry")
    );
    assert_eq!(
        retry.get("manual_gate_reason").and_then(Value::as_str),
        Some("none")
    );
    assert_eq!(
        retry.get("requeue_strategy").and_then(Value::as_str),
        Some("deferred")
    );
    assert_eq!(
        retry.get("can_execute_without_human").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        retry.get("execution_window").and_then(Value::as_str),
        Some("after_retry_after")
    );
    assert_eq!(
        retry
            .get("manual_gate_timeout_seconds")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        retry.get("next_action_after_seconds").and_then(Value::as_i64),
        Some(15)
    );
    assert_eq!(
        retry.get("readiness_state").and_then(Value::as_str),
        Some("deferred_retry_pending")
    );
    assert_eq!(
        retry.get("retry_after_seconds").and_then(Value::as_i64),
        Some(15)
    );
    assert_eq!(retry.get("retryable").and_then(Value::as_bool), Some(true));
    assert_eq!(retry.get("idempotent").and_then(Value::as_bool), Some(true));
    assert_eq!(
        retry.get("category").and_then(Value::as_str),
        Some("execution")
    );
    assert_eq!(retry.get("lane").and_then(Value::as_str), Some("web_fetch"));
}
