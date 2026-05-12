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
    "web_tool_quality_v3"
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

#[derive(Clone, Debug)]
struct ResearchFacet {
    id: String,
    requested_text: String,
    terms: HashSet<String>,
    distinctive_terms: HashSet<String>,
}

fn research_facet_signature(terms: &HashSet<String>) -> String {
    let mut sorted = terms.iter().cloned().collect::<Vec<_>>();
    sorted.sort();
    sorted.join("|")
}

fn research_facet_from_text(
    text: &str,
    index: usize,
    min_terms: usize,
) -> Option<ResearchFacet> {
    let requested_text = clean_text(text, 600);
    if requested_text.is_empty() {
        return None;
    }
    let terms = tokenize_relevance(&requested_text, 24);
    if terms.len() < min_terms.max(1) {
        return None;
    }
    Some(ResearchFacet {
        id: format!("facet_{:02}", index + 1),
        requested_text,
        terms,
        distinctive_terms: HashSet::new(),
    })
}

fn assign_distinctive_facet_terms(facets: &mut [ResearchFacet]) {
    if facets.len() <= 1 {
        for facet in facets {
            facet.distinctive_terms = facet.terms.clone();
        }
        return;
    }
    let mut counts = HashMap::<String, usize>::new();
    for facet in facets.iter() {
        for term in &facet.terms {
            *counts.entry(term.clone()).or_insert(0) += 1;
        }
    }
    let facet_count = facets.len();
    for facet in facets.iter_mut() {
        facet.distinctive_terms = facet
            .terms
            .iter()
            .filter(|term| counts.get(*term).copied().unwrap_or(0) < facet_count)
            .cloned()
            .collect::<HashSet<_>>();
        if facet.distinctive_terms.is_empty() {
            facet.distinctive_terms = facet.terms.clone();
        }
    }
}

fn inferred_facet_texts_from_query(query: &str) -> Vec<String> {
    let cleaned = clean_text(query, 900);
    if cleaned.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::<String>::new();
    for piece in cleaned.split(|ch| matches!(ch, ',' | ';' | ':' | '?' | '.')) {
        let piece = clean_text(piece, 240);
        if piece.split_whitespace().count() >= 3 {
            out.push(piece);
        }
    }
    for piece in cleaned.split(" and ") {
        let piece = clean_text(piece, 240);
        if piece.split_whitespace().count() >= 3 {
            out.push(piece);
        }
    }
    out
}

fn infer_research_facets(
    query: &str,
    query_plan: &[String],
    policy: &Value,
    budget: ApertureBudget,
) -> Vec<ResearchFacet> {
    if !facet_aware_evidence_enabled(policy) {
        return Vec::new();
    }
    let max_facets = facet_aware_max_facets(policy, budget);
    let min_terms = facet_aware_min_terms(policy);
    let mut texts = Vec::<String>::new();
    let base = clean_text(query, 600);
    if !base.is_empty() {
        texts.push(base.clone());
    }
    for item in query_plan {
        let item = clean_text(item, 600);
        if !item.is_empty() {
            texts.push(item);
        }
    }
    if texts.len() <= 1 {
        texts.extend(inferred_facet_texts_from_query(&base));
    }

    let mut seen = HashSet::<String>::new();
    let mut facets = Vec::<ResearchFacet>::new();
    for text in texts {
        if let Some(mut facet) = research_facet_from_text(&text, facets.len(), min_terms) {
            let signature = research_facet_signature(&facet.terms);
            if !seen.insert(signature) {
                continue;
            }
            facet.id = format!("facet_{:02}", facets.len() + 1);
            facets.push(facet);
        }
        if facets.len() >= max_facets {
            break;
        }
    }
    assign_distinctive_facet_terms(&mut facets);
    facets
}

fn candidate_facet_overlap(facet: &ResearchFacet, candidate: &Candidate) -> usize {
    let haystack = tokenize_relevance(
        &format!("{} {} {}", candidate.title, candidate.snippet, candidate.locator),
        160,
    );
    facet
        .terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .count()
}

fn candidate_matches_facet(
    facet: &ResearchFacet,
    candidate: &Candidate,
    min_terms: usize,
) -> bool {
    let overlap = candidate_facet_overlap(facet, candidate);
    if overlap == 0 {
        return false;
    }
    if !facet.distinctive_terms.is_empty() {
        let haystack = tokenize_relevance(
            &format!("{} {} {}", candidate.title, candidate.snippet, candidate.locator),
            160,
        );
        if !facet
            .distinctive_terms
            .iter()
            .any(|term| haystack.contains(term.as_str()))
        {
            return false;
        }
    }
    let required = min_terms.min(facet.terms.len()).max(1);
    overlap >= required || (facet.terms.len() <= 2 && overlap >= 1)
}

