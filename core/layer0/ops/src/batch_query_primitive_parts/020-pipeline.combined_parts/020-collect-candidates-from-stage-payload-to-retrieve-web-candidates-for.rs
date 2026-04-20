
fn collect_candidates_from_stage_payload(
    root: &Path,
    stage: &str,
    query: &str,
    payload: &Value,
    benchmark_intent: bool,
    fetched_links: &mut HashSet<String>,
) -> (Vec<Candidate>, Vec<String>) {
    let mut candidates = Vec::<Candidate>::new();
    let mut issues = Vec::<String>::new();
    let rendered_rows = candidates_from_rendered_search_payload(
        query,
        payload,
        if is_framework_catalog_intent(query) { 4 } else { 2 },
    );
    for candidate in rendered_rows {
        if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
            candidates.push(candidate);
        } else {
            issues.push(format!("{stage}:candidate_low_relevance"));
        }
    }

    if candidates.is_empty() {
        match candidate_from_search_payload(query, payload) {
            Ok(candidate) => {
                if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                    candidates.push(candidate);
                } else {
                    issues.push(format!("{stage}:candidate_low_relevance"));
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
    let should_fetch_links = candidates.is_empty()
        || looks_like_low_signal_search_summary(&summary)
        || candidates
            .iter()
            .all(|candidate| candidate_needs_link_fetch(query, candidate));
    if should_fetch_links {
        for link in payload_links_for_fallback(payload, LINK_FETCH_FALLBACK_LIMIT) {
            if !fetched_links.insert(link.clone()) {
                continue;
            }
            let fetch_payload = stage_fetch_payload(root, stage, &link);
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
                        issues.push(format!("{stage}:fetch_candidate_low_relevance"));
                    }
                }
                Err(err) => issues.push(format!("{stage}:fetch_candidate:{err}")),
            }
        }
    }
    (candidates, issues)
}

fn retrieve_web_candidates_for_query(root: &Path, query: &str) -> Result<Vec<Candidate>, String> {
    let benchmark_intent = is_benchmark_or_comparison_intent(query);
    let mut candidates = Vec::<Candidate>::new();
    let mut issues = Vec::<String>::new();
    let mut fetched_links = HashSet::<String>::new();

    let primary_payload = stage_search_payload(root, None, query, None);
    let (primary_candidates, primary_issues) = collect_candidates_from_stage_payload(
        root,
        "primary",
        query,
        &primary_payload,
        benchmark_intent,
        &mut fetched_links,
    );
    candidates.extend(primary_candidates);
    issues.extend(primary_issues);

    if candidates.is_empty()
        && issues
            .iter()
            .any(|issue| skip_duckduckgo_fallback_for_error(issue))
    {
        return Err(issues.join("|"));
    }

    if candidates.is_empty() {
        let bing_payload = stage_search_payload(root, Some("bing_rss"), query, Some("bing"));
        let (bing_candidates, bing_issues) = collect_candidates_from_stage_payload(
            root,
            "bing_rss",
            query,
            &bing_payload,
            benchmark_intent,
            &mut fetched_links,
        );
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
                stage_fetch_payload(root, "duckduckgo_instant", &fallback_url)
            };
        match candidate_from_duckduckgo_instant_payload(query, &fallback_url, &fallback_payload) {
            Ok(candidate) => {
                if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                    candidates.push(candidate);
                } else {
                    issues.push("duckduckgo_instant:candidate_low_relevance".to_string());
                }
            }
            Err(err) => issues.push(format!("duckduckgo_instant:{err}")),
        }
    }

    if is_framework_catalog_intent(query) && framework_catalog_candidate_coverage(&candidates) < 4 {
        for url in framework_catalog_official_urls(query) {
            if !fetched_links.insert(url.clone()) {
                continue;
            }
            let fetch_payload = stage_fetch_payload(root, "framework_official", &url);
            if !fetch_payload
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                issues.push(format!(
                    "framework_official:{}",
                    stage_error(&fetch_payload, "web_fetch_failed")
                ));
                continue;
            }
            match candidate_from_search_payload(query, &fetch_payload) {
                Ok(mut candidate) => {
                    if candidate.locator.is_empty()
                        || is_search_engine_domain(&candidate_domain_hint(&candidate))
                    {
                        candidate.locator = url.clone();
                    }
                    if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                        candidates.push(candidate);
                    } else {
                        issues.push("framework_official:fetch_candidate_low_relevance".to_string());
                    }
                }
                Err(err) => issues.push(format!("framework_official:fetch_candidate:{err}")),
            }
        }
    }

    if candidates.is_empty() {
        if issues.is_empty() {
            Err("no_usable_summary".to_string())
        } else {
            Err(issues.join("|"))
        }
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
        Ok(unique)
    }
}
