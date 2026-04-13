fn is_framework_catalog_intent(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    let ranking_marker = [
        "top ",
        "best ",
        "leading ",
        "popular ",
        "ranking",
        "rankings",
        "landscape",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    let framework_marker = [
        "agent framework",
        "agent frameworks",
        "agentic framework",
        "agentic frameworks",
        "framework",
        "frameworks",
        "agents sdk",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    ranking_marker && framework_marker
}

fn canonical_framework_catalog_focus(query: &str) -> Option<String> {
    if !is_framework_catalog_intent(query) {
        return None;
    }
    let tokens = clean_text(query, 600)
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .filter(|token| token.len() > 2 || token.eq_ignore_ascii_case("ai"))
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| {
            !matches!(
                token.as_str(),
                "top" | "best" | "leading" | "popular" | "ranking" | "rankings" | "landscape"
            )
        })
        .collect::<Vec<_>>();
    let focus = clean_text(&tokens.join(" "), 600);
    if focus.contains("framework") {
        Some(focus)
    } else {
        None
    }
}

fn preferred_query_rewrite(base: &str) -> String {
    if looks_like_instructional_query(base) {
        return normalize_instructional_query(base)
            .unwrap_or_else(|| clean_text(&format!("{base} overview"), 600));
    }
    if let Some(focus) = canonical_framework_catalog_focus(base) {
        return clean_text(&format!("{focus} comparison"), 600);
    }
    clean_text(&format!("{base} overview"), 600)
}

fn is_local_subject_comparison_query(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    let deictic_local_subject = [
        "this system",
        "this workspace",
        "this stack",
        "this platform",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    deictic_local_subject && is_benchmark_or_comparison_intent(query)
}

fn framework_name_hits(text: &str) -> usize {
    let lowered = clean_text(text, 2_400).to_ascii_lowercase();
    [
        "langgraph",
        "openai agents sdk",
        "autogen",
        "crewai",
        "llamaindex",
        "semantic kernel",
        "haystack",
        "mastra",
        "smolagents",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count()
}

fn looks_like_framework_catalog_text(text: &str) -> bool {
    let lowered = clean_text(text, 2_400).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if framework_name_hits(&lowered) >= 2 {
        return true;
    }
    lowered.contains("agent frameworks such as")
        || lowered.contains("popular agent frameworks")
        || lowered.contains("top agent frameworks")
        || lowered.contains("agentic frameworks")
}
