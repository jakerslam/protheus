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
