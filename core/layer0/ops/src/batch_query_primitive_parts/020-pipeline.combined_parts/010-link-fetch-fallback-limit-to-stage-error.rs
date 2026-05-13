const INTERNAL_ROUTE_HINT: &str =
    "This looks like an internal command mapping request, not a web search query. Use local route diagnostics instead of web retrieval.";

fn fixture_payload_for_stage_url(stage: &str, url: &str) -> Option<Value> {
    let fixtures = fixture_payload_map()?;
    let stage_key = format!("{stage}::{url}");
    fixtures
        .get(&stage_key)
        .cloned()
        .or_else(|| fixtures.get(&format!("fetch::{url}")).cloned())
        .or_else(|| fixtures.get(&format!("url::{url}")).cloned())
}

fn fixture_mode_enabled() -> bool {
    std::env::var("INFRING_BATCH_QUERY_TEST_FIXTURE_JSON")
        .map(|raw| !raw.trim().is_empty())
        .unwrap_or(false)
}

fn fixture_missing_payload() -> Value {
    json!({
        "ok": false,
        "error": "fixture_missing"
    })
}

fn stage_search_payload(
    root: &Path,
    stage: Option<&str>,
    query: &str,
    provider: Option<&str>,
    search_scope: &BatchQuerySearchScope,
) -> Value {
    if let Some(stage_name) = stage {
        if let Some(payload) = fixture_payload_for_stage_query(stage_name, query) {
            return payload;
        }
    } else if let Some(payload) = fixture_payload_for_query(query) {
        return payload;
    }
    if fixture_mode_enabled() {
        return fixture_missing_payload();
    }
    let request = stage_search_request(query, provider, search_scope);
    crate::web_conduit::api_search(root, &request)
}

fn stage_search_request(
    query: &str,
    provider: Option<&str>,
    search_scope: &BatchQuerySearchScope,
) -> Value {
    let mut request = json!({
        "query": query,
        "summary_only": false
    });
    if let Some(provider_name) = provider {
        request["provider"] = Value::String(provider_name.to_string());
    }
    if !search_scope.allowed_domains.is_empty() {
        request["allowed_domains"] = json!(search_scope.allowed_domains.clone());
        request["exclude_subdomains"] = json!(search_scope.exclude_subdomains);
    }
    request
}

fn stage_fetch_payload(root: &Path, stage: &str, url: &str, extract_mode: &str) -> Value {
    if let Some(payload) = fixture_payload_for_stage_url(stage, url) {
        return payload;
    }
    if fixture_mode_enabled() {
        return fixture_missing_payload();
    }
    crate::web_conduit::api_fetch(
        root,
        &json!({
            "url": url,
            "extract_mode": extract_mode,
            "summary_only": false
        }),
    )
}

fn payload_links_for_fallback(query: &str, payload: &Value, max_links: usize) -> Vec<String> {
    ranked_payload_links_for_fallback(query, payload, max_links)
}

fn payload_links_for_page_extraction(
    query: &str,
    policy: &Value,
    payload: &Value,
    max_links: usize,
) -> Vec<String> {
    let limit = max_links.max(1);
    let mut selected = Vec::<String>::new();
    let mut selected_by_key = HashMap::<String, usize>::new();
    for link in ranked_payload_links_for_fallback_with_min_score(
        query,
        payload,
        max_links.saturating_mul(4).max(max_links),
        page_extraction_min_link_score(policy),
    )
    .into_iter()
    .filter_map(|link| normalize_page_extraction_link(policy, &link))
    {
        let dedupe_key = page_extraction_link_dedupe_key(policy, &link);
        if dedupe_key.is_empty() {
            continue;
        }
        if let Some(index) = selected_by_key.get(&dedupe_key).copied() {
            if should_prefer_page_extraction_link(&link, &selected[index]) {
                selected[index] = link;
            }
            continue;
        }
        if selected.len() >= limit {
            continue;
        }
        selected_by_key.insert(dedupe_key, selected.len());
        selected.push(link);
    }
    selected
}

