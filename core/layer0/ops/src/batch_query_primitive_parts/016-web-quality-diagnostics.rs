fn current_web_intent(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    if lowered.contains(&current_year()) {
        return true;
    }
    [
        "latest",
        "current",
        "today",
        "this week",
        "this month",
        "as of",
        "recent",
        "newest",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn web_tool_quality_version() -> &'static str {
    "web_tool_quality_v2"
}

fn current_year() -> String {
    crate::now_iso().chars().take(4).collect::<String>()
}

fn source_trust_adjustment(candidate: &Candidate) -> f64 {
    let domain = candidate_domain_hint(candidate).to_ascii_lowercase();
    let locator = clean_text(&candidate.locator, 2_200).to_ascii_lowercase();
    let combined = clean_text(
        &format!("{} {} {}", candidate.title, candidate.snippet, candidate.locator),
        2_400,
    )
    .to_ascii_lowercase();
    if domain == "source" || is_search_engine_domain(&domain) {
        return -0.18;
    }
    if contains_web_junk_marker(&combined) || looks_like_competitive_programming_dump(&combined) {
        return -0.5;
    }
    if domain.contains("reddit.com")
        || domain.contains("quora.com")
        || domain.contains("zhihu.com")
        || domain.contains("medium.com")
        || domain.contains("dev.to")
        || domain.contains("hashnode.dev")
    {
        return -0.14;
    }
    if domain.contains("docs.")
        || locator.contains("/docs")
        || locator.contains("/documentation")
        || domain.contains("developer.")
        || domain.contains("github.com")
        || domain.contains("gitlab.com")
        || domain.contains("arxiv.org")
        || domain.contains("openai.com")
        || domain.contains("microsoft.com")
        || domain.contains("langchain.com")
        || domain.contains("huggingface.co")
        || domain.contains("crewai.com")
    {
        return 0.16;
    }
    if domain.contains("reuters.com")
        || domain.contains("apnews.com")
        || domain.contains("bloomberg.com")
        || domain.contains("wsj.com")
        || domain.contains("ft.com")
        || domain.contains("techcrunch.com")
        || domain.contains("theverge.com")
    {
        return 0.1;
    }
    0.0
}

fn recency_adjustment(query: &str, candidate: &Candidate) -> f64 {
    if !current_web_intent(query) {
        return 0.0;
    }
    let year = current_year();
    let haystack = clean_text(
        &format!(
            "{} {} {} {:?}",
            candidate.title, candidate.snippet, candidate.locator, candidate.timestamp
        ),
        2_400,
    )
    .to_ascii_lowercase();
    if haystack.contains(&year) || haystack.contains("today") || haystack.contains("this week") {
        0.08
    } else {
        -0.08
    }
}

fn candidate_quality_flags(query: &str, candidate: &Candidate, score: f64) -> Vec<String> {
    let mut flags = Vec::<String>::new();
    let snippet = clean_text(&candidate.snippet, 1_600);
    let trust = source_trust_adjustment(candidate);
    if trust >= 0.15 {
        flags.push("trusted_source".to_string());
    } else if trust <= -0.14 {
        flags.push("low_trust_source".to_string());
    }
    if current_web_intent(query) && recency_adjustment(query, candidate) < 0.0 {
        flags.push("freshness_unproven".to_string());
    }
    if looks_like_metric_rich_text(&snippet) {
        flags.push("metric_rich".to_string());
    }
    if query_overlap_terms(query, candidate) < 2 {
        flags.push("thin_query_overlap".to_string());
    }
    if contains_web_junk_marker(&snippet) || contains_web_junk_marker(&candidate.title) {
        flags.push("junk_marker".to_string());
    }
    if score < 0.35 {
        flags.push("low_score".to_string());
    }
    flags.sort();
    flags.dedup();
    flags
}

fn sorted_ranked_candidates(mut ranked: Vec<(Candidate, f64)>) -> Vec<(Candidate, f64)> {
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.title.cmp(&b.0.title))
    });
    ranked
}

