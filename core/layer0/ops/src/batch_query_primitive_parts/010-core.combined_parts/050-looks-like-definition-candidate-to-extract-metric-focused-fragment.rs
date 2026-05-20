
fn looks_like_definition_candidate(candidate: &Candidate) -> bool {
    let lowered = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    [
        "dictionary",
        "definition",
        "meaning",
        "thesaurus",
        "merriam-webster",
        "dictionary.com",
        "cambridge.org/dictionary",
        "collinsdictionary",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_comparison_noise_candidate(candidate: &Candidate) -> bool {
    let lowered = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    let low_quality_domain = [
        "wordreference.com",
        "forum.wordreference.com",
        "wiktionary.org",
        "grammar",
        "english usage",
        "merriam-webster",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    let noisy_compare_form = lowered.contains("compare [a with b]")
        || lowered.contains("compare a with b")
        || lowered.contains("vs compare")
        || lowered.contains("wordreference forums");
    low_quality_domain || noisy_compare_form
}

fn query_asks_for_word_meaning(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    [
        "definition of",
        "meaning of",
        "define ",
        "dictionary",
        "what does",
        "what is the meaning",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn query_asks_for_shopping_or_products(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    [
        "buy ",
        "price",
        "pricing",
        "deal",
        "discount",
        "where can i buy",
        "shopping",
        "retailer",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn query_asks_for_music_or_lyrics(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    [
        "lyrics", "song", "album", "music", "artist", "track", "chords",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_shopping_candidate(candidate: &Candidate) -> bool {
    let lowered = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    [
        "bestbuy.",
        "best buy",
        "add to cart",
        "shopping cart",
        "free shipping",
        "coupon",
        "store pickup",
        "shop now",
        "product reviews",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_lyrics_candidate(candidate: &Candidate) -> bool {
    let lowered = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    [
        "lyrics",
        "song lyrics",
        "genius.com",
        "azlyrics",
        "musixmatch",
        "chords",
        "official audio",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_off_intent_noise_candidate(query: &str, candidate: &Candidate) -> bool {
    (looks_like_definition_candidate(candidate) && !query_asks_for_word_meaning(query))
        || (looks_like_shopping_candidate(candidate) && !query_asks_for_shopping_or_products(query))
        || (looks_like_lyrics_candidate(candidate) && !query_asks_for_music_or_lyrics(query))
}

fn is_official_source_query_lane(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    [
        "official site",
        "official documentation",
        "official source",
        "official sources",
        "primary source evidence",
        "project sources",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn candidate_has_trusted_official_source_signal(query: &str, candidate: &Candidate) -> bool {
    if !is_official_source_query_lane(query) {
        return false;
    }
    let combined = candidate_relevance_text(candidate);
    let domain = candidate_domain_hint(candidate);
    let (overlap, distinctive_overlap, _) = query_overlap_profile(query, candidate);
    let trusted_source = source_trust_adjustment(candidate) >= 0.15
        || framework_official_domain(&domain)
        || combined.to_ascii_lowercase().contains("/docs")
        || combined.to_ascii_lowercase().contains("/documentation")
        || combined.to_ascii_lowercase().contains("/reference")
        || combined.to_ascii_lowercase().contains("/api");
    if !trusted_source {
        return false;
    }
    distinctive_overlap >= 1
        || overlap >= 1
        || looks_like_framework_overview_text(&combined)
        || looks_like_framework_catalog_text(&combined)
}

fn candidate_title_for_relevance(candidate: &Candidate) -> String {
    if candidate
        .title
        .to_ascii_lowercase()
        .starts_with("web result from ")
    {
        String::new()
    } else {
        candidate.title.clone()
    }
}

fn candidate_relevance_text(candidate: &Candidate) -> String {
    format!(
        "{} {} {}",
        candidate_title_for_relevance(candidate),
        candidate.snippet,
        candidate.locator
    )
}

fn is_relevance_stop_token(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "and"
            | "any"
            | "are"
            | "as"
            | "at"
            | "by"
            | "for"
            | "from"
            | "how"
            | "in"
            | "into"
            | "is"
            | "it"
            | "its"
            | "of"
            | "on"
            | "or"
            | "our"
            | "the"
            | "their"
            | "them"
            | "this"
            | "those"
            | "to"
            | "try"
            | "was"
            | "we"
            | "were"
            | "what"
            | "which"
            | "who"
            | "when"
            | "where"
            | "why"
            | "some"
            | "give"
            | "tell"
            | "show"
            | "find"
            | "about"
            | "compare"
            | "comparison"
            | "versus"
            | "with"
            | "you"
            | "your"
            | "verify"
            | "report"
            | "top"
            | "benchmark"
            | "benchmarks"
            | "metric"
            | "metrics"
            | "performance"
    )
}

fn tokenize_relevance(raw: &str, cap: usize) -> HashSet<String> {
    let mut out = HashSet::<String>::new();
    for token in clean_text(raw, 4_800)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
    {
        let normalized = token.trim();
        if normalized.len() < 3 || is_relevance_stop_token(normalized) {
            continue;
        }
        out.insert(normalized.to_string());
        if out.len() >= cap.max(1) {
            break;
        }
    }
    out
}

fn is_weak_relevance_token(token: &str) -> bool {
    if token.chars().all(|ch| ch.is_ascii_digit()) {
        return true;
    }
    matches!(
        token,
        "current"
            | "latest"
            | "recent"
            | "model"
            | "official"
            | "primary"
            | "source"
            | "sources"
            | "evidence"
            | "overview"
            | "guide"
            | "general"
            | "information"
            | "online"
            | "web"
            | "news"
            | "research"
            | "report"
            | "reports"
            | "science"
            | "scientific"
    )
}

fn query_overlap_profile(query: &str, candidate: &Candidate) -> (usize, usize, usize) {
    let query_tokens = tokenize_relevance(query, 40);
    if query_tokens.is_empty() {
        return (0, 0, 0);
    }
    let candidate_tokens = tokenize_relevance(&candidate_relevance_text(candidate), 120);
    if candidate_tokens.is_empty() {
        return (0, 0, query_tokens.len());
    }
    let overlap = query_tokens
        .iter()
        .filter(|token| candidate_tokens.contains(token.as_str()))
        .count();
    let distinctive_overlap = query_tokens
        .iter()
        .filter(|token| !is_weak_relevance_token(token))
        .filter(|token| candidate_tokens.contains(token.as_str()))
        .count();
    (overlap, distinctive_overlap, query_tokens.len())
}

fn has_only_weak_query_overlap(query: &str, candidate: &Candidate) -> bool {
    let (overlap, distinctive_overlap, query_len) = query_overlap_profile(query, candidate);
    query_len >= 2 && overlap > 0 && distinctive_overlap == 0
}

fn looks_like_portal_noise_candidate(candidate: &Candidate) -> bool {
    let lowered = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    [
        "login page",
        "log in",
        "sign in",
        "forgot password",
        "mychart",
        "watch live",
        "home news sport business",
        "create account",
        "manage account",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn candidate_passes_relevance_gate(
    query: &str,
    candidate: &Candidate,
    benchmark_intent: bool,
) -> bool {
    let query_tokens = tokenize_relevance(query, 40);
    if query_tokens.is_empty() {
        return true;
    }
    let candidate_relevance = candidate_relevance_text(candidate);
    let candidate_tokens = tokenize_relevance(&candidate_relevance, 120);
    if candidate_tokens.is_empty() {
        return false;
    }
    let (overlap, distinctive_overlap, query_len) = query_overlap_profile(query, candidate);
    if is_framework_catalog_intent(query) && overlap == 0 {
        let combined = candidate_relevance.clone();
        let domain = candidate_domain_hint(candidate);
        if framework_name_hits(&combined) >= 1
            && looks_like_framework_overview_text(&combined)
            && framework_official_domain(&domain)
        {
            return true;
        }
    }
    if overlap == 0 {
        return false;
    }
    if query_len >= 3 && distinctive_overlap == 0 {
        return false;
    }
    let overlap_ratio = overlap as f64 / query_len as f64;
    if benchmark_intent {
        if overlap < 2 && overlap_ratio < 0.22 && !looks_like_metric_rich_text(&candidate.snippet) {
            return false;
        }
        if query_len >= 3 && overlap < 2 && distinctive_overlap < 1 {
            return false;
        }
        if looks_like_portal_noise_candidate(candidate) && overlap < 3 {
            return false;
        }
        return true;
    }
    if query_len >= 3 && overlap < 2 {
        return false;
    }
    if looks_like_portal_noise_candidate(candidate) && overlap < 2 && overlap_ratio < 0.25 {
        return false;
    }
    true
}

fn candidate_mentions_entity(candidate: &Candidate, entity: &str) -> bool {
    let needle = clean_text(entity, 80).to_ascii_lowercase();
    if needle.is_empty() {
        return false;
    }
    let haystack = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    haystack.contains(&needle)
}

fn extract_metric_focused_fragment(text: &str) -> String {
    let cleaned = clean_text(text, 1_200);
    if cleaned.is_empty() {
        return String::new();
    }
    for segment in cleaned.split(['.', ';', '\n', '|']) {
        let segment_clean = clean_text(segment, 400);
        if segment_clean.is_empty() {
            continue;
        }
        if looks_like_metric_rich_text(&segment_clean) {
            return segment_clean;
        }
    }
    cleaned
}
