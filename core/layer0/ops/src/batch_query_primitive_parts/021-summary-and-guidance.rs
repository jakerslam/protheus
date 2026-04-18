fn local_subject_comparison_summary(query: &str) -> String {
    let comparison_entities = comparison_entities_from_query(query);
    let external_side = comparison_entities
        .into_iter()
        .find(|entity| entity != "infring")
        .unwrap_or_else(|| "the requested external system".to_string());
    format!(
        "Web retrieval alone cannot compare this local workspace/system to {}. Use workspace analysis for the local side and web retrieval for the external side, then synthesize both.",
        external_side
    )
}

fn framework_forum_domain(domain: &str) -> bool {
    let lowered = clean_text(domain, 160).to_ascii_lowercase();
    lowered.contains("zhihu.com")
        || lowered.contains("reddit.com")
        || lowered.contains("quora.com")
        || lowered.contains("news.ycombinator.com")
}

fn framework_official_domain(domain: &str) -> bool {
    let lowered = clean_text(domain, 160).to_ascii_lowercase();
    lowered.contains("langchain.com")
        || lowered.contains("openai.com")
        || lowered.contains("openai.github.io")
        || lowered.contains("crewai.com")
        || lowered.contains("github.com")
        || lowered.contains("microsoft.com")
}

fn framework_summary_contains_low_signal_sources(text: &str) -> bool {
    let lowered = clean_text(text, 800).to_ascii_lowercase();
    lowered.contains("zhihu.com")
        || lowered.contains("reddit.com")
        || lowered.contains("quora.com")
        || lowered.contains("langgraph.com.cn")
        || lowered.contains("crewai.org.cn")
        || lowered.contains(".org.cn")
        || lowered.contains(".com.cn")
        || lowered.contains("support.microsoft.com")
}