fn select_diverse_ranked_candidates(
    ranked: Vec<(Candidate, f64)>,
    max_evidence: usize,
) -> Vec<(Candidate, f64)> {
    let limit = max_evidence.max(1);
    let sorted = sorted_ranked_candidates(ranked);
    let mut selected = Vec::<(Candidate, f64)>::new();
    let mut seen_domains = HashSet::<String>::new();
    for row in &sorted {
        let domain = candidate_domain_hint(&row.0).to_ascii_lowercase();
        if !domain.is_empty() && domain != "source" && !seen_domains.insert(domain) {
            continue;
        }
        selected.push(row.clone());
        if selected.len() >= limit {
            return selected;
        }
    }
    for row in sorted {
        if selected.iter().any(|(candidate, _)| {
            candidate.locator == row.0.locator && candidate.excerpt_hash == row.0.excerpt_hash
        }) {
            continue;
        }
        selected.push(row);
        if selected.len() >= limit {
            break;
        }
    }
    selected
}

fn fallback_link_score(query: &str, link: &str) -> f64 {
    let cleaned = clean_text(link, 2_200);
    let lowered = cleaned.to_ascii_lowercase();
    let domain = extract_domains_from_text(&cleaned, 1)
        .into_iter()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if domain.is_empty() || is_search_engine_domain(&domain) || contains_web_junk_marker(&lowered)
    {
        return -1.0;
    }
    let mut score: f64 = 0.0;
    let candidate = Candidate {
        source_kind: "web".to_string(),
        title: format!("Web result from {domain}"),
        locator: cleaned.clone(),
        snippet: cleaned.clone(),
        excerpt_hash: sha256_hex(&cleaned),
        timestamp: None,
        permissions: Some("public_web".to_string()),
        status_code: 200,
    };
    score += source_trust_adjustment(&candidate);
    if current_web_intent(query) {
        score += recency_adjustment(query, &candidate);
    }
    let query_tokens = tokenize_relevance(query, 40);
    let link_tokens = tokenize_relevance(&cleaned, 80);
    if !query_tokens.is_empty() {
        let overlap = query_tokens.intersection(&link_tokens).count() as f64;
        score += 0.4 * (overlap / query_tokens.len() as f64);
    }
    if lowered.contains("/docs") || lowered.contains("/blog") || lowered.contains("/news") {
        score += 0.04;
    }
    if lowered.contains("login") || lowered.contains("signup") || lowered.contains("account") {
        score -= 0.2;
    }
    score
}

fn ranked_payload_links_for_fallback(query: &str, payload: &Value, max_links: usize) -> Vec<String> {
    let mut ranked = non_search_engine_links(payload, max_links.saturating_mul(4).max(max_links))
        .into_iter()
        .map(|link| {
            let score = fallback_link_score(query, &link);
            (link, score)
        })
        .filter(|(_, score)| *score > -1.0)
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    ranked
        .into_iter()
        .take(max_links.max(1))
        .map(|(link, _)| link)
        .collect::<Vec<_>>()
}

fn issue_quality_flags(partial_failures: &[String]) -> Vec<String> {
    let mut flags = Vec::<String>::new();
    for failure in partial_failures {
        let lowered = clean_text(failure, 320).to_ascii_lowercase();
        if lowered.contains("anti_bot_challenge") {
            flags.push("anti_bot_filtered".to_string());
        }
        if lowered.contains("junk_page") {
            flags.push("junk_filtered".to_string());
        }
        if lowered.contains("candidate_low_relevance")
            || lowered.contains("fetch_candidate_low_relevance")
        {
            flags.push("low_relevance_filtered".to_string());
        }
        if lowered.contains("query_timeout") {
            flags.push("provider_timeout".to_string());
        }
        if lowered.contains("tool_surface_degraded")
            || lowered.contains("provider readiness mismatch")
        {
            flags.push("provider_degraded".to_string());
        }
        if lowered.contains("no_usable_summary") || lowered.contains("fixture_missing") {
            flags.push("low_signal".to_string());
        }
        if lowered.contains("query_result_mismatch") {
            flags.push("query_result_mismatch".to_string());
        }
    }
    flags.sort();
    flags.dedup();
    flags
}

