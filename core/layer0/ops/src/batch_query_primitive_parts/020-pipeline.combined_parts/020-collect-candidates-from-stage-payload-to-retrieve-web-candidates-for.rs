
fn collect_candidates_from_stage_payload(
    root: &Path,
    stage: &str,
    query: &str,
    policy: &Value,
    payload: &Value,
    benchmark_intent: bool,
    fetched_links: &mut HashSet<String>,
) -> (Vec<Candidate>, Vec<String>, Option<Value>) {
    let mut candidates = Vec::<Candidate>::new();
    let mut issues = Vec::<String>::new();
    let low_relevance_issue = |candidate: &Candidate, suffix: &str| {
        if looks_like_competitive_programming_dump(&format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        )) {
            format!("{stage}:query_result_mismatch")
        } else {
            format!("{stage}:{suffix}")
        }
    };
    if structured_results_enabled(policy) {
        for candidate in candidates_from_structured_search_payload(
            query,
            payload,
            structured_results_max_rows_per_stage(policy),
        ) {
            if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                candidates.push(candidate);
            } else {
                issues.push(low_relevance_issue(&candidate, "candidate_low_relevance"));
            }
        }
    }
    let rendered_rows = candidates_from_rendered_search_payload(
        query,
        payload,
        if is_framework_catalog_intent(query) { 4 } else { 2 },
    );
    for candidate in rendered_rows {
        if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
            candidates.push(candidate);
        } else {
            issues.push(low_relevance_issue(&candidate, "candidate_low_relevance"));
        }
    }

    if candidates.is_empty() {
        match candidate_from_search_payload(query, payload) {
            Ok(candidate) => {
                if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                    candidates.push(candidate);
                } else {
                    issues.push(low_relevance_issue(&candidate, "candidate_low_relevance"));
                }
            }
            Err(err) => issues.push(format!("{stage}:{err}")),
        }
    }

    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    if contains_antibot_marker(&summary) || contains_antibot_marker(&content) {
        issues.push(format!("{stage}:anti_bot_challenge"));
    }
    if looks_like_competitive_programming_dump(&format!("{summary} {content}")) {
        issues.push(format!("{stage}:query_result_mismatch"));
    }
    if contains_web_junk_marker(&summary) || contains_web_junk_marker(&content) {
        issues.push(format!("{stage}:junk_page"));
    }
    let should_fetch_links = page_extraction_enabled(policy)
        && page_extraction_max_links_per_stage(policy) > 0
        && page_extraction_max_total_fetches(policy) > fetched_links.len()
        && (candidates.is_empty()
            || looks_like_low_signal_search_summary(&summary)
            || candidates
                .iter()
                .all(|candidate| candidate_needs_link_fetch(query, candidate)));
    if should_fetch_links {
        for link in payload_links_for_page_extraction(
            query,
            policy,
            payload,
            page_extraction_max_links_per_stage(policy),
        ) {
            if fetched_links.len() >= page_extraction_max_total_fetches(policy) {
                issues.push(format!("{stage}:page_extraction_budget_exhausted"));
                break;
            }
            if !fetched_links.insert(link.clone()) {
                continue;
            }
            let fetch_payload =
                stage_fetch_payload(root, stage, &link, &page_extraction_extract_mode(policy));
            if !fetch_payload
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                issues.push(format!(
                    "{stage}:fetch:{}",
                    stage_error(&fetch_payload, "web_fetch_failed")
                ));
                continue;
            }
            match candidate_from_search_payload(query, &fetch_payload) {
                Ok(mut candidate) => {
                    if candidate.locator.is_empty()
                        || is_search_engine_domain(&candidate_domain_hint(&candidate))
                    {
                        candidate.locator = link.clone();
                    }
                    if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                        candidates.push(candidate);
                    } else {
                        issues.push(low_relevance_issue(
                            &candidate,
                            "fetch_candidate_low_relevance",
                        ));
                    }
                }
                Err(err) => issues.push(format!("{stage}:fetch_candidate:{err}")),
            }
        }
    }
    let provider_result = hidden_provider_result_artifact(stage, query, payload, candidates.len(), &issues);
    (candidates, issues, provider_result)
}