fn coverage_aware_score(
    base_query: &str,
    facets: &[ResearchFacet],
    candidate: &Candidate,
    min_terms: usize,
) -> f64 {
    let mut best = rerank_score(base_query, candidate);
    for facet in facets {
        let overlap = candidate_facet_overlap(facet, candidate);
        if overlap == 0 {
            continue;
        }
        let mut score = rerank_score(&facet.requested_text, candidate);
        if overlap >= min_terms.min(facet.terms.len()).max(1) {
            score += 0.08;
        } else {
            score += 0.03;
        }
        best = best.max(score);
    }
    best.clamp(0.0, 1.0)
}

fn candidate_identity_key(candidate: &Candidate) -> String {
    format!(
        "{}|{}|{}",
        candidate.locator.to_ascii_lowercase(),
        candidate.title.to_ascii_lowercase(),
        candidate.excerpt_hash
    )
}

fn select_facet_covered_ranked_candidates(
    ranked: Vec<(Candidate, f64)>,
    facets: &[ResearchFacet],
    max_evidence: usize,
    min_terms: usize,
) -> Vec<(Candidate, f64)> {
    if facets.is_empty() {
        return select_diverse_ranked_candidates(ranked, max_evidence);
    }
    let limit = max_evidence.max(1);
    let sorted = sorted_ranked_candidates(ranked);
    let mut selected = Vec::<(Candidate, f64)>::new();
    let mut selected_keys = HashSet::<String>::new();
    for facet in facets {
        if let Some(row) = sorted.iter().find(|(candidate, _)| {
            !selected_keys.contains(&candidate_identity_key(candidate))
                && candidate_matches_facet(facet, candidate, min_terms)
        }) {
            selected_keys.insert(candidate_identity_key(&row.0));
            selected.push(row.clone());
            if selected.len() >= limit {
                return selected;
            }
        }
    }
    let remaining = sorted
        .into_iter()
        .filter(|(candidate, _)| !selected_keys.contains(&candidate_identity_key(candidate)))
        .collect::<Vec<_>>();
    for row in select_diverse_ranked_candidates(remaining, limit.saturating_sub(selected.len())) {
        selected_keys.insert(candidate_identity_key(&row.0));
        selected.push(row);
        if selected.len() >= limit {
            break;
        }
    }
    selected
}

fn candidate_coverage_facets(
    facets: &[ResearchFacet],
    candidate: &Candidate,
    min_terms: usize,
) -> Vec<String> {
    facets
        .iter()
        .filter(|facet| candidate_matches_facet(facet, candidate, min_terms))
        .map(|facet| facet.id.clone())
        .collect::<Vec<_>>()
}

fn evidence_coverage_from_ranked_candidates(
    facets: &[ResearchFacet],
    evidence_ranked: &[(Candidate, f64)],
    min_terms: usize,
) -> Value {
    if facets.is_empty() {
        return json!([]);
    }
    Value::Array(
        facets
            .iter()
            .map(|facet| {
                let matching = evidence_ranked
                    .iter()
                    .filter(|(candidate, _)| candidate_matches_facet(facet, candidate, min_terms))
                    .collect::<Vec<_>>();
                let usable_count = matching
                    .iter()
                    .filter(|(candidate, _)| !candidate_is_low_confidence_retained(candidate))
                    .count();
                let low_confidence_count = matching.len().saturating_sub(usable_count);
                let mut domains = matching
                    .iter()
                    .map(|(candidate, _)| candidate_domain_hint(candidate).to_ascii_lowercase())
                    .filter(|domain| !domain.is_empty() && domain != "source")
                    .collect::<Vec<_>>();
                domains.sort();
                domains.dedup();
                let status = if usable_count > 0 {
                    "covered"
                } else if low_confidence_count > 0 {
                    "weak"
                } else {
                    "missing"
                };
                json!({
                    "facet_id": facet.id,
                    "requested_text": facet.requested_text,
                    "status": status,
                    "evidence_count": matching.len(),
                    "usable_evidence_count": usable_count,
                    "low_confidence_raw_count": low_confidence_count,
                    "source_domain_count": domains.len(),
                    "source_domains": domains
                })
            })
            .collect::<Vec<_>>(),
    )
}