fn potential_source_conflict(actionable_ranked: &[(Candidate, f64)]) -> bool {
    let mut positive_claim = false;
    let mut limiting_claim = false;
    for (candidate, _) in actionable_ranked {
        let text = clean_text(
            &format!("{} {}", candidate.title, candidate.snippet),
            1_600,
        )
        .to_ascii_lowercase();
        positive_claim |= [
            "best",
            "better",
            "outperform",
            "leads",
            "faster",
            "stronger",
            "higher",
        ]
        .iter()
        .any(|marker| text.contains(marker));
        limiting_claim |= [
            "worse",
            "weaker",
            "slower",
            "lower",
            "limitation",
            "trade-off",
            "tradeoff",
            "lacks",
        ]
        .iter()
        .any(|marker| text.contains(marker));
    }
    positive_claim && limiting_claim
}

fn web_tool_quality_report(
    query: &str,
    status: &str,
    candidate_count: usize,
    evidence_count: usize,
    partial_failures: &[String],
    hard_partial_failures: &[String],
    actionable_ranked: &[(Candidate, f64)],
) -> Value {
    let mut flags = issue_quality_flags(partial_failures);
    if status == "no_results" || evidence_count == 0 {
        flags.push("insufficient_evidence".to_string());
    } else if status == "partial" {
        flags.push("partial_results".to_string());
    } else if evidence_count >= 2 && hard_partial_failures.is_empty() {
        flags.push("high_confidence".to_string());
    }
    let mut domains = HashSet::<String>::new();
    for (candidate, _) in actionable_ranked {
        let domain = candidate_domain_hint(candidate).to_ascii_lowercase();
        if !domain.is_empty() && domain != "source" {
            domains.insert(domain);
        }
    }
    if evidence_count > 1 && domains.len() < evidence_count {
        flags.push("source_diversity_limited".to_string());
    }
    if is_benchmark_or_comparison_intent(query) && evidence_count < 2 {
        flags.push("comparison_evidence_insufficient".to_string());
    }
    if is_benchmark_or_comparison_intent(query) && evidence_count > 1 {
        flags.push("comparative_synthesis_required".to_string());
    }
    if potential_source_conflict(actionable_ranked) {
        flags.push("potential_source_conflict".to_string());
    }
    flags.sort();
    flags.dedup();
    let candidate_quality = actionable_ranked
        .iter()
        .take(6)
        .map(|(candidate, score)| {
            json!({
                "title": clean_text(&candidate.title, 160),
                "domain": candidate_domain_hint(candidate),
                "locator": clean_text(&candidate.locator, 240),
                "snippet_preview": trim_words(&clean_text(&candidate.snippet, 600), 32),
                "score": (*score * 100.0).round() / 100.0,
                "flags": candidate_quality_flags(query, candidate, *score)
            })
        })
        .collect::<Vec<_>>();
    let weak_single_source = evidence_count == 1
        && actionable_ranked.first().is_some_and(|(candidate, score)| {
            let candidate_flags = candidate_quality_flags(query, candidate, *score);
            *score < 0.65
                && candidate_flags.iter().any(|flag| {
                    matches!(
                        flag.as_str(),
                        "thin_query_overlap"
                            | "low_score"
                            | "low_trust_source"
                            | "freshness_unproven"
                            | "junk_marker"
                    )
                })
        });
    if weak_single_source {
        flags.push("weak_single_source".to_string());
    }
    flags.sort();
    flags.dedup();
    let retry_reason = if flags.iter().any(|flag| flag == "anti_bot_filtered") {
        "anti_bot_filtered"
    } else if flags.iter().any(|flag| flag == "provider_degraded") {
        "provider_degraded"
    } else if flags.iter().any(|flag| flag == "insufficient_evidence") {
        "insufficient_evidence"
    } else if flags
        .iter()
        .any(|flag| flag == "comparison_evidence_insufficient")
    {
        "comparison_evidence_insufficient"
    } else if flags.iter().any(|flag| flag == "weak_single_source") {
        "weak_single_source"
    } else if flags.iter().any(|flag| flag == "low_signal") {
        "low_signal"
    } else {
        "none"
    };
    json!({
        "version": web_tool_quality_version(),
        "status": status,
        "flags": flags,
        "candidate_count": candidate_count,
        "evidence_count": evidence_count,
        "freshness": {
            "current_intent": current_web_intent(query),
            "current_year": current_year()
        },
        "candidate_quality": candidate_quality,
        "synthesis_contract": {
            "authority": "agent_authored",
            "must_not_claim_more_than_evidence": true,
            "use_candidate_snippet_previews_only_as_grounding": true
        },
        "retry": {
            "recommended": retry_reason != "none",
            "reason": retry_reason,
            "input_contract": {
                "authority": "agent_submitted",
                "input_kind": "query_or_query_pack",
                "required_fields": ["query"],
                "optional_fields": ["queries", "source_url"],
                "hidden_query_expansion": false
            },
            "query_strategy_hints": [
                "preserve the user's original research objective",
                "split broad requests into entity-specific or aspect-specific searches",
                "keep query packs concise; choose the strongest 4-6 query angles instead of an exhaustive entity-by-source-class cross product",
                "for open-ended discovery, start with category plus source-class or candidate-discovery searches before candidate-specific searches",
                "target primary, official, source-backed, or directly citable pages when possible",
                "use partial snippets, failure reasons, and off-topic signals to remove weak terms",
                "for current or recent research, prefer current/recent source-class searches such as changelog, release notes, repository, publication, announcement, security advisory, or methodology over broad stale year ranges",
                "for one named entity, keep the exact entity name in every retry query and vary source class or decision aspect rather than replacing it with loose synonyms",
                "for fused or camelcase entity names, include the exact token and a readable spaced or alias variant alongside source-class terms",
                "for sparse benchmark research, search benchmark results, leaderboards or evaluation suites, methodology and reproducibility, and practical evaluation criteria",
                "prefer another agent-submitted query pack before asking the user to narrow"
            ],
            "next_action": if retry_reason == "none" {
                "synthesize_from_evidence"
            } else {
                "agent_refine_query_pack_and_retry_if_budget_remains"
            }
        }
    })
}

fn cached_web_tool_quality_report(
    query: &str,
    status: &str,
    partial_failure_details: &Value,
    evidence_refs: &Value,
) -> Value {
    let failures = partial_failure_details
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let evidence_count = evidence_refs.as_array().map(Vec::len).unwrap_or(0);
    let mut report =
        web_tool_quality_report(query, status, 0, evidence_count, &failures, &failures, &[]);
    let weak_cached_single_source = evidence_count == 1
        && evidence_refs
            .as_array()
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("score"))
            .and_then(Value::as_f64)
            .map(|score| score < 0.65)
            .unwrap_or(false);
    if weak_cached_single_source {
        if let Some(flags) = report.get_mut("flags").and_then(Value::as_array_mut) {
            if !flags.iter().any(|flag| flag == "weak_single_source") {
                flags.push(json!("weak_single_source"));
            }
        }
        report["retry"]["recommended"] = json!(true);
        report["retry"]["reason"] = json!("weak_single_source");
        report["retry"]["next_action"] =
            json!("agent_refine_query_pack_and_retry_if_budget_remains");
    }
    report
}
