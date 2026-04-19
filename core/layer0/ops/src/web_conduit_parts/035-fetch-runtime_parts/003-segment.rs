fn fetch_requested_url_looks_meta_diagnostic(raw_requested_url: &str) -> bool {
    let lowered = clean_text(raw_requested_url, 600).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if lowered.starts_with("http://")
        || lowered.starts_with("https://")
        || lowered.starts_with("www.")
        || lowered.contains("://")
    {
        return false;
    }
    if !lowered.contains(' ') && !lowered.contains('?') {
        return false;
    }
    if [
        "that was just a test",
        "that was a test",
        "did you do the web request",
        "did you try it",
        "where did that come from",
        "why did my last prompt",
        "you returned no result",
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
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    let urlish_hits = [
        ".com", ".org", ".net", ".io", "site:", "docs", "api.", "www.", "http", "https",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    meta_hits >= 2 && urlish_hits == 0
}