fn usable_ranked_candidates_for_coverage(
    query: &str,
    facets: &[ResearchFacet],
    candidates: &[Candidate],
    min_terms: usize,
) -> Vec<(Candidate, f64)> {
    let benchmark_intent = is_benchmark_or_comparison_intent(query);
    let min_score = minimum_synthesis_score(benchmark_intent);
    candidates
        .iter()
        .filter(|candidate| !candidate_is_low_confidence_retained(candidate))
        .map(|candidate| {
            let score = if facets.is_empty() {
                rerank_score(query, candidate)
            } else {
                coverage_aware_score(query, facets, candidate, min_terms)
            };
            (candidate.clone(), score)
        })
        .filter(|(candidate, score)| {
            *score >= min_score
                && candidate_passes_relevance_gate(query, candidate, benchmark_intent)
                && candidate_is_substantive(query, candidate, benchmark_intent)
        })
        .collect::<Vec<_>>()
}

fn missing_research_facets_for_coverage(
    query: &str,
    facets: &[ResearchFacet],
    candidates: &[Candidate],
    min_terms: usize,
) -> Vec<ResearchFacet> {
    if facets.is_empty() {
        return Vec::new();
    }
    let usable = usable_ranked_candidates_for_coverage(query, facets, candidates, min_terms);
    facets
        .iter()
        .filter(|facet| {
            !usable
                .iter()
                .any(|(candidate, _)| candidate_matches_facet(facet, candidate, min_terms))
        })
        .cloned()
        .collect::<Vec<_>>()
}

fn coverage_gap_recovery_needed(
    policy: &Value,
    query: &str,
    facets: &[ResearchFacet],
    candidates: &[Candidate],
    budget: ApertureBudget,
) -> bool {
    if !coverage_gap_recovery_enabled(policy) || facets.is_empty() || candidates.is_empty() {
        return false;
    }
    let min_terms = facet_aware_min_terms(policy);
    let usable = usable_ranked_candidates_for_coverage(query, facets, candidates, min_terms);
    let min_usable = coverage_gap_recovery_min_usable_evidence(policy, budget);
    if usable.len() < min_usable {
        return true;
    }
    let covered = facets
        .iter()
        .filter(|facet| {
            usable
                .iter()
                .any(|(candidate, _)| candidate_matches_facet(facet, candidate, min_terms))
        })
        .count();
    covered < coverage_gap_recovery_min_covered_facets(policy, facets.len(), budget)
}

