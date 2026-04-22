// Layer ownership: core/layer0/ops::batch-query-api (authoritative)
    let min_synthesis_score = minimum_synthesis_score(benchmark_intent);
    let mut actionable_ranked = ranked
        .into_iter()
        .filter(|(row, score)| {
            let snippet = clean_text(&row.snippet, 1_200);
            let domain = candidate_domain_hint(row);
            !snippet.is_empty()
                && *score >= min_synthesis_score
                && !looks_like_ack_only(&snippet)
                && !looks_like_low_signal_search_summary(&snippet)
                && !looks_like_source_only_snippet(&snippet)
                && !is_search_engine_domain(&domain)
                && candidate_passes_relevance_gate(&rerank_query, row, benchmark_intent)
                && candidate_is_substantive(&rerank_query, row, benchmark_intent)
                && !(benchmark_intent && looks_like_definition_candidate(row))
                && !(benchmark_intent && looks_like_comparison_noise_candidate(row))
        })
        .collect::<Vec<_>>();

    let comparison_entities = if benchmark_intent {
        comparison_entities_from_query(&query)
    } else {
        Vec::new()
    };
    let mut comparison_guard_summary = None::<String>;
    if comparison_entities.len() >= 2 {
        let coverage_ok = comparison_entities.iter().all(|entity| {
            actionable_ranked
                .iter()
                .any(|(row, _)| candidate_mentions_entity(row, entity))
        });
        if !coverage_ok {
            actionable_ranked.clear();
            comparison_guard_summary = Some(format!(
                "Search did not produce enough source coverage to compare {} in this turn. This is a retrieval-quality miss, not proof the systems are equivalent. Retry with named competitors or one specific source URL per side.",
                comparison_entities.join(" vs ")
            ));
        }
    }

    let evidence_refs = actionable_ranked
        .iter()
        .map(|(row, score)| EvidenceRef {
            source_kind: row.source_kind.clone(),
            title: row.title.clone(),
            locator: row.locator.clone(),
            excerpt_hash: row.excerpt_hash.clone(),
            score: (*score * 100.0).round() / 100.0,
            timestamp: row.timestamp.clone(),
            permissions: row.permissions.clone(),
        })
        .collect::<Vec<_>>();

    let hard_partial_failures = partial_failures
        .iter()
        .filter(|row| !is_benign_partial_failure(row))
        .cloned()
        .collect::<Vec<_>>();
    let status = if evidence_refs.is_empty() {
        "no_results"
    } else if hard_partial_failures.is_empty() {
        "ok"
    } else {
        "partial"
    };
    let summary = if evidence_refs.is_empty() {
        let partial_failure_value = Value::Array(
            hard_partial_failures
                .iter()
                .cloned()
                .map(Value::String)
                .collect::<Vec<_>>(),
        );
        no_results_summary_for_batch_query(
            &query,
            &source,
            &partial_failure_value,
            comparison_guard_summary,
        )
    } else {
        let mut synthesized_insights = Vec::<String>::new();
        let mut seen_domains = HashSet::<String>::new();
        for (candidate, _) in &actionable_ranked {
            let snippet_raw = if benchmark_intent {
                extract_metric_focused_fragment(&candidate.snippet)
            } else {
                clean_text(&candidate.snippet, 1_200)
            };
            let snippet = trim_words(&snippet_raw, if benchmark_intent { 30 } else { 42 });
            if snippet.is_empty() {
                continue;
            }
            if benchmark_intent {
                if !looks_like_metric_rich_text(&snippet_raw) && looks_like_instructional_query(&snippet_raw) {
                    continue;
                }
                let comparison_haystack = clean_text(
                    &format!("{} {} {}", candidate.title, snippet_raw, candidate.locator),
                    1_600,
                )
                .to_ascii_lowercase();
                let entity_hits = comparison_entities
                    .iter()
                    .filter(|entity| comparison_haystack.contains(entity.as_str()))
                    .count();
                let comparative_copy = comparison_haystack.contains(" vs ")
                    || comparison_haystack.contains("versus")
                    || comparison_haystack.contains("compared")
                    || comparison_haystack.contains("better")
                    || comparison_haystack.contains("worse")
                    || comparison_haystack.contains("faster")
                    || comparison_haystack.contains("slower");
                let benchmark_quality_ok = looks_like_metric_rich_text(&snippet_raw)
                    || (comparison_entities.len() >= 2 && entity_hits >= 1 && comparative_copy);
                if !benchmark_quality_ok {
                    continue;
                }
            }
            let domain = candidate_domain_hint(candidate);
            let domain_key = clean_text(&domain, 160).to_ascii_lowercase();
            if domain_key != "source" && !domain_key.is_empty() && !seen_domains.insert(domain_key)
            {
                continue;
            }
            let insight = if domain == "source" {
                snippet.clone()
            } else {
                format!("{domain}: {snippet}")
            };
            if synthesized_insights
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&insight))
            {
                continue;
            }
            synthesized_insights.push(insight);
            if synthesized_insights.len() >= budget.max_evidence.max(1) {
                break;
            }
        }
        if is_framework_catalog_intent(&query) {
            let fallback_insights =
                framework_catalog_fallback_insights(&actionable_ranked, budget.max_evidence);
            let synthesized_joined = synthesized_insights.join(" ");
            let fallback_joined = fallback_insights.join(" ");
            if framework_name_hits(&synthesized_joined) < 2
                && framework_name_hits(&fallback_joined)
                    > framework_name_hits(&synthesized_joined)
            {
                synthesized_insights = fallback_insights.clone();
            }
            if framework_summary_contains_low_signal_sources(&synthesized_insights.join(" "))
                && !fallback_insights.is_empty()
            {
                synthesized_insights = fallback_insights;
            }
        }
        if synthesized_insights.is_empty() {
            if source == "web" {
                "Web retrieval ran, but only low-signal snippets were available for synthesis in this turn. Retry with a narrower query or one specific source URL for source-backed findings."
                    .to_string()
            } else {
                crate::tool_output_match_filter::no_findings_user_copy().to_string()
            }
        } else {
            let comparison_intent = comparison_entities.len() >= 2;
            let prefix = if source == "web" {
                if comparison_intent {
                    "Comparison findings:"
                } else if benchmark_intent {
                    "Web benchmark synthesis:"
                } else {
                    "From web retrieval:"
                }
            } else if comparison_intent {
                "Comparison findings:"
            } else if benchmark_intent {
                "Benchmark findings:"
            } else {
                "Key findings:"
            };
            trim_words(
                &format!("{prefix} {}", synthesized_insights.join("; ")),
                budget.max_summary_tokens,
            )
        }
    };