fn hidden_provider_result_quality(
    payload_ok: bool,
    candidate_count: usize,
    issues: &[String],
) -> &'static str {
    if candidate_count > 0 {
        return "usable";
    }
    if !payload_ok {
        return "provider_error";
    }
    if issues.iter().any(|issue| {
        issue.contains("candidate_low_relevance")
            || issue.contains("fetch_candidate_low_relevance")
            || issue.contains("query_result_mismatch")
    }) {
        return "low_relevance";
    }
    if issues
        .iter()
        .any(|issue| issue.contains("no_usable_summary") || issue.contains("low_signal"))
    {
        return "low_signal";
    }
    "no_synthesis_candidate"
}

fn hidden_provider_result_artifact(
    stage: &str,
    query: &str,
    payload: &Value,
    candidate_count: usize,
    issues: &[String],
) -> Option<Value> {
    let query_text = clean_text(query, 600);
    let stage_name = clean_text(stage, 80);
    let provider = clean_text(
        payload
            .get("provider")
            .or_else(|| payload.get("source"))
            .and_then(Value::as_str)
            .unwrap_or(stage),
        80,
    );
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        1_200,
    );
    let content_preview = trim_words(
        &clean_text(
            payload.get("content").and_then(Value::as_str).unwrap_or(""),
            1_600,
        ),
        48,
    );
    let locator = clean_text(
        payload.get("requested_url").and_then(Value::as_str).unwrap_or(""),
        2_200,
    );
    let error = clean_text(payload.get("error").and_then(Value::as_str).unwrap_or(""), 240);
    let links = payload_links_for_fallback(query, payload, 3);
    if summary.is_empty()
        && content_preview.is_empty()
        && locator.is_empty()
        && error.is_empty()
        && links.is_empty()
    {
        return None;
    }
    let payload_ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false)
        || payload
            .get("provider_payload_rejected")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let result_quality = hidden_provider_result_quality(payload_ok, candidate_count, issues);
    let ok = result_quality == "usable";
    let status = clean_text(
        payload
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or(if payload_ok { "ok" } else { "error" }),
        32,
    );
    let mut out = serde_json::Map::new();
    if !query_text.is_empty() {
        out.insert("query".to_string(), Value::String(query_text));
    }
    if !stage_name.is_empty() {
        out.insert("stage".to_string(), Value::String(stage_name));
    }
    if !provider.is_empty() {
        out.insert("provider".to_string(), Value::String(provider));
    }
    out.insert("ok".to_string(), Value::Bool(ok));
    out.insert("provider_transport_ok".to_string(), Value::Bool(payload_ok));
    out.insert(
        "result_quality".to_string(),
        Value::String(result_quality.to_string()),
    );
    out.insert(
        "synthesis_candidate_count".to_string(),
        json!(candidate_count),
    );
    if !status.is_empty() {
        out.insert("status".to_string(), Value::String(status));
    }
    if !summary.is_empty() {
        out.insert("summary".to_string(), Value::String(summary));
    }
    if !content_preview.is_empty() && content_preview != out.get("summary").and_then(Value::as_str).unwrap_or("") {
        out.insert(
            "content_preview".to_string(),
            Value::String(content_preview),
        );
    }
    if !locator.is_empty() {
        out.insert("locator".to_string(), Value::String(locator));
    }
    if !error.is_empty() {
        out.insert("error".to_string(), Value::String(error));
    } else if payload_ok && !ok {
        out.insert(
            "error".to_string(),
            Value::String(result_quality.to_string()),
        );
    }
    if !links.is_empty() {
        out.insert(
            "links".to_string(),
            Value::Array(links.into_iter().map(Value::String).collect::<Vec<_>>()),
        );
    }
    Some(Value::Object(out))
}

