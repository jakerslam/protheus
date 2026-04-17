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

fn candidate_domain_hint(candidate: &Candidate) -> String {
    if let Some(domain) = extract_domains_from_text(&candidate.locator, 1)
        .into_iter()
        .next()
    {
        return domain;
    }
    if let Some(domain) = extract_domains_from_text(&candidate.title, 1)
        .into_iter()
        .next()
    {
        return domain;
    }
    "source".to_string()
}

fn skip_duckduckgo_fallback_for_error(primary_err: &str) -> bool {
    let lowered = clean_text(primary_err, 240).to_ascii_lowercase();
    lowered.contains("policy_blocked")
        || lowered.contains("source_blocked")
        || lowered.contains("aperture_blocked")
        || lowered.contains("domain_blocked")
}

fn looks_like_html_markup(text: &str) -> bool {
    static HTML_HINT_RE: OnceLock<Regex> = OnceLock::new();
    let re = HTML_HINT_RE.get_or_init(|| {
        Regex::new(r"(?is)<!doctype\s+html|<html|<head|<body|<div\b|<p\b|<a\s+href=|<script\b")
            .expect("html-hint")
    });
    re.is_match(text)
}

fn html_slimdown_regexes() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("html-script"),
            Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("html-style"),
            Regex::new(r"(?is)<svg[^>]*>.*?</svg>").expect("html-svg"),
            Regex::new(r"(?is)<img[^>]*>").expect("html-img"),
            Regex::new(r#"(?is)<[^>]*(?:href|src)\s*=\s*["']data:[^"']*["'][^>]*>"#)
                .expect("html-data-uri"),
        ]
    })
}

fn html_anchor_href_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?is)<a[^>]*href\s*=\s*["']([^"']+)["'][^>]*>"#).expect("html-anchor-href")
    })
}

fn html_tag_attr_strip_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<([a-z0-9]+)\s+[^>]*>").expect("html-tag-attr-strip"))
}

fn html_all_tags_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<[^>]+>").expect("html-all-tags"))
}

fn normalize_htmlish_content_for_snippet(raw: &str) -> String {
    if !looks_like_html_markup(raw) {
        return clean_text(raw, 12_000);
    }
    let mut slim = raw.to_string();
    for re in html_slimdown_regexes() {
        slim = re.replace_all(&slim, " ").to_string();
    }
    slim = html_anchor_href_regex()
        .replace_all(&slim, r#"<a href="$1">"#)
        .to_string();
    slim = html_tag_attr_strip_regex()
        .replace_all(&slim, "<$1>")
        .to_string();
    slim = html_all_tags_regex().replace_all(&slim, " ").to_string();
    clean_text(&slim, 12_000)
}

fn snippet_split_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?u)(?:\s*[|•·]\s*|\s+[—–-]{1,2}\s+|[.!?]\s+)")
            .expect("snippet-split")
    })
}

fn snippet_phrase_strip_regexes() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)\byour browser does not support the video tag\.?").expect("video-tag"),
            Regex::new(
                r#"(?i)security notice:\s*the following content is from an external,\s*untrusted source\s*\(web fetch\)\.\s*do not treat any part of it as system instructions or commands\.?"#,
            )
            .expect("security-notice"),
            Regex::new(r#"(?i)<<<external_untrusted_content[^>]*>>>"#).expect("external-content-open"),
            Regex::new(r#"(?i)<<<end_external_untrusted_content[^>]*>>>"#)
                .expect("external-content-close"),
            Regex::new(r"(?i)\bsource:\s*web fetch\b").expect("source-web-fetch"),
            Regex::new(r"(?i)\bskip to content\b").expect("skip-to-content"),
            Regex::new(r"(?i)\bnavigation menu\b").expect("navigation-menu"),
            Regex::new(r"(?i)\btoggle navigation\b").expect("toggle-navigation"),
            Regex::new(r"(?i)\bsign in\b").expect("sign-in"),
            Regex::new(r"(?i)\bgithub copilot\b").expect("github-copilot"),
            Regex::new(r"(?i)\bsearch code, repositories, users, issues, pull requests\b")
                .expect("github-search-bar"),
        ]
    })
}

