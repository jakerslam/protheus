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
