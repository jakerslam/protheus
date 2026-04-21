fn chat_ui_is_meta_diagnostic_request(lowered: &str) -> bool {
    if lowered.is_empty() {
        return false;
    }
    let explicit_web_intent = chat_ui_has_explicit_web_intent(lowered);
    let explicit_web_diagnostic_qualifier = [
        "randomly again",
        "random web",
        "bad web request",
        "shouldnt require a web search",
        "shouldn't require a web search",
        "didnt require a web search",
        "didn't require a web search",
        "why did my last prompt become a web search",
        "web search kicking in",
        "kicking in randomly",
    ]
    .iter()
    .any(|marker| lowered.contains(*marker));
    if explicit_web_intent && !explicit_web_diagnostic_qualifier {
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
        "automatic tool selection",
        "automatic tool calls",
        "system shouldnt be doing automatic tool calls",
        "system shouldn't be doing automatic tool calls",
        "backend automation",
        "bad web request",
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
        "tool routing",
        "automatic tool",
        "backend automation",
        "file tooling",
        "local tooling",
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