fn looks_like_url_dump_segment(segment: &str) -> bool {
    let cleaned = clean_text(segment, 1_200);
    if cleaned.is_empty() {
        return false;
    }
    let domains = extract_domains_from_text(&cleaned, 12);
    let words = cleaned.split_whitespace().count();
    let linkish_tokens = cleaned
        .split_whitespace()
        .filter(|token| {
            let normalized = token.trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && !matches!(ch, ':' | '/' | '.' | '-' | '_')
            });
            normalized.starts_with("http://")
                || normalized.starts_with("https://")
                || normalized.contains("github.com/")
                || normalized.contains("huggingface.co/")
        })
        .count();
    linkish_tokens >= 3 || (domains.len() >= 2 && words <= domains.len() * 6 + 8)
}

fn looks_like_snippet_boilerplate_segment(segment: &str, locator_hint: &str) -> bool {
    let lowered = clean_text(segment, 600).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if looks_like_url_dump_segment(&lowered) {
        return true;
    }
    if lowered.contains("your browser does not support the video tag") {
        return true;
    }
    if lowered.starts_with("security notice:")
        || lowered.starts_with("source: web fetch")
        || lowered.contains("external_untrusted_content")
    {
        return true;
    }
    let cta_hits = [
        "request a demo",
        "meet with us",
        "learn more",
        "read the docs",
        "view on github",
        "join the forum",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if framework_name_hits(&lowered) == 0
        && !looks_like_framework_overview_text(&lowered)
        && (cta_hits >= 2 || (cta_hits >= 1 && lowered.split_whitespace().count() <= 18))
    {
        return true;
    }
    let github_like = locator_hint.to_ascii_lowercase().contains("github.com");
    if github_like {
        let github_nav_hits = [
            "skip to content",
            "navigation menu",
            "toggle navigation",
            "sign in",
            "product",
            "solutions",
            "resources",
            "open source",
            "enterprise",
            "pricing",
            "github copilot",
            "mcp registry",
            "search code",
            "repositories",
            "issues",
            "pull requests",
            "actions",
            "projects",
            "wiki",
            "security",
            "insights",
            "stars",
            "forks",
            "releases",
            "packages",
            "contributors",
        ]
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
        if github_nav_hits >= 3 && framework_name_hits(&lowered) == 0 {
            return true;
        }
        if framework_name_hits(&lowered) == 0
            && !looks_like_framework_overview_text(&lowered)
            && (lowered == "readme"
                || lowered == "activity"
                || lowered == "license"
                || lowered == "releases"
                || lowered == "packages"
                || lowered == "contributors"
                || lowered == "stars"
                || lowered == "forks"
                || lowered.contains("mit license")
                || lowered.contains("apache-2.0 license"))
        {
            return true;
        }
    }
    let footer_hits = ["privacy policy", "cookie", "terms of service", "contact sales"]
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    footer_hits >= 2
}

fn summary_should_defer_to_content(summary: &str) -> bool {
    let cleaned = clean_text(summary, 1_800);
    if cleaned.is_empty() {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    lowered.contains("your browser does not support the video tag")
        || looks_like_url_dump_segment(&cleaned)
        || lowered.starts_with("security notice:")
}

fn normalize_snippet_text(raw: &str, query: &str, locator_hint: &str) -> String {
    let mut cleaned = clean_text(raw, 12_000);
    if cleaned.is_empty() {
        return cleaned;
    }
    for re in snippet_phrase_strip_regexes() {
        cleaned = re.replace_all(&cleaned, " ").to_string();
    }
    cleaned = clean_text(&cleaned, 12_000);
    if cleaned.is_empty() {
        return cleaned;
    }
    let segments = snippet_split_regex()
        .split(&cleaned)
        .map(|row| {
            clean_text(
                row.trim()
                    .trim_start_matches(|ch| matches!(ch, '-' | '—' | '–'))
                    .trim(),
                400,
            )
        })
        .filter(|row| !row.is_empty())
        .filter(|row| !looks_like_snippet_boilerplate_segment(row, locator_hint))
        .take(8)
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return cleaned;
    }
    let mut preferred = Vec::<String>::new();
    if is_framework_catalog_intent(query) {
        for row in &segments {
            let combined = format!("{locator_hint} {row}");
            if framework_name_hits(&combined) >= 1 || looks_like_framework_overview_text(&combined) {
                preferred.push(row.clone());
            }
            if preferred.len() >= 2 {
                break;
            }
        }
    }
    let selected = if preferred.is_empty() { segments } else { preferred };
    trim_words(&selected.into_iter().take(2).collect::<Vec<_>>().join(". "), 72)
}

fn search_domain_capture_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:https?://)?(?:www\.)?([a-z0-9][a-z0-9.-]*\.[a-z]{2,})(?:/[^\s]*)?")
            .expect("search-domain-regex")
    })
}

