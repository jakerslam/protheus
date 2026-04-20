fn chat_ui_is_meta_diagnostic_request(lowered: &str) -> bool {
    if lowered.is_empty() {
        return false;
    }
    if chat_ui_has_explicit_web_intent(lowered) {
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
