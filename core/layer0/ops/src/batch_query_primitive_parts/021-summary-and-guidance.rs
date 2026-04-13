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

fn rewrite_cached_batch_query_summary(
    query: &str,
    source: &str,
    raw_summary: &str,
    partial_failure_details: &Value,
) -> String {
    let raw_summary = clean_text(raw_summary, 240);
    let raw_summary_lowered = clean_text(&raw_summary, 320).to_ascii_lowercase();
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
    if raw_summary_lowered.contains("search returned no useful information")
        || raw_summary_lowered.contains("don't have usable tool findings from this turn yet")
        || raw_summary_lowered.contains("dont have usable tool findings from this turn yet")
    {
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

