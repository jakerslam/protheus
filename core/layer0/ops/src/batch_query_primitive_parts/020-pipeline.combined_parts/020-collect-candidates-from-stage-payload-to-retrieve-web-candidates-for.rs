
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
    let mut retained_low_confidence = 0usize;
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
    if let Some(blocker_class) = payload_access_blocker_class(payload) {
        issues.push(format!("{stage}:{blocker_class}"));
        let provider_result = hidden_provider_result_artifact(stage, query, payload, 0, &issues);
        return (candidates, issues, provider_result);
    }
    if structured_results_enabled(policy) {
        for candidate in candidates_from_structured_search_payload(
            query,
            payload,
            structured_results_max_rows_per_stage(policy),
        ) {
            if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                candidates.push(candidate);
            } else if let Some(candidate) =
                retain_low_confidence_candidate(policy, &candidate, &mut retained_low_confidence)
            {
                issues.push(low_relevance_issue(
                    &candidate,
                    "candidate_low_relevance_retained_low_confidence",
                ));
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
        } else if let Some(candidate) =
            retain_low_confidence_candidate(policy, &candidate, &mut retained_low_confidence)
        {
            issues.push(low_relevance_issue(
                &candidate,
                "candidate_low_relevance_retained_low_confidence",
            ));
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
                } else if let Some(candidate) =
                    retain_low_confidence_candidate(policy, &candidate, &mut retained_low_confidence)
                {
                    issues.push(low_relevance_issue(
                        &candidate,
                        "candidate_low_relevance_retained_low_confidence",
                    ));
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
    let usable_candidate_count = candidates
        .iter()
        .filter(|candidate| !candidate_is_low_confidence_retained(candidate))
        .count();
    let should_fetch_links = page_extraction_enabled(policy)
        && page_extraction_max_links_per_stage(policy) > 0
        && page_extraction_max_total_fetches(policy) > fetched_links.len()
        && (usable_candidate_count < page_extraction_min_usable_items_before_skip(policy)
            || looks_like_low_signal_search_summary(&summary)
            || candidates
                .iter()
                .any(|candidate| candidate_needs_link_fetch(query, policy, candidate)));
    if should_fetch_links {
        let include_substantive_candidates =
            usable_candidate_count < page_extraction_min_usable_items_before_skip(policy)
                || looks_like_low_signal_search_summary(&summary);
        for link in links_for_page_extraction(
            query,
            policy,
            payload,
            &candidates,
            page_extraction_max_links_per_stage(policy),
            include_substantive_candidates,
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
                    mark_candidate_as_page_enriched(&mut candidate);
                    if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                        merge_or_push_page_enriched_candidate(query, policy, &mut candidates, candidate);
                    } else if let Some(candidate) = retain_low_confidence_candidate(
                        policy,
                        &candidate,
                        &mut retained_low_confidence,
                    ) {
                        issues.push(low_relevance_issue(
                            &candidate,
                            "fetch_candidate_low_relevance_retained_low_confidence",
                        ));
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
    let synthesis_candidate_count = candidates
        .iter()
        .filter(|candidate| !candidate_is_low_confidence_retained(candidate))
        .count();
    let provider_result =
        hidden_provider_result_artifact(stage, query, payload, synthesis_candidate_count, &issues);
    (candidates, issues, provider_result)
}

fn mark_candidate_as_page_enriched(candidate: &mut Candidate) {
    if !candidate
        .source_kind
        .to_ascii_lowercase()
        .contains("page_enriched")
    {
        candidate.source_kind = format!("{}_page_enriched", candidate.source_kind);
    }
    let permissions = candidate.permissions.clone().unwrap_or_default();
    candidate.permissions = Some(if permissions.is_empty() {
        "public_web;page_enriched".to_string()
    } else if permissions.contains("page_enriched") {
        permissions
    } else {
        format!("{permissions};page_enriched")
    });
}

fn page_enriched_candidate_value(query: &str, candidate: &Candidate) -> usize {
    let snippet = clean_text(&candidate.snippet, 2_400);
    let word_count = snippet.split_whitespace().count().min(220);
    let overlap = query_overlap_terms(query, candidate);
    let status_bonus = if (200..400).contains(&candidate.status_code) {
        12
    } else {
        0
    };
    let substance_bonus = if !looks_like_low_signal_search_summary(&snippet)
        && !looks_like_source_only_snippet(&snippet)
        && !looks_like_ack_only(&snippet)
    {
        24
    } else {
        0
    };
    word_count + overlap.saturating_mul(18) + status_bonus + substance_bonus
}

fn candidates_share_locator(a: &Candidate, b: &Candidate) -> bool {
    let left = clean_text(&a.locator, 2_200).to_ascii_lowercase();
    let right = clean_text(&b.locator, 2_200).to_ascii_lowercase();
    !left.is_empty() && left == right
}

fn should_replace_with_page_enriched_candidate(
    query: &str,
    policy: &Value,
    existing: &Candidate,
    enriched: &Candidate,
) -> bool {
    if !candidates_share_locator(existing, enriched) {
        return false;
    }
    if candidate_needs_link_fetch(query, policy, existing) {
        return true;
    }
    page_enriched_candidate_value(query, enriched)
        > page_enriched_candidate_value(query, existing).saturating_add(8)
}

fn merge_or_push_page_enriched_candidate(
    query: &str,
    policy: &Value,
    candidates: &mut Vec<Candidate>,
    enriched: Candidate,
) {
    if let Some(existing) = candidates.iter_mut().find(|candidate| {
        should_replace_with_page_enriched_candidate(query, policy, candidate, &enriched)
    }) {
        *existing = enriched;
        return;
    }
    candidates.push(enriched);
}

fn candidate_is_low_confidence_retained(candidate: &Candidate) -> bool {
    candidate
        .source_kind
        .to_ascii_lowercase()
        .contains("low_confidence")
        || candidate
            .permissions
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains("low_confidence_raw")
}

fn retain_low_confidence_candidate(
    policy: &Value,
    candidate: &Candidate,
    retained_count: &mut usize,
) -> Option<Candidate> {
    if !low_confidence_retention_enabled(policy) {
        return None;
    }
    let max_retained = policy
        .pointer("/batch_query/result_retention/max_low_confidence_items")
        .and_then(Value::as_u64)
        .unwrap_or(6)
        .clamp(1, 24) as usize;
    if *retained_count >= max_retained {
        return None;
    }
    let domain = candidate_domain_hint(candidate);
    if candidate.locator.is_empty()
        || is_search_engine_domain(&domain)
        || looks_like_low_signal_search_summary(&candidate.snippet)
        || contains_web_junk_marker(&candidate.snippet)
        || looks_like_ack_only(&candidate.snippet)
    {
        return None;
    }
    let mut candidate = candidate.clone();
    if !candidate.source_kind.contains("low_confidence") {
        candidate.source_kind = format!("{}_low_confidence_raw", candidate.source_kind);
    }
    let permissions = candidate.permissions.clone().unwrap_or_default();
    candidate.permissions = Some(if permissions.is_empty() {
        "public_web;low_confidence_raw".to_string()
    } else if permissions.contains("low_confidence_raw") {
        permissions
    } else {
        format!("{permissions};low_confidence_raw")
    });
    *retained_count += 1;
    Some(candidate)
}

fn has_usable_synthesis_candidate(candidates: &[Candidate]) -> bool {
    candidates
        .iter()
        .any(|candidate| !candidate_is_low_confidence_retained(candidate))
}

fn hidden_provider_result_quality(
    payload_ok: bool,
    candidate_count: usize,
    issues: &[String],
) -> &'static str {
    if candidate_count > 0 {
        return "usable";
    }
    if issues.iter().any(|issue| issue_is_access_or_throttle_failure(issue)) {
        return "blocked_or_throttled";
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
    let payload_ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false)
        || payload
            .get("provider_payload_rejected")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let error = hidden_provider_error(payload, !(payload_ok && candidate_count > 0));
    let links = payload_links_for_fallback(query, payload, 3);
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
    let provider_raw_count = hidden_provider_raw_count(payload, links.len(), &summary, &content_preview, &locator, &error);
    out.insert(
        "provider_raw_count".to_string(),
        json!(provider_raw_count),
    );
    out.insert(
        "provider_filtered_count".to_string(),
        json!(issues.len()),
    );
    if !issues.is_empty() {
        out.insert(
            "failure_reasons".to_string(),
            Value::Array(
                issues
                    .iter()
                    .map(|issue| Value::String(clean_text(issue, 180)))
                    .collect::<Vec<_>>(),
            ),
        );
    }
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

fn hidden_provider_error(payload: &Value, include_recovered_provider_errors: bool) -> String {
    let mut pointers = vec![
        "/error",
        "/retry/reason",
        "/policy_decision/reason",
        "/tool_execution_gate/reason",
        "/result/error",
    ];
    if include_recovered_provider_errors {
        pointers.extend(["/provider_errors/0/error", "/provider_errors/0/reason"]);
    }
    for pointer in pointers {
        let value = clean_text(payload.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 240);
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

fn hidden_provider_array_count(value: &Value, depth: usize) -> usize {
    if depth > 5 {
        return 0;
    }
    match value {
        Value::Array(rows) => rows.len(),
        Value::Object(map) => map
            .iter()
            .filter(|(key, _)| {
                matches!(
                    key.to_ascii_lowercase().as_str(),
                    "web"
                        | "news"
                        | "results"
                        | "items"
                        | "organic"
                        | "documents"
                        | "data"
                        | "links"
                        | "sources"
                )
            })
            .map(|(_, row)| hidden_provider_array_count(row, depth + 1))
            .sum(),
        _ => 0,
    }
}

fn hidden_provider_raw_count(
    payload: &Value,
    fallback_link_count: usize,
    summary: &str,
    content_preview: &str,
    locator: &str,
    error: &str,
) -> usize {
    let structured_count = hidden_provider_array_count(payload, 0);
    if structured_count > 0 {
        return structured_count;
    }
    if fallback_link_count > 0 {
        return fallback_link_count;
    }
    if !summary.is_empty() || !content_preview.is_empty() || !locator.is_empty() || !error.is_empty() {
        return 1;
    }
    0
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

    if !has_usable_synthesis_candidate(&candidates)
        && issues
            .iter()
            .any(|issue| skip_duckduckgo_fallback_for_error(issue))
    {
        return (Vec::new(), issues, provider_results);
    }

    if !has_usable_synthesis_candidate(&candidates) {
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

    if !has_usable_synthesis_candidate(&candidates) {
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
                    let mut retained_count = 0usize;
                    if let Some(candidate) =
                        retain_low_confidence_candidate(policy, &candidate, &mut retained_count)
                    {
                        duckduckgo_issues.push(
                            "duckduckgo_instant:candidate_low_relevance_retained_low_confidence"
                                .to_string(),
                        );
                        candidates.push(candidate);
                    } else {
                        duckduckgo_issues
                            .push("duckduckgo_instant:candidate_low_relevance".to_string());
                    }
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

    if !has_usable_synthesis_candidate(&candidates) {
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
            if has_usable_synthesis_candidate(&provider_candidates) {
                candidates.append(&mut provider_candidates);
                break;
            } else if !provider_candidates.is_empty() {
                candidates.append(&mut provider_candidates);
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