fn expand_coverage_gap_recovery_template(template: &str, query: &str, facet: &str) -> Option<String> {
    let expanded = template
        .replace("{query}", query)
        .replace("{facet}", facet);
    let cleaned = clean_text(&expanded, 600);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn coverage_gap_recovery_queries(
    policy: &Value,
    query: &str,
    existing_queries: &[String],
    facets: &[ResearchFacet],
    candidates: &[Candidate],
    budget: ApertureBudget,
) -> Vec<String> {
    if !coverage_gap_recovery_needed(policy, query, facets, candidates, budget) {
        return Vec::new();
    }
    let min_terms = facet_aware_min_terms(policy);
    let max_queries = coverage_gap_recovery_max_queries(policy, budget);
    let missing = missing_research_facets_for_coverage(query, facets, candidates, min_terms);
    if missing.is_empty() {
        return Vec::new();
    }
    let mut seen = existing_queries
        .iter()
        .map(|row| clean_text(row, 600).to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut out = Vec::<String>::new();
    for facet in missing {
        for template in coverage_gap_recovery_templates(policy) {
            let template = if template.contains("{facet}") {
                template
            } else {
                template.replace("{query}", "{facet}")
            };
            if let Some(candidate) =
                expand_coverage_gap_recovery_template(&template, query, &facet.requested_text)
            {
                if seen.insert(candidate.to_ascii_lowercase()) {
                    out.push(candidate);
                }
            }
            if out.len() >= max_queries {
                return out;
            }
        }
    }
    out
}

fn evidence_pack_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/evidence_pack/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn evidence_pack_max_items(policy: &Value, max_evidence: usize) -> usize {
    if !evidence_pack_enabled(policy) {
        return 0;
    }
    policy
        .pointer("/batch_query/evidence_pack/max_items")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(max_evidence)
        .clamp(1, max_evidence.max(1))
}

fn evidence_pack_max_snippet_words(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/evidence_pack/max_snippet_words")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(72)
        .clamp(16, 160)
}

fn policy_string_list(policy: &Value, pointer: &str) -> Vec<String> {
    policy
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 160).to_ascii_lowercase())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn evidence_pack_source_class(policy: &Value, candidate: &Candidate) -> String {
    let source_kind = clean_text(&candidate.source_kind, 80).to_ascii_lowercase();
    let locator = clean_text(&candidate.locator, 2_200).to_ascii_lowercase();
    let domain = candidate_domain_hint(candidate).to_ascii_lowercase();
    if let Some(rules) = policy
        .pointer("/batch_query/evidence_pack/source_class_rules")
        .and_then(Value::as_array)
    {
        for rule in rules {
            let class = clean_text(
                rule.get("class").and_then(Value::as_str).unwrap_or(""),
                80,
            );
            if class.is_empty() {
                continue;
            }
            let source_kind_matches = policy_string_list(rule, "/source_kinds")
                .iter()
                .any(|value| value == &source_kind);
            let host_suffix_matches = policy_string_list(rule, "/host_suffixes")
                .iter()
                .any(|suffix| domain.ends_with(suffix));
            let host_contains_matches = policy_string_list(rule, "/host_contains")
                .iter()
                .any(|needle| domain.contains(needle));
            let path_contains_matches = policy_string_list(rule, "/path_contains")
                .iter()
                .any(|needle| locator.contains(needle));
            if source_kind_matches
                || host_suffix_matches
                || host_contains_matches
                || path_contains_matches
            {
                return class;
            }
        }
    }
    if source_kind.is_empty() {
        "general_web".to_string()
    } else {
        source_kind
    }
}

fn evidence_pack_freshness_status(query: &str, candidate: &Candidate) -> String {
    if !current_web_intent(query) {
        return "not_time_sensitive".to_string();
    }
    if recency_adjustment(query, candidate) >= 0.0 {
        "current_signal_present".to_string()
    } else {
        "freshness_unproven".to_string()
    }
}

fn evidence_pack_claim_hints(query: &str, snippet: &str, limit: usize) -> Vec<String> {
    let query_terms = tokenize_relevance(query, 40);
    let mut out = Vec::<String>::new();
    for segment in snippet.split(|ch| matches!(ch, '.' | ';' | '\n' | '\r')) {
        let cleaned = clean_text(segment, 420);
        if cleaned.split_whitespace().count() < 6 {
            continue;
        }
        let segment_terms = tokenize_relevance(&cleaned, 80);
        let has_query_overlap = query_terms.is_empty()
            || query_terms
                .iter()
                .any(|term| segment_terms.contains(term.as_str()));
        if !has_query_overlap && out.len() + 1 < limit {
            continue;
        }
        out.push(trim_words(&cleaned, 34));
        if out.len() >= limit.max(1) {
            break;
        }
    }
    out
}

fn evidence_pack_term_hints(query: &str, candidate: &Candidate, limit: usize) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<String>::new();
    for term in tokenize_relevance(
        &format!("{} {} {}", candidate.title, candidate.snippet, candidate.locator),
        160,
    ) {
        if term.len() < 4 {
            continue;
        }
        if query.to_ascii_lowercase().contains(term.as_str()) {
            continue;
        }
        if seen.insert(term.clone()) {
            out.push(term);
        }
        if out.len() >= limit.max(1) {
            break;
        }
    }
    out
}

