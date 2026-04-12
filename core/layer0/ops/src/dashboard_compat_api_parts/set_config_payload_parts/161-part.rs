fn leading_quote_pair(raw: &str) -> Option<(char, char)> {
    let first = raw.chars().next()?;
    match first {
        '"' => Some(('"', '"')),
        '\'' => Some(('\'', '\'')),
        '`' => Some(('`', '`')),
        '“' => Some(('“', '”')),
        _ => None,
    }
}

fn trailing_web_query_instruction_tail(raw: &str) -> bool {
    let lowered = clean_text(raw, 240)
        .trim()
        .trim_matches(|ch| matches!(ch, '"' | '\'' | '`' | '“' | '”'))
        .trim()
        .to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    [
        "and return the results",
        "and return results",
        "and return the result",
        "and return the answer",
        "and return the findings",
        "and give me the results",
        "and give the results",
        "and show me the results",
        "and tell me the results",
        "and tell me what you find",
        "and tell me what you found",
        "and summarize the results",
        "and summarize the answer",
        "and summarize",
    ]
    .iter()
    .any(|suffix| lowered.starts_with(suffix))
}

fn extract_leading_quoted_natural_web_query(text: &str, max_chars: usize) -> Option<String> {
    let trimmed = clean_text(text, max_chars);
    let trimmed = trimmed.trim();
    let (_, close) = leading_quote_pair(trimmed)?;
    let rest = &trimmed[trimmed.chars().next()?.len_utf8()..];
    let end_rel = rest.find(close)?;
    let inside = clean_text(&rest[..end_rel], max_chars);
    if inside.is_empty() {
        return None;
    }
    if trailing_web_query_instruction_tail(&rest[end_rel + close.len_utf8()..]) {
        return Some(inside);
    }
    None
}

fn strip_wrapped_natural_web_query(text: &str, max_chars: usize) -> String {
    let mut cleaned = clean_text(text, max_chars);
    if cleaned.is_empty() {
        return cleaned;
    }
    if let Some(quoted) = extract_leading_quoted_natural_web_query(&cleaned, max_chars) {
        cleaned = quoted;
    }
    cleaned = cleaned
        .trim()
        .trim_matches(|ch| matches!(ch, '"' | '\'' | '`' | '“' | '”'))
        .trim()
        .to_string();
    loop {
        let lowered = cleaned.to_ascii_lowercase();
        let mut stripped = false;
        for suffix in [
            " and return the results",
            " and return results",
            " and return the result",
            " and return the answer",
            " and return the findings",
            " and give me the results",
            " and give the results",
            " and show me the results",
            " and tell me the results",
            " and tell me what you find",
            " and tell me what you found",
            " and summarize the results",
            " and summarize the answer",
            " and summarize",
        ] {
            if lowered.ends_with(suffix) && cleaned.len() > suffix.len() {
                cleaned = clean_text(&cleaned[..cleaned.len() - suffix.len()], max_chars);
                stripped = true;
                break;
            }
        }
        if stripped {
            cleaned = cleaned.trim().to_string();
            continue;
        }
        if matches!(cleaned.chars().last(), Some('.' | '!' | '?' | ';' | ':')) {
            cleaned.pop();
            cleaned = cleaned.trim_end().to_string();
            continue;
        }
        break;
    }
    clean_text(&cleaned, max_chars)
}
