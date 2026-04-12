fn regex_noscript() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<noscript[^>]*>.*?</noscript>").expect("regex"))
}

fn regex_title() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<title[^>]*>(.*?)</title>").expect("regex"))
}

fn regex_anchor() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?is)<a\s+[^>]*href=["']([^"'#]+[^"']*)["'][^>]*>(.*?)</a>"#)
            .expect("regex")
    })
}

fn regex_heading() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<h([1-6])[^>]*>(.*?)</h[1-6]>").expect("regex"))
}

fn regex_list_item() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<li[^>]*>(.*?)</li>").expect("regex"))
}

fn regex_breaks() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<(br|hr)\s*/?>").expect("regex"))
}

fn regex_block_endings() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?is)</(p|div|section|article|header|footer|table|tr|ul|ol)>")
            .expect("regex")
    })
}

fn regex_markdown_images() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"!\[[^\]]*]\([^)]+\)").expect("regex"))
}

fn regex_markdown_links() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\[([^\]]+)]\([^)]+\)").expect("regex"))
}

fn regex_fenced_code() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?s)```.*?```").expect("regex"))
}

fn regex_inline_code() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"`([^`]+)`").expect("regex"))
}

fn regex_markdown_heading_prefix() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?m)^#{1,6}\s+").expect("regex"))
}

fn regex_markdown_bullets() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?m)^\s*[-*+]\s+").expect("regex"))
}

fn regex_markdown_numbers() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?m)^\s*\d+\.\s+").expect("regex"))
}

fn normalize_block_text(raw: &str) -> String {
    strip_invisible_unicode(raw)
        .replace('\r', "")
        .replace(" \n", "\n")
        .replace("\t", " ")
        .split('\n')
        .map(|row| row.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("\n\n\n", "\n\n")
        .trim()
        .to_string()
}

fn decode_html_entities(raw: &str) -> String {
    raw.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn strip_tags_to_text(raw: &str) -> String {
    decode_html_entities(&regex_tags().replace_all(raw, " "))
}

fn html_title(raw_html: &str) -> Option<String> {
    regex_title()
        .captures(raw_html)
        .and_then(|captures| captures.get(1).map(|m| m.as_str().to_string()))
        .map(|title| normalize_block_text(&strip_tags_to_text(&title)))
        .filter(|title| !title.is_empty())
}

fn html_to_markdown_document(raw_html: &str) -> (String, Option<String>) {
    let title = html_title(raw_html);
    let readable_html = select_readable_html_body(raw_html);
    let mut text = regex_script().replace_all(&readable_html, " ").to_string();
    text = regex_style().replace_all(&text, " ").to_string();
    text = regex_noscript().replace_all(&text, " ").to_string();
    text = regex_anchor()
        .replace_all(&text, |captures: &regex::Captures| {
            let href = clean_text(captures.get(1).map(|m| m.as_str()).unwrap_or(""), 2200);
            let body = normalize_block_text(&strip_tags_to_text(
                captures.get(2).map(|m| m.as_str()).unwrap_or(""),
            ));
            if body.is_empty() {
                href
            } else {
                format!("[{body}]({href})")
            }
        })
        .to_string();
    text = regex_heading()
        .replace_all(&text, |captures: &regex::Captures| {
            let level = captures
                .get(1)
                .and_then(|m| m.as_str().parse::<usize>().ok())
                .unwrap_or(1)
                .clamp(1, 6);
            let body = normalize_block_text(&strip_tags_to_text(
                captures.get(2).map(|m| m.as_str()).unwrap_or(""),
            ));
            if body.is_empty() {
                String::new()
            } else {
                format!("\n{} {}\n", "#".repeat(level), body)
            }
        })
        .to_string();
    text = regex_list_item()
        .replace_all(&text, |captures: &regex::Captures| {
            let body = normalize_block_text(&strip_tags_to_text(
                captures.get(1).map(|m| m.as_str()).unwrap_or(""),
            ));
            if body.is_empty() {
                String::new()
            } else {
                format!("\n- {body}")
            }
        })
        .to_string();
    text = regex_breaks().replace_all(&text, "\n").to_string();
    text = regex_block_endings().replace_all(&text, "\n").to_string();
    let markdown = normalize_block_text(&strip_tags_to_text(&text));
    if markdown.is_empty() {
        let title_markdown = title
            .as_ref()
            .map(|value| normalize_block_text(value))
            .filter(|value| !value.is_empty())
            .map(|value| format!("# {value}"));
        (title_markdown.unwrap_or_default(), title)
    } else {
        (markdown, title)
    }
}

fn markdown_to_text_document(markdown: &str) -> String {
    let mut text = regex_markdown_images().replace_all(markdown, "").to_string();
    text = regex_markdown_links().replace_all(&text, "$1").to_string();
    text = regex_fenced_code()
        .replace_all(&text, |captures: &regex::Captures| {
            captures
                .get(0)
                .map(|m| {
                    m.as_str()
                        .replace("```text", "")
                        .replace("```markdown", "")
                        .replace("```", "")
                })
                .unwrap_or_default()
        })
        .to_string();
    text = regex_inline_code().replace_all(&text, "$1").to_string();
    text = regex_markdown_heading_prefix().replace_all(&text, "").to_string();
    text = regex_markdown_bullets().replace_all(&text, "").to_string();
    text = regex_markdown_numbers().replace_all(&text, "").to_string();
    normalize_block_text(&text)
}

fn truncate_chars(raw: &str, max_chars: usize) -> (String, bool) {
    if raw.chars().count() <= max_chars {
        return (raw.to_string(), false);
    }
    (
        raw.chars().take(max_chars.max(1)).collect::<String>(),
        true,
    )
}

fn looks_like_html_document(raw: &str, content_type: &str) -> bool {
    let lowered = clean_text(content_type, 120).to_ascii_lowercase();
    if lowered.contains("html") {
        return true;
    }
    let trimmed = raw.trim_start().to_ascii_lowercase();
    trimmed.starts_with("<!doctype html") || trimmed.starts_with("<html")
}

fn extract_fetch_content(
    raw_body: &str,
    content_type: &str,
    extract_mode: &str,
    max_chars: usize,
) -> (String, Option<String>, bool) {
    let body = strip_invisible_unicode(raw_body);
    if body.trim().is_empty() {
        return (String::new(), None, false);
    }
    if looks_like_html_document(&body, content_type) {
        let (markdown, title) = html_to_markdown_document(&body);
        let rendered = if extract_mode == "markdown" {
            markdown
        } else {
            markdown_to_text_document(&markdown)
        };
        let (text, truncated) = truncate_chars(&rendered, max_chars);
        return (text, title, truncated);
    }
    let normalized = normalize_block_text(&body);
    let (text, truncated) = truncate_chars(&normalized, max_chars);
    (text, None, truncated)
}

fn extract_fetch_summary_body(content: &str, extract_mode: &str) -> String {
    if extract_mode == "markdown" {
        markdown_to_text_document(content)
    } else {
        normalize_block_text(content)
    }
}

fn percent_decode_urlish(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = String::new();
    let mut idx = 0usize;
    while idx < bytes.len() {
        if bytes[idx] == b'%' && idx + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&raw[idx + 1..idx + 3], 16) {
                out.push(value as char);
                idx += 3;
                continue;
            }
        }
        if bytes[idx] == b'+' {
            out.push(' ');
        } else {
            out.push(bytes[idx] as char);
        }
        idx += 1;
    }
    out
}

