fn tool_error_text(payload: &Value) -> String {
    clean_text(
        payload
            .get("error")
            .or_else(|| payload.get("message"))
            .or_else(|| payload.pointer("/result/error"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    )
}

fn looks_like_domain_token(value: &str) -> bool {
    if value.is_empty() || !value.contains('.') {
        return false;
    }
    if value.starts_with('.') || value.ends_with('.') {
        return false;
    }
    if value
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-')))
    {
        return false;
    }
    let Some(tld) = value.rsplit('.').next() else {
        return false;
    };
    (2..=24).contains(&tld.len())
}

fn extract_search_result_domains(summary: &str, max_domains: usize) -> Vec<String> {
    let mut domains = Vec::<String>::new();
    for token in clean_text(summary, 4_000).split_whitespace() {
        let stripped = token
            .trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-' && ch != '/'
            })
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_start_matches("www.");
        let host = stripped
            .split('/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if !looks_like_domain_token(&host) {
            continue;
        }
        if host == "duckduckgo.com" {
            continue;
        }
        if domains.iter().any(|existing| existing == &host) {
            continue;
        }
        domains.push(host);
        if domains.len() >= max_domains.max(1) {
            break;
        }
    }
    domains
}

fn web_search_no_findings_fallback(
    query: &str,
    combined: &str,
    requested_url: &str,
    domain: &str,
) -> String {
    let query_label = if query.is_empty() {
        "this query".to_string()
    } else {
        format!("\"{}\"", trim_text(query, 120))
    };
    let source = if domain.trim().is_empty() {
        source_label_from_url(requested_url)
    } else {
        clean_text(domain, 120)
    };
    let lowered = clean_text(combined, 4_000).to_ascii_lowercase();
    let search_chrome_like = looks_like_search_engine_chrome_summary(&lowered)
        || lowered.contains("all regions ")
        || lowered.contains("safe search")
        || lowered.contains("any time")
        || lowered.contains(" at duckduckgo");
    if search_chrome_like {
        if source.is_empty() {
            return format!(
                "Web search for {} returned low-signal search-engine chrome with no extractable findings. This is a retrieval/parsing miss, not a confirmed no-answer. Retry with `batch_query` or provide one specific source URL.",
                query_label
            );
        }
        return format!(
            "Web search for {} returned low-signal search-engine chrome from {} with no extractable findings. This is a retrieval/parsing miss, not a confirmed no-answer. Retry with `batch_query` or provide one specific source URL.",
            query_label,
            trim_text(&source, 120)
        );
    }
    if source.is_empty() {
        return format!(
            "Web search for {} completed but produced no extractable findings. Retry with a narrower query or ask for a provisional answer without live sources.",
            query_label
        );
    }
    format!(
        "Web search for {} completed but produced no extractable findings from {}. Retry with a narrower query or ask for a provisional answer without live sources.",
        query_label,
        trim_text(&source, 120)
    )
}

fn extract_search_result_findings(summary: &str, max_items: usize) -> Vec<String> {
    if max_items == 0 {
        return Vec::new();
    }
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let normalized = clean_text(summary, 6_000);
    for line in normalized
        .split(|ch| matches!(ch, '\n' | '|' | '•'))
        .map(|row| clean_text(row, 280))
    {
        if line.is_empty() {
            continue;
        }
        if looks_like_search_engine_chrome_summary(&line) {
            continue;
        }
        let lowered = line.to_ascii_lowercase();
        if lowered.contains("duckduckgo all regions")
            || lowered.starts_with("all regions ")
            || lowered.starts_with("safe search ")
            || lowered.contains(" at duckduckgo")
            || lowered.contains("site links")
            || lowered.contains("key findings for")
            || lowered.contains("potential sources:")
        {
            continue;
        }
        if lowered.contains(" at ") && lowered.contains("duckduckgo") {
            continue;
        }
        if lowered.starts_with("bing.com:")
            || lowered.starts_with("duckduckgo.com:")
            || lowered.starts_with("google.com:")
            || lowered.starts_with("www.bing.com:")
            || lowered.starts_with("www.duckduckgo.com:")
            || lowered.starts_with("www.google.com:")
        {
            continue;
        }
        if let Some((prefix, _)) = lowered.split_once(':') {
            let domain_prefix = prefix.trim().trim_start_matches("www.");
            if looks_like_domain_token(domain_prefix) {
                continue;
            }
        }
        let has_link_hint = lowered.contains("http://")
            || lowered.contains("https://")
            || lowered.contains(".org/")
            || lowered.contains(".com/")
            || lowered.contains(".ai/")
            || lowered.contains(".dev/");
        if lowered.contains("...") && lowered.contains("all regions") {
            continue;
        }
        if !has_link_hint && line.len() < 44 {
            continue;
        }
        let compact = trim_text(&line.replace('\t', " ").replace("  ", " "), 240);
        if compact.is_empty() {
            continue;
        }
        let key = compact.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        out.push(compact);
        if out.len() >= max_items {
            break;
        }
    }
    out
}