fn retrieve_web_candidates_for_query(
    root: &Path,
    query: &str,
    policy: &Value,
    search_scope: &BatchQuerySearchScope,
) -> (Vec<Candidate>, Vec<String>, Vec<Value>) {
    let benchmark_intent = is_benchmark_or_comparison_intent(query);
    let mut candidates = Vec::<Candidate>::new();
    let mut issues = Vec::<String>::new();
    let mut provider_results = Vec::<Value>::new();
    let mut fetched_links = HashSet::<String>::new();

    let primary_payload = stage_search_payload(root, None, query, None, search_scope);
    let (primary_candidates, primary_issues, primary_provider_result) =
        collect_candidates_from_stage_payload(
        root,
        "primary",
        query,
        policy,
        &primary_payload,
        benchmark_intent,
        &mut fetched_links,
    );
    if let Some(value) = primary_provider_result {
        provider_results.push(value);
    }
    candidates.extend(primary_candidates);
    issues.extend(primary_issues);

    if candidates.is_empty()
        && issues
            .iter()
            .any(|issue| skip_duckduckgo_fallback_for_error(issue))
    {
        return (Vec::new(), issues, provider_results);
    }

    if candidates.is_empty() {
        let bing_payload =
            stage_search_payload(root, Some("bing_rss"), query, Some("bing"), search_scope);
        let (bing_candidates, bing_issues, bing_provider_result) =
            collect_candidates_from_stage_payload(
            root,
            "bing_rss",
            query,
            policy,
            &bing_payload,
            benchmark_intent,
            &mut fetched_links,
        );
        if let Some(value) = bing_provider_result {
            provider_results.push(value);
        }
        candidates.extend(bing_candidates);
        issues.extend(bing_issues);
    }

    if candidates.is_empty() {
        let fallback_url = duckduckgo_instant_answer_url(query);
        let fallback_payload =
            if let Some(payload) = fixture_payload_for_stage_query("duckduckgo_instant", query) {
                payload
            } else if fixture_mode_enabled() {
                fixture_missing_payload()
            } else {
                stage_fetch_payload(root, "duckduckgo_instant", &fallback_url, "text")
            };
        let mut duckduckgo_candidate_count = 0usize;
        let mut duckduckgo_issues = Vec::<String>::new();
        match candidate_from_duckduckgo_instant_payload(query, &fallback_url, &fallback_payload) {
            Ok(candidate) => {
                if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                    duckduckgo_candidate_count = 1;
                    candidates.push(candidate);
                } else {
                    duckduckgo_issues.push("duckduckgo_instant:candidate_low_relevance".to_string());
                }
            }
            Err(err) => duckduckgo_issues.push(format!("duckduckgo_instant:{err}")),
        }
        if let Some(value) = hidden_provider_result_artifact(
            "duckduckgo_instant",
            query,
            &fallback_payload,
            duckduckgo_candidate_count,
            &duckduckgo_issues,
        ) {
            provider_results.push(value);
        }
        issues.extend(duckduckgo_issues);
    }

    if candidates.is_empty() {
        for provider in provider_recovery_providers(policy, query) {
            let provider_payload =
                stage_search_payload(root, Some(&provider), query, Some(&provider), search_scope);
            let (mut provider_candidates, provider_issues, provider_result) =
                collect_candidates_from_stage_payload(
                    root,
                    &provider,
                    query,
                    policy,
                    &provider_payload,
                    benchmark_intent,
                    &mut fetched_links,
                );
            if let Some(value) = provider_result {
                provider_results.push(value);
            }
            issues.extend(provider_issues);
            if !provider_candidates.is_empty() {
                candidates.append(&mut provider_candidates);
                break;
            }
        }
    }

    if candidates.is_empty() {
        if issues.is_empty() {
            issues.push("no_usable_summary".to_string());
        }
        (Vec::new(), issues, provider_results)
    } else {
        let mut dedup = HashSet::<String>::new();
        let mut unique = Vec::<Candidate>::new();
        for candidate in candidates {
            let key = format!(
                "{}|{}|{}",
                candidate.locator.to_ascii_lowercase(),
                candidate.title.to_ascii_lowercase(),
                candidate.excerpt_hash
            );
            if dedup.insert(key) {
                unique.push(candidate);
            }
        }
        (unique, issues, provider_results)
    }
}
