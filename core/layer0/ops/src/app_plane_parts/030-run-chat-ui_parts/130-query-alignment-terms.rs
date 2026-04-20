fn chat_ui_query_alignment_terms(text: &str, max_terms: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for token in clean(text, 2_000)
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