fn query_param_value(raw_url: &str, allowed_keys: &[&str]) -> Option<String> {
    let (_, query) = raw_url.split_once('?')?;
    for pair in query.split('&') {
        let mut chunks = pair.splitn(2, '=');
        let key = chunks.next().unwrap_or_default().trim();
        let value = chunks.next().unwrap_or_default().trim();
        if !allowed_keys.iter().any(|candidate| *candidate == key) {
            continue;
        }
        let decoded = percent_decode_urlish(value);
        if decoded.starts_with("http://") || decoded.starts_with("https://") {
            return Some(clean_text(&decoded, 2200));
        }
    }
    None
}

fn citation_redirect_host(raw_url: &str) -> bool {
    matches!(
        extract_domain(raw_url).as_str(),
        "duckduckgo.com" | "google.com" | "www.google.com" | "news.google.com"
    )
}

fn decode_citation_redirect_url(raw_url: &str) -> Option<String> {
    let domain = extract_domain(raw_url);
    match domain.as_str() {
        "duckduckgo.com" => query_param_value(raw_url, &["uddg", "u", "url"]),
        "google.com" | "www.google.com" | "news.google.com" => {
            query_param_value(raw_url, &["url", "q", "u"])
        }
        _ => None,
    }
}

fn resolve_redirect_with_head(raw_url: &str, timeout_ms: u64) -> Option<String> {
    if !citation_redirect_host(raw_url) {
        return None;
    }
    let timeout_sec = ((timeout_ms as f64) / 1000.0).ceil() as u64;
    let output = Command::new("curl")
        .arg("-sS")
        .arg("-I")
        .arg("-L")
        .arg("--proto")
        .arg("=http,https")
        .arg("--connect-timeout")
        .arg(timeout_sec.max(1).to_string())
        .arg("--max-time")
        .arg(timeout_sec.max(1).to_string())
        .arg("-w")
        .arg("\n__EFFECTIVE_URL__:%{url_effective}")
        .arg(raw_url)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let (_, effective) = stdout.rsplit_once("\n__EFFECTIVE_URL__:")?;
    let candidate = clean_text(effective, 2200);
    if candidate.starts_with("http://") || candidate.starts_with("https://") {
        Some(candidate)
    } else {
        None
    }
}

fn resolve_citation_redirect_url(raw_url: &str, timeout_ms: u64) -> (String, bool) {
    if let Some(decoded) = decode_citation_redirect_url(raw_url) {
        return (decoded, true);
    }
    if let Some(resolved) = resolve_redirect_with_head(raw_url, timeout_ms) {
        return (resolved, true);
    }
    (clean_text(raw_url, 2200), false)
}

fn normalize_search_result_link(raw_url: &str) -> String {
    decode_citation_redirect_url(raw_url).unwrap_or_else(|| clean_text(raw_url, 2200))
}