fn looks_like_placeholder_fetch_content(text: &str, requested_url: &str) -> bool {
    let lowered = clean_text(text, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let requested = clean_text(requested_url, 400).to_ascii_lowercase();
    if requested.contains("example.com") {
        return true;
    }
    lowered.contains("example domain")
        && lowered.contains("for use in documentation examples")
        && lowered.contains("without needing permission")
}

fn looks_like_navigation_chrome_payload(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let marker_count = [
        "skip to content",
        "home",
        "news",
        "sport",
        "business",
        "technology",
        "health",
        "culture",
        "travel",
        "audio",
        "video",
        "live",
        "all regions",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    marker_count >= 5 && lowered.split_whitespace().count() >= 14
}

fn source_label_from_url(raw: &str) -> String {
    let cleaned = clean_text(raw, 2200);
    if cleaned.is_empty() {
        return String::new();
    }
    if let Some(rest) = cleaned
        .strip_prefix("https://")
        .or_else(|| cleaned.strip_prefix("http://"))
    {
        return clean_text(rest.split('/').next().unwrap_or(""), 200);
    }
    clean_text(cleaned.split('/').next().unwrap_or(""), 200)
}

fn summarize_web_fetch_payload(payload: &Value) -> String {
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return user_facing_tool_failure_summary("web_fetch", payload)
            .unwrap_or_else(|| "Web fetch couldn't complete right now.".to_string());
    }
    let requested_url = clean_text(
        payload
            .get("requested_url")
            .or_else(|| payload.pointer("/receipt/requested_url"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    );
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        4_000,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        4_000,
    );
    let body = if summary.is_empty() {
        content.clone()
    } else {
        summary.clone()
    };
    if body.is_empty() {
        if requested_url.is_empty() {
            return "I fetched the page, but it returned no readable content.".to_string();
        }
        return format!(
            "I fetched {}, but it returned no readable content.",
            trim_text(&requested_url, 220)
        );
    }
    if looks_like_placeholder_fetch_content(&body, &requested_url) {
        return "The fetched page is placeholder/test content (for example, `example.com`), so it doesn't provide real findings. Ask me to run a web search query or fetch a specific real source URL.".to_string();
    }
    if looks_like_navigation_chrome_payload(&body) || looks_like_search_engine_chrome_summary(&body)
    {
        let source = source_label_from_url(&requested_url);
        if !source.is_empty() {
            return format!(
                "I fetched {}, but the response was mostly page navigation/chrome instead of answer-ready findings. Ask me to run `batch_query` or `web_search` for your question.",
                trim_text(&source, 120)
            );
        }
        return "I fetched the page, but the response was mostly navigation/chrome instead of answer-ready findings. Ask me to run `batch_query` or `web_search` for your question.".to_string();
    }
    let snippet = first_sentence(&body, 320);
    if snippet.is_empty() {
        return "I fetched the page, but couldn't extract a reliable summary sentence from it yet."
            .to_string();
    }
    let source = source_label_from_url(&requested_url);
    if source.is_empty() {
        snippet
    } else {
        format!("From {}: {}", trim_text(&source, 120), snippet)
    }
}

fn looks_like_search_engine_chrome_summary(summary: &str) -> bool {
    let lowered = summary.to_ascii_lowercase();
    let potential_source_mentions = lowered.matches("potential sources:").count();
    if lowered.contains("unfortunately, bots use duckduckgo too")
        || lowered.contains("please complete the following challenge")
        || lowered.contains("select all squares containing a duck")
        || lowered.contains("error-lite@duckduckgo.com")
    {
        return true;
    }
    if lowered.contains("key findings for") && potential_source_mentions >= 1 {
        return true;
    }
    if potential_source_mentions >= 1
        && !lowered.contains("http://")
        && !lowered.contains("https://")
    {
        return true;
    }
    if lowered.contains("key findings for")
        && !lowered.contains("http://")
        && !lowered.contains("https://")
    {
        return true;
    }
    let markers = [
        "duckduckgo all regions",
        "all regions argentina",
        "all regions australia",
        "all regions canada",
        "safe search",
        "any time",
    ];
    let hits = markers
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    hits >= 2
}

fn user_facing_tool_failure_summary(tool_name: &str, payload: &Value) -> Option<String> {
    let normalized = normalize_tool_name(tool_name);
    let lowered = tool_error_text(payload).to_ascii_lowercase();
    if lowered.contains("unsupported_tool_command")
        || lowered.contains("tool_command_")
        || lowered == "invalid_tool_command"
    {
        let message = clean_text(payload.get("message").and_then(Value::as_str).unwrap_or(""), 320);
        if !message.is_empty() {
            return Some(message);
        }
    }
    if lowered.is_empty() {
        if normalized == "system_diagnostic" {
            return Some(
                "`system_diagnostic` couldn't run in this turn. I can still diagnose manually from the latest prompt/response and runtime symptoms if you want me to continue."
                    .to_string(),
            );
        }
        return Some(format!("I couldn't complete `{normalized}` right now."));
    }
    if lowered == "tool_explicit_signoff_required" || lowered == "tool_confirmation_required" {
        return Some(format!(
            "I need your confirmation before running `{normalized}`. Reply `yes` to execute it now."
        ));
    }
    if lowered.contains("query_required") {
        return Some(format!("`{normalized}` needs a query before it can run."));
    }
    if lowered.contains("url_required") {
        return Some(format!(
            "`{normalized}` needs a valid URL before it can run."
        ));
    }
    if normalized == "file_read" || normalized == "read_file" || normalized == "file" {
        if lowered.contains("path_required") {
            return Some("I need a workspace file path before I can read it.".to_string());
        }
        if lowered.contains("path_outside_workspace") {
            return Some(
                "That path is outside the active workspace. Give me a workspace-relative file path."
                    .to_string(),
            );
        }
        if lowered.contains("file_not_found") {
            return Some("I couldn't find that file in the active workspace.".to_string());
        }
        if lowered.contains("binary_file_requires_opt_in") {
            return Some(
                "That file is binary. Re-run `file_read` with `allow_binary=true` if you want base64 output."
                    .to_string(),
            );
        }
    }
    if normalized == "file_read_many"
        || normalized == "read_files"
        || normalized == "files_read"
        || normalized == "batch_file_read"
    {
        if lowered.contains("paths_required") || lowered.contains("path_required") {
            return Some(
                "I need one or more workspace file paths before batch read can run.".to_string(),
            );
        }
        if lowered.contains("path_outside_workspace") {
            return Some(
                "One or more paths were outside the active workspace. Provide workspace-relative file paths."
                    .to_string(),
            );
        }
    }
    if normalized == "system_diagnostic" {
        return Some(
            "`system_diagnostic` couldn't run in this turn. I can still diagnose manually from the latest prompt/response and runtime symptoms if you want me to continue."
                .to_string(),
        );
    }
    if lowered.contains("denied_domain")
        || lowered.contains("network_policy")
        || lowered.contains("domain_blocked")
    {
        return Some(format!(
            "`{normalized}` was blocked by network policy for this request."
        ));
    }
    if lowered.contains("request_read_failed")
        || lowered.contains("resource temporarily unavailable")
        || lowered.contains("os error 35")
    {
        return Some(format!(
            "`{normalized}` hit temporary runtime I/O pressure (`request_read_failed`). I already retry transient failures automatically; retry once, then run `infringctl doctor --json` if it persists."
        ));
    }
    if lowered.contains("timeout")
        || lowered.contains("timed out")
        || lowered.contains("unavailable")
        || lowered.contains("connection")
    {
        return Some(format!(
            "`{normalized}` hit a temporary network/runtime issue. Retry once; if it repeats, run `infringctl doctor --json`."
        ));
    }
    Some(format!("I couldn't complete `{normalized}` right now."))
}

fn transient_tool_failure(payload: &Value) -> bool {
    if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return false;
    }
    let lowered = tool_error_text(payload).to_ascii_lowercase();
    lowered.contains("aborted")
        || lowered.contains("timeout")
        || lowered.contains("timed out")
        || lowered.contains("temporar")
        || lowered.contains("unavailable")
        || lowered.contains("network")
        || lowered.contains("connection")
        || lowered.contains("retry")
        || lowered.contains("econnreset")
        || lowered.contains("request_read_failed")
        || lowered.contains("resource temporarily unavailable")
        || lowered.contains("os error 35")
}