fn evidence_pack_from_ranked_candidates(
    policy: &Value,
    query: &str,
    facets: &[ResearchFacet],
    min_terms: usize,
    actionable_ranked: &[(Candidate, f64)],
    max_evidence: usize,
) -> Value {
    let max_items = evidence_pack_max_items(policy, max_evidence);
    if max_items == 0 {
        return json!([]);
    }
    let max_snippet_words = evidence_pack_max_snippet_words(policy);
    Value::Array(
        actionable_ranked
            .iter()
            .take(max_items)
            .map(|(candidate, score)| {
                let domain = candidate_domain_hint(candidate);
                let quality_flags = if candidate_is_low_confidence_retained(candidate) {
                    vec!["low_confidence_raw".to_string()]
                } else {
                    candidate_quality_flags(query, candidate, *score)
                };
                let coverage_facets = candidate_coverage_facets(facets, candidate, min_terms);
                let confidence = if candidate_is_low_confidence_retained(candidate) {
                    "low_confidence_raw"
                } else {
                    "usable"
                };
                json!({
                    "pack_version": "evidence_pack_v1",
                    "source_kind": candidate.source_kind.clone(),
                    "source_class": evidence_pack_source_class(policy, candidate),
                    "title": clean_text(&candidate.title, 240),
                    "locator": clean_text(&candidate.locator, 2_200),
                    "source_scope": domain,
                    "source_domain": domain,
                    "snippet": trim_words(&clean_text(&candidate.snippet, 1_800), max_snippet_words),
                    "claim_hints": evidence_pack_claim_hints(query, &candidate.snippet, 2),
                    "term_hints": evidence_pack_term_hints(query, candidate, 8),
                    "excerpt_hash": candidate.excerpt_hash.clone(),
                    "score": (*score * 100.0).round() / 100.0,
                    "score_components": {
                        "relevance": (*score * 100.0).round() / 100.0,
                        "source_trust_delta": (source_trust_adjustment(candidate) * 100.0).round() / 100.0,
                        "freshness_delta": (recency_adjustment(query, candidate) * 100.0).round() / 100.0
                    },
                    "confidence": confidence,
                    "quality_flags": quality_flags,
                    "coverage_facets": coverage_facets,
                    "freshness": {
                        "status": evidence_pack_freshness_status(query, candidate),
                        "current_intent": current_web_intent(query)
                    },
                    "timestamp": candidate.timestamp.clone(),
                    "permissions": candidate.permissions.clone(),
                    "content_authority": "retrieved_public_web",
                    "visibility": "synthesis_context"
                })
            })
            .collect::<Vec<_>>(),
    )
}

fn value_array_len(value: &Value) -> usize {
    value.as_array().map(Vec::len).unwrap_or(0)
}

fn policy_usize_at(policy: &Value, pointer: &str, default_value: usize, min: usize, max: usize) -> usize {
    policy
        .pointer(pointer)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(default_value)
        .clamp(min, max)
}

fn source_class_diversity_min(policy: &Value, query: &str) -> usize {
    let default_value = if current_web_intent(query) { 2 } else { 1 };
    policy_usize_at(
        policy,
        "/batch_query/source_diversification/min_source_classes",
        default_value,
        1,
        8,
    )
}