fn summary_looks_like_competitive_programming_dump(text: &str) -> bool {
    let lowered = clean_text(text, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let marker_hits = [
        "given a tree",
        "input specification",
        "output specification",
        "sample input",
        "sample output",
        "#include <stdio.h>",
        "int main()",
        "public class",
        "translate the following java code",
        "csdn.net",
        "acm",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    marker_hits >= 3
}

fn cached_evidence_domains(evidence_refs: &Value, max_domains: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let Some(rows) = evidence_refs.as_array() else {
        return out;
    };
    for row in rows {
        let locator = clean_text(row.get("locator").and_then(Value::as_str).unwrap_or(""), 2_200);
        let title = clean_text(row.get("title").and_then(Value::as_str).unwrap_or(""), 200);
        for domain in extract_domains_from_text(&format!("{locator} {title}"), 2) {
            let key = domain.to_ascii_lowercase();
            if seen.insert(key) {
                out.push(domain);
                if out.len() >= max_domains.max(1) {
                    return out;
                }
            }
        }
    }
    out
}

fn cached_framework_summary_requires_refresh(
    query: &str,
    raw_summary: &str,
    evidence_refs: &Value,
) -> bool {
    if !is_framework_catalog_intent(query) {
        return false;
    }
    let lowered = clean_text(raw_summary, 600).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let evidence_domains = cached_evidence_domains(evidence_refs, 12);
    let has_official_evidence = evidence_domains.iter().any(|domain| framework_official_domain(domain));
    if !has_official_evidence {
        return false;
    }
    let has_forum_evidence = evidence_domains.iter().any(|domain| framework_forum_domain(domain));
    let forum_led_summary = evidence_domains
        .iter()
        .filter(|domain| framework_forum_domain(domain))
        .any(|domain| lowered.starts_with(&format!("key findings: {domain}:")));
    let thin_summary = framework_name_hits(&lowered) < 2;
    forum_led_summary || (has_forum_evidence && thin_summary)
}

fn framework_label_from_cached_domain(domain: &str) -> Option<&'static str> {
    let lowered = clean_text(domain, 160).to_ascii_lowercase();
    if lowered.contains("langchain.com") {
        return Some("LangGraph");
    }
    if lowered.contains("crewai.com") {
        return Some("CrewAI");
    }
    if lowered.contains("openai.com") || lowered.contains("openai.github.io") {
        return Some("OpenAI Agents SDK");
    }
    if lowered.contains("microsoft.com") || lowered.contains("microsoft.github.io") {
        return Some("AutoGen");
    }
    if lowered.contains("huggingface.co") {
        return Some("smolagents");
    }
    None
}

fn cached_framework_catalog_rewrite(raw_summary: &str, evidence_refs: &Value) -> Option<String> {
    let mut insights = Vec::<String>::new();
    let mut seen_frameworks = HashSet::<String>::new();
    let rows = evidence_refs.as_array()?;

    for row in rows {
        let title = clean_text(row.get("title").and_then(Value::as_str).unwrap_or(""), 220);
        let locator = clean_text(row.get("locator").and_then(Value::as_str).unwrap_or(""), 320);
        let domain = extract_domains_from_text(&format!("{title} {locator}"), 1)
            .into_iter()
            .next()
            .unwrap_or_default();
        let mut names = framework_names_in_text(&format!("{title} {locator}"));
        if names.is_empty() {
            if let Some(label) = framework_label_from_cached_domain(&domain) {
                names.push(label);
            }
        }
        for framework in names {
            let framework_key = framework.to_ascii_lowercase();
            if !seen_frameworks.insert(framework_key) {
                continue;
            }
            let insight = if domain.is_empty() {
                format!("{framework}: official framework source captured")
            } else {
                format!("{framework} ({domain}): official framework source captured")
            };
            insights.push(insight);
            if insights.len() >= 4 {
                break;
            }
        }
        if insights.len() >= 4 {
            break;
        }
    }

    for framework in framework_names_in_text(raw_summary) {
        let framework_key = framework.to_ascii_lowercase();
        if !seen_frameworks.insert(framework_key) {
            continue;
        }
        insights.push(format!("{framework}: secondary web evidence referenced this framework"));
        if insights.len() >= 4 {
            break;
        }
    }

    (insights.len() >= 2).then(|| format!("Key findings: {}", insights.join("; ")))
}

fn rewrite_cached_batch_query_summary(
    query: &str,
    source: &str,
    raw_summary: &str,
    evidence_refs: &Value,
    partial_failure_details: &Value,
) -> String {
    let raw_summary = clean_text(raw_summary, 240);
    let raw_summary_lowered = clean_text(&raw_summary, 320).to_ascii_lowercase();
    if summary_looks_like_competitive_programming_dump(&raw_summary) {
        return no_results_summary_for_batch_query(
            query,
            source,
            partial_failure_details,
            Some(
                "Web retrieval returned content that appears unrelated to the request intent (query_result_mismatch). Retry with a narrower query or one specific source URL."
                    .to_string(),
            ),
        );
    }
    if is_local_subject_comparison_query(query) {
        return local_subject_comparison_summary(query);
    }
    if raw_summary_lowered.contains("search returned no useful comparison findings") {
        let comparison_entities = comparison_entities_from_query(query);
        let entity_label = if comparison_entities.len() >= 2 {
            comparison_entities.join(" vs ")
        } else {
            "the requested sides".to_string()
        };
        return format!(
            "Search did not produce enough source coverage to compare {} in this turn. This is a retrieval-quality miss, not proof the systems are equivalent. Retry with named competitors or one specific source URL per side.",
            entity_label
        );
    }
    if is_framework_catalog_intent(query) && framework_name_hits(&raw_summary_lowered) < 2 {
        if let Some(rewritten) = cached_framework_catalog_rewrite(&raw_summary, evidence_refs) {
            return trim_words(&rewritten, 80);
        }
    }
    if raw_summary_lowered.contains("search returned no useful information")
        || raw_summary_lowered.contains("don't have usable tool findings from this turn yet")
        || raw_summary_lowered.contains("dont have usable tool findings from this turn yet")
    {
        return no_results_summary_for_batch_query(query, source, partial_failure_details, None);
    }
    if looks_like_low_signal_search_summary(&raw_summary) {
        return no_results_summary_for_batch_query(query, source, partial_failure_details, None);
    }
    raw_summary
}

fn no_results_summary_for_batch_query(
    query: &str,
    source: &str,
    partial_failure_details: &Value,
    comparison_guard_summary: Option<String>,
) -> String {
    if is_local_subject_comparison_query(query) {
        return local_subject_comparison_summary(query);
    }
    let anti_bot_detected = partial_failure_details
        .as_array()
        .map(|rows| {
            rows.iter().any(|row| {
                clean_text(row.as_str().unwrap_or(""), 320)
                    .to_ascii_lowercase()
                    .contains("anti_bot_challenge")
            })
        })
        .unwrap_or(false);
    let has_partial_failures = partial_failure_details
        .as_array()
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    if anti_bot_detected {
        return "Search providers returned anti-bot challenge pages before usable content was extracted. Retry with specific source URLs or alternate providers."
            .to_string();
    }
    if let Some(summary) = comparison_guard_summary {
        return summary;
    }
    if has_partial_failures {
        if is_framework_catalog_intent(query) {
            return "Search providers ran, but did not return enough catalog-style framework evidence in this turn. Retry with named frameworks or one specific source URL for source-backed findings."
                .to_string();
        }
        return "Search providers ran, but only low-signal or low-relevance web results came back in this turn. Retry with a narrower query or one specific source URL for source-backed findings."
            .to_string();
    }
    if source == "web" && is_framework_catalog_intent(query) {
        return "Web retrieval ran, but did not return enough catalog-style framework evidence in this turn. Retry with named frameworks or one specific source URL for source-backed findings."
            .to_string();
    }
    if source == "web" {
        return "Web retrieval ran, but no usable findings were extracted in this turn. Retry with a narrower query or one specific source URL for source-backed findings."
            .to_string();
    }
    crate::tool_output_match_filter::no_findings_user_copy().to_string()
}
