const LINK_FETCH_FALLBACK_LIMIT: usize = 2;
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
    let mut request = json!({
        "query": query,
        "summary_only": false
    });
    if let Some(provider_name) = provider {
        request["provider"] = Value::String(provider_name.to_string());
    }
    crate::web_conduit::api_search(root, &request)
}

fn stage_fetch_payload(root: &Path, stage: &str, url: &str) -> Value {
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
            "summary_only": false
        }),
    )
}

fn payload_links_for_fallback(payload: &Value, max_links: usize) -> Vec<String> {
    non_search_engine_links(payload, max_links)
}

fn framework_catalog_official_urls(query: &str) -> Vec<String> {
    if !is_framework_catalog_intent(query) {
        return Vec::new();
    }
    vec![
        "https://www.langchain.com/langgraph".to_string(),
        "https://openai.github.io/openai-agents-python/".to_string(),
        "https://microsoft.github.io/autogen/".to_string(),
        "https://crewai.com/".to_string(),
        "https://github.com/huggingface/smolagents".to_string(),
    ]
}

fn framework_catalog_candidate_coverage(candidates: &[Candidate]) -> usize {
    let mut seen = HashSet::<String>::new();
    for candidate in candidates {
        let combined = format!("{} {} {}", candidate.title, candidate.snippet, candidate.locator);
        for framework in framework_names_in_text(&combined) {
            seen.insert(framework.to_ascii_lowercase());
        }
    }
    seen.len()
}

fn query_overlap_terms(query: &str, candidate: &Candidate) -> usize {
    let query_tokens = tokenize_relevance(query, 40);
    if query_tokens.is_empty() {
        return 0;
    }
    let candidate_tokens = tokenize_relevance(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        120,
    );
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