fn evidence_pack_quality_report(policy: &Value, evidence_pack: &Value, evidence_coverage: &Value) -> Value {
    let min_usable = policy_usize_at(
        policy,
        "/batch_query/evidence_pack_quality/min_usable_items",
        2,
        1,
        12,
    );
    let min_domains = policy_usize_at(
        policy,
        "/batch_query/evidence_pack_quality/min_source_domains",
        2,
        1,
        12,
    );
    let mut usable_count = 0usize;
    let mut low_confidence_count = 0usize;
    let mut domains = HashSet::<String>::new();
    let mut source_classes = HashSet::<String>::new();
    if let Some(rows) = evidence_pack.as_array() {
        for row in rows {
            let confidence = clean_text(
                row.get("confidence").and_then(Value::as_str).unwrap_or(""),
                80,
            );
            if confidence == "low_confidence_raw" {
                low_confidence_count += 1;
            } else {
                usable_count += 1;
            }
            let domain = clean_text(
                row.get("source_domain")
                    .or_else(|| row.get("source_scope"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                180,
            )
            .to_ascii_lowercase();
            if !domain.is_empty() && domain != "source" {
                domains.insert(domain);
            }
            let source_class = clean_text(
                row.get("source_class").and_then(Value::as_str).unwrap_or(""),
                120,
            )
            .to_ascii_lowercase();
            if !source_class.is_empty() {
                source_classes.insert(source_class);
            }
        }
    }
    let mut missing_facets = 0usize;
    let mut weak_facets = 0usize;
    if let Some(rows) = evidence_coverage.as_array() {
        for row in rows {
            match row.get("status").and_then(Value::as_str).unwrap_or("") {
                "missing" => missing_facets += 1,
                "weak" => weak_facets += 1,
                _ => {}
            }
        }
    }
    let item_count = value_array_len(evidence_pack);
    let status = if item_count == 0 {
        "absent"
    } else if usable_count == 0 {
        "low_confidence_only"
    } else if usable_count < min_usable || domains.len() < min_domains || missing_facets > 0 {
        "thin"
    } else {
        "usable"
    };
    json!({
        "version": "evidence_pack_quality_v1",
        "status": status,
        "item_count": item_count,
        "usable_count": usable_count,
        "low_confidence_count": low_confidence_count,
        "source_domain_count": domains.len(),
        "source_class_count": source_classes.len(),
        "missing_facet_count": missing_facets,
        "weak_facet_count": weak_facets,
        "thresholds": {
            "min_usable_items": min_usable,
            "min_source_domains": min_domains
        },
        "synthesis_boundary": "quality metadata is calibration context, not citable evidence or final answer structure"
    })
}

fn source_class_coverage_from_evidence_pack(
    policy: &Value,
    query: &str,
    evidence_pack: &Value,
    evidence_coverage: &Value,
) -> Value {
    let mut class_counts = HashMap::<String, usize>::new();
    let mut domains = HashSet::<String>::new();
    let mut low_confidence_count = 0usize;
    if let Some(rows) = evidence_pack.as_array() {
        for row in rows {
            let source_class = clean_text(
                row.get("source_class").and_then(Value::as_str).unwrap_or("general_web"),
                120,
            )
            .to_ascii_lowercase();
            if !source_class.is_empty() {
                *class_counts.entry(source_class).or_insert(0) += 1;
            }
            let domain = clean_text(
                row.get("source_domain")
                    .or_else(|| row.get("source_scope"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                180,
            )
            .to_ascii_lowercase();
            if !domain.is_empty() && domain != "source" {
                domains.insert(domain);
            }
            if row.get("confidence").and_then(Value::as_str) == Some("low_confidence_raw") {
                low_confidence_count += 1;
            }
        }
    }
    let mut classes = class_counts
        .iter()
        .map(|(class, count)| json!({"source_class": class, "evidence_count": count}))
        .collect::<Vec<_>>();
    classes.sort_by(|a, b| {
        b.get("evidence_count")
            .and_then(Value::as_u64)
            .cmp(&a.get("evidence_count").and_then(Value::as_u64))
            .then_with(|| {
                a.get("source_class")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .cmp(b.get("source_class").and_then(Value::as_str).unwrap_or(""))
            })
    });
    let preferred = policy_string_list(
        policy,
        "/batch_query/source_diversification/preferred_source_classes",
    );
    let missing_preferred = preferred
        .iter()
        .filter(|class| !class_counts.contains_key(*class))
        .cloned()
        .collect::<Vec<_>>();
    let mut missing_facets = 0usize;
    if let Some(rows) = evidence_coverage.as_array() {
        missing_facets = rows
            .iter()
            .filter(|row| row.get("status").and_then(Value::as_str) == Some("missing"))
            .count();
    }
    let min_classes = source_class_diversity_min(policy, query);
    let item_count = value_array_len(evidence_pack);
    let status = if item_count == 0 {
        "absent"
    } else if class_counts.len() < min_classes {
        "limited"
    } else if missing_facets > 0 {
        "coverage_gaps"
    } else {
        "diverse"
    };
    json!({
        "version": "source_class_coverage_v1",
        "status": status,
        "source_class_count": class_counts.len(),
        "source_domain_count": domains.len(),
        "low_confidence_count": low_confidence_count,
        "classes": classes,
        "preferred_source_classes_missing": missing_preferred,
        "missing_facet_count": missing_facets,
        "thresholds": {
            "min_source_classes": min_classes
        },
        "non_goal": "do_not_require_any_specific_source_class_for_all_research"
    })
}

fn source_class_coverage_from_ranked_candidates(
    policy: &Value,
    query: &str,
    evidence_ranked: &[(Candidate, f64)],
    evidence_coverage: &Value,
) -> Value {
    let pack_like = Value::Array(
        evidence_ranked
            .iter()
            .map(|(candidate, _)| {
                json!({
                    "source_class": evidence_pack_source_class(policy, candidate),
                    "source_domain": candidate_domain_hint(candidate),
                    "confidence": if candidate_is_low_confidence_retained(candidate) {
                        "low_confidence_raw"
                    } else {
                        "usable"
                    }
                })
            })
            .collect::<Vec<_>>(),
    );
    source_class_coverage_from_evidence_pack(policy, query, &pack_like, evidence_coverage)
}

fn provider_attempts_from_retrieval_telemetry(retrieval_telemetry: &Value) -> Value {
    let attempts = retrieval_telemetry
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let provider_count = row
                        .get("provider_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let provider_raw_rows = row
                        .get("provider_raw_rows")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let synthesis_rows = row
                        .get("synthesis_candidate_rows")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let low_confidence_rows = row
                        .get("low_confidence_raw_rows")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let failure_count = row
                        .get("failure_reasons")
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0);
                    let status = if synthesis_rows > 0 {
                        "usable"
                    } else if low_confidence_rows > 0 {
                        "low_confidence"
                    } else if provider_raw_rows > 0 {
                        "filtered"
                    } else if provider_count > 0 || failure_count > 0 {
                        "failed_or_empty"
                    } else {
                        "not_recorded"
                    };
                    json!({
                        "query": row.get("query").cloned().unwrap_or_else(|| json!("")),
                        "phase": row.get("phase").cloned().unwrap_or_else(|| json!("unknown")),
                        "status": status,
                        "provider_count": provider_count,
                        "provider_raw_rows": provider_raw_rows,
                        "candidate_rows": row.get("candidate_rows").cloned().unwrap_or_else(|| json!(0)),
                        "synthesis_candidate_rows": synthesis_rows,
                        "low_confidence_raw_rows": low_confidence_rows,
                        "failure_reasons": row.get("failure_reasons").cloned().unwrap_or_else(|| json!([]))
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Value::Array(attempts)
}

fn provider_attempts_from_provider_results(provider_results: &Value) -> Value {
    let attempts = provider_results
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let raw_count = row
                        .get("provider_raw_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let synthesis_count = row
                        .get("synthesis_candidate_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let quality = row
                        .get("result_quality")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let ok = row.get("ok").and_then(Value::as_bool).unwrap_or(false);
                    let status = if quality == "usable" || synthesis_count > 0 {
                        "usable"
                    } else if quality == "low_signal" || quality == "low_confidence_raw" {
                        "low_confidence"
                    } else if raw_count > 0 && ok {
                        "filtered"
                    } else {
                        "failed_or_empty"
                    };
                    json!({
                        "query": row.get("query").cloned().unwrap_or_else(|| json!("")),
                        "phase": row.get("stage").cloned().unwrap_or_else(|| json!("provider_result")),
                        "provider": row.get("provider").cloned().unwrap_or_else(|| json!("unknown")),
                        "status": status,
                        "provider_count": 1,
                        "provider_raw_rows": raw_count,
                        "candidate_rows": raw_count,
                        "synthesis_candidate_rows": synthesis_count,
                        "low_confidence_raw_rows": if status == "low_confidence" { raw_count } else { 0 },
                        "failure_reasons": row.get("failure_reasons").cloned().unwrap_or_else(|| {
                            row.get("error")
                                .and_then(Value::as_str)
                                .map(|error| json!([error]))
                                .unwrap_or_else(|| json!([]))
                        })
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Value::Array(attempts)
}

fn retrieval_broker_report(
    status: &str,
    submitted_query_plan: Value,
    executed_query_plan: Value,
    query_plan_source: &str,
    second_pass_recovery: Value,
    retrieval_telemetry: &Value,
    provider_results: &Value,
    evidence_pack: &Value,
    evidence_coverage: &Value,
    tool_result_quality: &Value,
    source_class_coverage: &Value,
    evidence_pack_quality: &Value,
) -> Value {
    let mut provider_attempts = provider_attempts_from_retrieval_telemetry(retrieval_telemetry);
    if value_array_len(&provider_attempts) == 0 {
        provider_attempts = provider_attempts_from_provider_results(provider_results);
    }
    let provider_attempt_count = value_array_len(&provider_attempts);
    let provider_success_count = provider_attempts
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter(|row| row.get("status").and_then(Value::as_str) == Some("usable"))
                .count()
        })
        .unwrap_or(0);
    let evidence_status = evidence_pack_quality
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("absent");
    let provider_status = if provider_attempt_count == 0 {
        "not_recorded"
    } else if provider_success_count > 0 {
        "usable"
    } else {
        "degraded"
    };
    let submitted_query_count = value_array_len(&submitted_query_plan);
    let executed_query_count = value_array_len(&executed_query_plan);
    let provider_result_count = value_array_len(provider_results);
    let evidence_pack_count = value_array_len(evidence_pack);
    let coverage_facet_count = value_array_len(evidence_coverage);
    json!({
        "version": "web_research_broker_report_v1",
        "primitive": "web_research",
        "authority": "tool_cd_policy_runtime_observation",
        "chat_visibility": "hidden_until_synthesized",
        "status": status,
        "query_planning": {
            "submitted_query_plan": submitted_query_plan,
            "executed_query_plan": executed_query_plan,
            "query_plan_source": query_plan_source,
            "hidden_query_expansion": false
        },
        "lanes": [
            {
                "lane": "query_planning",
                "status": "complete",
                "submitted_query_count": submitted_query_count,
                "executed_query_count": executed_query_count
            },
            {
                "lane": "provider_retrieval",
                "status": provider_status,
                "attempt_count": provider_attempt_count,
                "usable_attempt_count": provider_success_count,
                "provider_result_count": provider_result_count
            },
            {
                "lane": "evidence_packaging",
                "status": evidence_status,
                "evidence_pack_count": evidence_pack_count
            },
            {
                "lane": "coverage_review",
                "status": source_class_coverage
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                "coverage_facet_count": coverage_facet_count
            }
        ],
        "provider_attempts": provider_attempts,
        "second_pass_recovery": second_pass_recovery,
        "source_class_coverage": source_class_coverage,
        "evidence_pack_quality": evidence_pack_quality,
        "tool_result_quality_flags": tool_result_quality
            .get("flags")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "synthesis_boundary": "broker diagnostics guide retrieval calibration but are not final-answer text or citable source evidence"
    })
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

fn ranked_payload_links_for_fallback_with_min_score(
    query: &str,
    payload: &Value,
    max_links: usize,
    min_score: f64,
) -> Vec<String> {
    let mut ranked = non_search_engine_links(payload, max_links.saturating_mul(4).max(max_links))
        .into_iter()
        .map(|link| {
            let score = fallback_link_score(query, &link);
            (link, score)
        })
        .filter(|(_, score)| *score > -1.0 && *score >= min_score)
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

fn ranked_payload_links_for_fallback(query: &str, payload: &Value, max_links: usize) -> Vec<String> {
    ranked_payload_links_for_fallback_with_min_score(query, payload, max_links, -1.0)
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
        if lowered.contains("comparison_entity_coverage_gap")
            || lowered.contains("source coverage to compare")
        {
            flags.push("comparison_evidence_insufficient".to_string());
        }
    }
    flags.sort();
    flags.dedup();
    flags
}

fn strong_evidence_available(query: &str, actionable_ranked: &[(Candidate, f64)]) -> bool {
    actionable_ranked.iter().any(|(candidate, score)| {
        if *score < 0.75 || candidate_is_low_confidence_retained(candidate) {
            return false;
        }
        let flags = candidate_quality_flags(query, candidate, *score);
        !flags.iter().any(|flag| {
            matches!(
                flag.as_str(),
                "low_trust_source"
                    | "thin_query_overlap"
                    | "freshness_unproven"
                    | "junk_marker"
                    | "low_score"
            )
        })
    })
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
    } else if status == "low_signal" {
        flags.push("low_signal".to_string());
    } else if status == "partial" {
        flags.push("partial_results".to_string());
    } else if evidence_count >= 2 && hard_partial_failures.is_empty() {
        flags.push("high_confidence".to_string());
    }
    if actionable_ranked
        .iter()
        .any(|(candidate, _)| candidate_is_low_confidence_retained(candidate))
    {
        flags.push("low_confidence_raw_evidence".to_string());
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
    if status == "ok" && strong_evidence_available(query, actionable_ranked) {
        flags.retain(|flag| flag != "low_signal");
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
    let coverage_status = if evidence_count == 0 {
        "missing"
    } else if weak_single_source || flags.iter().any(|flag| flag == "low_signal") {
        "weak"
    } else {
        "covered"
    };
    let mut missing_buckets = Vec::<String>::new();
    if evidence_count == 0 {
        missing_buckets.push("usable_evidence".to_string());
    }
    if weak_single_source {
        missing_buckets.push("source_diversity_or_confidence".to_string());
    }
    if flags
        .iter()
        .any(|flag| flag == "comparison_evidence_insufficient")
    {
        missing_buckets.push("comparison_coverage".to_string());
    }
    if flags.iter().any(|flag| flag == "freshness_unproven") {
        missing_buckets.push("freshness_signal".to_string());
    }
    missing_buckets.sort();
    missing_buckets.dedup();
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
        "coverage": {
            "bucket_status": coverage_status,
            "missing_buckets": missing_buckets,
            "source_domain_count": domains.len(),
            "basis": "usable_evidence_source_diversity_and_quality_flags"
        },
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
                "target primary, official, source-backed, or directly citable pages when possible",
                "use partial snippets, failure reasons, and off-topic signals to remove weak terms",
                "for current or recent research, prefer current/recent source-class searches such as changelog, release notes, repository, publication, announcement, security advisory, or methodology over broad stale year ranges",
                "for one named entity, keep the exact entity name in every retry query and vary source class or decision aspect rather than replacing it with loose synonyms",
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
