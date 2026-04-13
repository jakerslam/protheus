fn search_row_url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"https?://[^\s)]+").expect("search-row-url"))
}

fn trim_search_row_segment(raw: &str) -> String {
    clean_text(raw, 600)
        .trim()
        .trim_matches(|ch| matches!(ch, '—' | '-' | '|' | ':' | ';'))
        .trim()
        .to_string()
}

fn candidate_from_rendered_search_row(
    _query: &str,
    row: &str,
    status_code: i64,
) -> Option<Candidate> {
    let cleaned = clean_text(row, 1_400);
    if cleaned.is_empty()
        || looks_like_ack_only(&cleaned)
        || looks_like_low_signal_search_summary(&cleaned)
        || looks_like_source_only_snippet(&cleaned)
    {
        return None;
    }
    let url_match = search_row_url_regex().find(&cleaned)?;
    let locator = clean_text(url_match.as_str(), 2_200);
    let domain = extract_domains_from_text(&locator, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    if domain.is_empty() || is_search_engine_domain(&domain) {
        return None;
    }
    let prefix = trim_search_row_segment(&cleaned[..url_match.start()]);
    let suffix = trim_search_row_segment(&cleaned[url_match.end()..]);
    let title = if prefix.is_empty() {
        format!("Web result from {}", clean_text(&domain, 120))
    } else {
        prefix
    };
    let snippet = normalize_htmlish_content_for_snippet(&suffix);
    if snippet.is_empty() {
        return None;
    }
    let excerpt_seed = format!("{} {}", title, snippet);
    Some(Candidate {
        source_kind: "web".to_string(),
        title,
        locator,
        snippet: snippet.clone(),
        excerpt_hash: sha256_hex(&excerpt_seed),
        timestamp: Some(crate::now_iso()),
        permissions: Some("public_web".to_string()),
        status_code,
    })
}

fn candidates_from_rendered_search_payload(
    query: &str,
    payload: &Value,
    max_rows: usize,
) -> Vec<Candidate> {
    if max_rows == 0 {
        return Vec::new();
    }
    let raw_content = payload.get("content").and_then(Value::as_str).unwrap_or("");
    if raw_content.trim().is_empty() {
        return Vec::new();
    }
    let status_code = payload
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let mut out = Vec::<Candidate>::new();
    let mut seen = HashSet::<String>::new();
    for row in raw_content.lines() {
        let Some(candidate) = candidate_from_rendered_search_row(query, row, status_code) else {
            continue;
        };
        let key = format!(
            "{}|{}|{}",
            candidate.locator.to_ascii_lowercase(),
            candidate.title.to_ascii_lowercase(),
            candidate.excerpt_hash
        );
        if !seen.insert(key) {
            continue;
        }
        out.push(candidate);
        if out.len() >= max_rows {
            break;
        }
    }
    out
}