fn normalize_page_extraction_link(policy: &Value, link: &str) -> Option<String> {
    let mut cleaned = clean_text(link, 2_200);
    if cleaned.is_empty() {
        return None;
    }
    if !page_extraction_url_hygiene_enabled(policy) {
        return Some(cleaned);
    }
    let lowered = cleaned.to_ascii_lowercase();
    if page_extraction_require_http_protocol(policy)
        && !(lowered.starts_with("http://") || lowered.starts_with("https://"))
    {
        return None;
    }
    if let Some((without_fragment, _)) = cleaned.split_once('#') {
        if page_extraction_drop_fragment_for_dedupe(policy) {
            cleaned = without_fragment.to_string();
        }
    }
    let without_query = cleaned
        .split_once('?')
        .map(|(value, _)| value)
        .unwrap_or(cleaned.as_str())
        .to_ascii_lowercase();
    if page_extraction_excluded_file_extensions(policy)
        .iter()
        .any(|extension| without_query.ends_with(extension))
    {
        return None;
    }
    let domain = extract_domains_from_text(&cleaned, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    if domain.is_empty() || is_search_engine_domain(&domain) {
        return None;
    }
    Some(cleaned)
}

fn page_extraction_link_dedupe_key(policy: &Value, link: &str) -> String {
    if !page_extraction_canonical_dedupe_enabled(policy) {
        return link.to_ascii_lowercase();
    }
    let Some((_, host, path, query)) = parse_page_extraction_http_url(link) else {
        return link.to_ascii_lowercase();
    };
    let host = host.trim_start_matches("www.").to_ascii_lowercase();
    if host.is_empty() {
        return link.to_ascii_lowercase();
    }
    let mut path = path.trim_end_matches('/').to_string();
    if path.is_empty() {
        path = "/".to_string();
    }
    match query {
        Some(query) if !query.is_empty() => format!("{host}{path}?{query}"),
        _ => format!("{host}{path}"),
    }
}

fn should_prefer_page_extraction_link(candidate: &str, current: &str) -> bool {
    let (Some((candidate_scheme, candidate_host, _, _)), Some((current_scheme, current_host, _, _))) = (
        parse_page_extraction_http_url(candidate),
        parse_page_extraction_http_url(current),
    ) else {
        return false;
    };
    if candidate_scheme == "https" && current_scheme == "http" {
        return true;
    }
    if candidate_scheme != current_scheme {
        return false;
    }
    current_host.starts_with("www.") && !candidate_host.starts_with("www.")
}

fn parse_page_extraction_http_url(link: &str) -> Option<(&str, &str, &str, Option<&str>)> {
    let trimmed = link.trim();
    let lowered = trimmed.to_ascii_lowercase();
    let (scheme, after_scheme) = if lowered.starts_with("https://") {
        ("https", &trimmed[8..])
    } else if lowered.starts_with("http://") {
        ("http", &trimmed[7..])
    } else {
        return None;
    };
    let host_end = after_scheme
        .find(['/', '?'])
        .unwrap_or(after_scheme.len());
    let host_with_port = &after_scheme[..host_end];
    let host = host_with_port
        .rsplit_once('@')
        .map(|(_, value)| value)
        .unwrap_or(host_with_port)
        .split_once(':')
        .map(|(value, _)| value)
        .unwrap_or(host_with_port);
    if host.is_empty() {
        return None;
    }
    let remainder = &after_scheme[host_end..];
    if remainder.is_empty() {
        return Some((scheme, host, "/", None));
    }
    if let Some(query) = remainder.strip_prefix('?') {
        return Some((scheme, host, "/", Some(query)));
    }
    let (path, query) = remainder
        .split_once('?')
        .map(|(path, query)| (path, Some(query)))
        .unwrap_or((remainder, None));
    Some((scheme, host, path, query))
}

fn query_overlap_terms(query: &str, candidate: &Candidate) -> usize {
    let query_tokens = tokenize_relevance(query, 40);
    if query_tokens.is_empty() {
        return 0;
    }
    let candidate_tokens = tokenize_relevance(&candidate_relevance_text(candidate), 120);
    if candidate_tokens.is_empty() {
        return 0;
    }
    query_tokens
        .iter()
        .filter(|token| candidate_tokens.contains(token.as_str()))
        .count()
}

fn candidate_is_substantive(query: &str, candidate: &Candidate, benchmark_intent: bool) -> bool {
    let snippet = clean_text(&candidate.snippet, 1_800);
    if snippet.is_empty() {
        return false;
    }
    if contains_antibot_marker(&snippet) || contains_antibot_marker(&candidate.title) {
        return false;
    }
    if contains_web_junk_marker(&snippet) || contains_web_junk_marker(&candidate.title) {
        return false;
    }
    if looks_like_off_intent_noise_candidate(query, candidate) {
        return false;
    }
    if looks_like_domain_list_noise(&snippet) {
        return false;
    }
    let word_count = snippet.split_whitespace().count();
    let overlap = query_overlap_terms(query, candidate);
    if benchmark_intent {
        if word_count < 8 && overlap < 2 {
            return false;
        }
    } else if word_count < 6 && overlap < 1 {
        return false;
    }
    if is_framework_catalog_intent(query) && word_count < 8 && overlap < 2 {
        let combined = format!("{} {}", candidate.title, snippet);
        let domain = candidate_domain_hint(candidate);
        if framework_official_domain(&domain) && looks_like_framework_overview_text(&combined) {
            return true;
        }
        return false;
    }
    true
}

fn candidate_is_synthesis_eligible(
    query: &str,
    candidate: &Candidate,
    benchmark_intent: bool,
) -> bool {
    let framework_catalog_intent = is_framework_catalog_intent(query);
    let snippet = clean_text(&candidate.snippet, 1_200);
    let lowered_snippet = snippet.to_ascii_lowercase();
    if lowered_snippet.contains("candidate domains include")
        && lowered_snippet.contains("require direct page verification")
    {
        return false;
    }
    if !candidate_passes_relevance_gate(query, candidate, benchmark_intent) {
        return false;
    }
    if looks_like_ack_only(&candidate.snippet)
        || looks_like_low_signal_search_summary(&candidate.snippet)
        || looks_like_source_only_snippet(&candidate.snippet)
    {
        return false;
    }
    if !candidate_is_substantive(query, candidate, benchmark_intent) {
        return false;
    }
    let domain = candidate_domain_hint(candidate);
    if is_search_engine_domain(&domain) {
        return false;
    }
    if benchmark_intent {
        if looks_like_definition_candidate(candidate)
            || looks_like_comparison_noise_candidate(candidate)
        {
            return false;
        }
        if !looks_like_metric_rich_text(&candidate.snippet)
            && query_overlap_terms(query, candidate) < 2
        {
            return false;
        }
    }
    if framework_catalog_intent
        && !looks_like_framework_catalog_text(&format!("{} {}", candidate.title, candidate.snippet))
        && !looks_like_framework_overview_text(&format!("{} {}", candidate.title, candidate.snippet))
        && query_overlap_terms(query, candidate) < 2
    {
        return false;
    }
    true
}

fn stage_error(payload: &Value, fallback: &str) -> String {
    clean_text(
        payload
            .get("error")
            .or_else(|| payload.pointer("/result/error"))
            .and_then(Value::as_str)
            .unwrap_or(fallback),
        220,
    )
}