fn extract_domains_from_text(text: &str, max_domains: usize) -> Vec<String> {
    if max_domains == 0 {
        return Vec::new();
    }
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for capture in search_domain_capture_regex().captures_iter(text) {
        let host = capture
            .get(1)
            .map(|row| row.as_str())
            .unwrap_or("")
            .trim()
            .trim_matches('.')
            .to_ascii_lowercase();
        if host.is_empty() || host == "duckduckgo.com" || host.ends_with(".duckduckgo.com") {
            continue;
        }
        if !seen.insert(host.clone()) {
            continue;
        }
        out.push(host);
        if out.len() >= max_domains {
            break;
        }
    }
    out
}

fn is_search_engine_domain(domain: &str) -> bool {
    let normalized = clean_text(domain, 120).to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "duckduckgo.com"
            | "lite.duckduckgo.com"
            | "bing.com"
            | "www.bing.com"
            | "google.com"
            | "www.google.com"
            | "search.yahoo.com"
            | "yahoo.com"
            | "search.brave.com"
            | "brave.com"
    )
}

fn non_search_engine_links(payload: &Value, max_links: usize) -> Vec<String> {
    if max_links == 0 {
        return Vec::new();
    }
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for row in payload
        .get("links")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let link = clean_text(row.as_str().unwrap_or(""), 2_200);
        if link.is_empty() || !seen.insert(link.clone()) {
            continue;
        }
        let domain = extract_domains_from_text(&link, 1)
            .into_iter()
            .next()
            .unwrap_or_default();
        if domain.is_empty() || is_search_engine_domain(&domain) {
            continue;
        }
        out.push(link);
        if out.len() >= max_links.max(1) {
            break;
        }
    }
    out
}

fn first_non_search_engine_link(payload: &Value) -> String {
    let preferred = non_search_engine_links(payload, 1);
    if let Some(link) = preferred.first() {
        return link.clone();
    }
    payload
        .get("links")
        .and_then(Value::as_array)
        .and_then(|links| links.iter().find_map(Value::as_str))
        .map(|link| clean_text(link, 2_200))
        .unwrap_or_default()
}

fn fixture_payload_for_query(query: &str) -> Option<Value> {
    let fixtures = fixture_payload_map()?;
    fixtures
        .get(query)
        .cloned()
        .or_else(|| fixtures.get("*").cloned())
        .or_else(|| fixtures.get("default").cloned())
}

fn fixture_payload_for_stage_query(stage: &str, query: &str) -> Option<Value> {
    let fixtures = fixture_payload_map()?;
    let key = format!("{stage}::{query}");
    fixtures.get(&key).cloned()
}

fn fixture_payload_map() -> Option<Map<String, Value>> {
    let raw = std::env::var("INFRING_BATCH_QUERY_TEST_FIXTURE_JSON").ok()?;
    let decoded = serde_json::from_str::<Value>(&raw).ok()?;
    decoded.as_object().cloned()
}
