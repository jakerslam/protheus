
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
    let overlap = query_tokens.intersection(&candidate_tokens).count();
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
    let overlap_ratio = overlap as f64 / query_tokens.len() as f64;
    if benchmark_intent {
        if overlap < 2 && overlap_ratio < 0.22 && !looks_like_metric_rich_text(&candidate.snippet) {
            return false;
        }
        if looks_like_portal_noise_candidate(candidate) && overlap < 3 {
            return false;
        }
        return true;
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
