// FILE_SIZE_EXCEPTION: reason=Single action-dispatch function with dense branch graph; split deferred pending semantic extraction; owner=jay; expires=2026-04-23
fn clean_chat_text_preserve_layout(value: &str, max_len: usize) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|ch| *ch == '\n' || *ch == '\t' || !ch.is_control())
        .take(max_len)
        .collect::<String>()
}

fn assistant_runtime_access_denied(assistant_lower: &str) -> bool {
    const DENIED_SIGNATURES: [&str; 7] = [
        "don't have access",
        "do not have access",
        "cannot access",
        "without system monitoring",
        "text-based ai assistant",
        "cannot directly interface",
        "no access to",
    ];
    DENIED_SIGNATURES
        .iter()
        .any(|signature| assistant_lower.contains(signature))
}

fn runtime_sync_requested(input_lower: &str) -> bool {
    input_lower.contains("report runtime sync now")
        || ((input_lower.contains("queue depth")
            || input_lower.contains("cockpit blocks")
            || input_lower.contains("conduit signals"))
            && (input_lower.contains("runtime")
                || input_lower.contains("sync")
                || input_lower.contains("status")
                || input_lower.contains("what changed")))
}

fn app_chat_has_explicit_web_intent(lowered: &str) -> bool {
    lowered.contains("web search")
        || lowered.contains("websearch")
        || lowered.contains("search the web")
        || lowered.contains("search online")
        || lowered.contains("find information")
        || lowered.contains("finding information")
        || lowered.contains("look it up")
        || lowered.contains("look this up")
        || lowered.contains("search again")
        || lowered.contains("best chili recipes")
}

fn app_chat_is_meta_diagnostic_request(lowered: &str) -> bool {
    if lowered.is_empty() {
        return false;
    }
    if app_chat_has_explicit_web_intent(lowered) {
        return false;
    }
    if [
        "that was just a test",
        "that was a test",
        "did you do the web request",
        "did you try it",
        "where did that come from",
        "where the hell did that come from",
        "you returned no result",
        "you hallucinated",
        "answer the question",
    ]
    .iter()
    .any(|marker| lowered.contains(*marker))
    {
        return true;
    }
    let meta_hits = [
        "what happened",
        "workflow",
        "tool call",
        "web tooling",
        "hallucination",
        "hallucinated",
        "training data",
        "context issue",
        "last response",
        "previous response",
        "system issue",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if meta_hits == 0 {
        return false;
    }
    let signal_terms = lowered
        .split_whitespace()
        .filter(|token| token.len() >= 3)
        .count();
    meta_hits >= 2 || signal_terms <= 7
}

fn app_chat_requests_live_web(raw_input_lower: &str) -> bool {
    let lowered = clean_text(raw_input_lower, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let explicit_web_intent = app_chat_has_explicit_web_intent(&lowered);
    if explicit_web_intent {
        return true;
    }
    if app_chat_is_meta_diagnostic_request(&lowered) {
        return false;
    }
    let inferred_online_research = (lowered.contains("latest")
        || lowered.contains("current")
        || lowered.contains("today"))
        && (lowered.contains("framework")
            || lowered.contains("frameworks")
            || lowered.contains("news")
            || lowered.contains("price")
            || lowered.contains("release"))
        && (lowered.contains("what")
            || lowered.contains("which")
            || lowered.contains("top")
            || lowered.contains("best")
            || lowered.contains("compare"));
    inferred_online_research
}

fn app_chat_extract_web_query(raw_input: &str) -> String {
    let cleaned = clean_text(raw_input, 600);
    if cleaned.is_empty() {
        return "latest public web updates".to_string();
    }
    if let Some(start) = cleaned.find('"') {
        if let Some(end_rel) = cleaned[start + 1..].find('"') {
            let quoted = clean_text(&cleaned[start + 1..start + 1 + end_rel], 320);
            if !quoted.is_empty() {
                return quoted;
            }
        }
    }
    let lowered = cleaned.to_ascii_lowercase();
    for marker in ["about ", "for "] {
        if let Some(idx) = lowered.rfind(marker) {
            let candidate = clean_text(&cleaned[idx + marker.len()..], 320);
            if !candidate.is_empty() {
                return candidate;
            }
        }
    }
    cleaned
}

fn app_chat_alignment_terms(text: &str, max_terms: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for token in clean_text(text, 2_000)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
    {
        if token.len() < 3 {
            continue;
        }
        if matches!(
            token,
            "the"
                | "and"
                | "for"
                | "with"
                | "this"
                | "that"
                | "from"
                | "into"
                | "what"
                | "when"
                | "where"
                | "why"
                | "how"
                | "about"
                | "just"
                | "again"
                | "please"
                | "best"
                | "top"
                | "give"
                | "show"
                | "find"
                | "search"
                | "web"
                | "results"
                | "result"
        ) {
            continue;
        }
        if out.iter().any(|existing| existing == token) {
            continue;
        }
        out.push(token.to_string());
        if out.len() >= max_terms {
            break;
        }
    }
    out
}
