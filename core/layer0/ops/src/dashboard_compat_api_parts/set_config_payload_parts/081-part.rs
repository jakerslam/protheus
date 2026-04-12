fn response_is_actionable_tool_diagnostic(text: &str) -> bool {
    let cleaned = clean_text(text, 1_400);
    if cleaned.is_empty() {
        return false;
    }
    if response_looks_like_unsynthesized_web_snippet_dump(&cleaned)
        || response_looks_like_raw_web_artifact_dump(&cleaned)
        || response_contains_tool_telemetry_dump(&cleaned)
    {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    lowered.contains("low-signal snippets without synthesis")
        || lowered.contains("low-signal web snippets")
        || lowered.contains("raw web output")
        || lowered.contains("search returned no useful comparison findings")
        || lowered.contains("retrieval-quality miss")
        || lowered.contains("retrieval/synthesis miss")
        || lowered.contains("tooling is partially working")
        || lowered.contains("needs a query before it can run")
        || lowered.contains("query before it can run")
        || lowered.contains("fit safely in context")
        || lowered.contains("doctor --json")
}

fn rewrite_tool_result_for_user_summary(tool_name: &str, raw_result: &str) -> Option<String> {
    let cleaned = clean_text(raw_result, 2_400);
    if cleaned.is_empty() {
        return None;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.contains("search returned no useful comparison findings") {
        let base = cleaned
            .trim_end_matches(|ch| matches!(ch, '.' | '!' | '?'))
            .trim()
            .to_string();
        return Some(trim_text(
            &format!(
                "{}; this is a retrieval-quality miss, not proof that the systems are equivalent.",
                base
            ),
            420,
        ));
    }
    if response_is_actionable_tool_diagnostic(&cleaned) {
        return Some(trim_text(&cleaned, 420));
    }
    let normalized = normalize_tool_name(tool_name);
    let is_web_tool = matches!(
        normalized.as_str(),
        "batch_query"
            | "web_search"
            | "search_web"
            | "search"
            | "web_query"
            | "web_fetch"
            | "browse"
            | "web_conduit_fetch"
    );
    if !is_web_tool {
        return None;
    }
    if response_mentions_context_guard(&cleaned) {
        return Some(web_tool_context_guard_fallback("Live web retrieval"));
    }
    if let Some((rewritten, _)) = crate::tool_output_match_filter::rewrite_unsynthesized_web_dump(&cleaned)
    {
        return Some(trim_text(&rewritten, 420));
    }
    if response_looks_like_raw_web_artifact_dump(&cleaned) {
        return Some(
            "I only have raw web output (placeholder or page/search chrome), not synthesized findings yet. I can rerun with `batch_query` or a narrower query and return a concise answer with sources."
                .to_string(),
        );
    }
    if response_contains_tool_telemetry_dump(&cleaned) {
        return Some(
            "The tool emitted internal telemetry instead of a user-facing answer. I can retry the retrieval or diagnose the failing lane."
                .to_string(),
        );
    }
    if lowered.contains("search returned no useful information")
        || response_is_no_findings_placeholder(&cleaned)
    {
        return Some(
            "Web retrieval ran, but this turn still came back without usable findings. That is a retrieval/synthesis miss, not a silent success. Retry with a narrower query or one specific source URL."
                .to_string(),
        );
    }
    None
}
