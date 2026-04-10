fn render_serper_payload(
    body: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    max_response_bytes: usize,
) -> Value {
    let parsed = match serde_json::from_str::<Value>(body) {
        Ok(value) => value,
        Err(_) => {
            return json!({
                "ok": false,
                "error": "serper_decode_failed",
                "summary": "",
                "content": "",
                "links": [],
                "content_domains": [],
                "provider_raw_count": 0,
                "provider_filtered_count": 0
            });
        }
    };
    let organic = parsed
        .get("organic")
        .or_else(|| parsed.get("results"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut lines = Vec::<String>::new();
    let mut links = Vec::<String>::new();
    let mut domains = Vec::<String>::new();
    for row in &organic {
        let link = clean_text(row.get("link").and_then(Value::as_str).unwrap_or(""), 2200);
        if link.is_empty() || !domain_allowed_for_scope(&link, allowed_domains, exclude_subdomains)
        {
            continue;
        }
        let rendered = render_search_row(
            row.get("title").and_then(Value::as_str).unwrap_or(""),
            row.get("snippet").and_then(Value::as_str).unwrap_or(""),
            &link,
        );
        if rendered.is_empty() {
            continue;
        }
        lines.push(rendered);
        links.push(link.clone());
        push_unique_link_domain(&mut domains, &link);
        if lines.len() >= top_k.max(1) {
            break;
        }
    }
    let content = clean_text(&lines.join("\n"), max_response_bytes.min(120_000));
    let ok = !content.is_empty();
    json!({
        "ok": ok,
        "summary": if ok {
            summarize_text(&content, 900)
        } else {
            crate::tool_output_match_filter::no_findings_user_copy().to_string()
        },
        "content": content,
        "links": links,
        "content_domains": domains,
        "provider_raw_count": organic.len(),
        "provider_filtered_count": lines.len(),
        "error": if ok {
            Value::Null
        } else {
            Value::String("no_relevant_results".to_string())
        }
    })
}

fn decode_xml_entities(raw: &str) -> String {
    raw.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn extract_xml_tag_value(block: &str, tag: &str) -> String {
    let pattern = format!(r"(?is)<{tag}[^>]*>(.*?)</{tag}>");
    let Ok(re) = Regex::new(&pattern) else {
        return String::new();
    };
    let Some(captures) = re.captures(block) else {
        return String::new();
    };
    let raw = captures.get(1).map(|m| m.as_str()).unwrap_or("");
    let trimmed = raw
        .trim()
        .trim_start_matches("<![CDATA[")
        .trim_end_matches("]]>");
    clean_html_content(&decode_xml_entities(trimmed), 2_400)
}

fn render_bing_rss_payload(
    body: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    max_response_bytes: usize,
) -> Value {
    static ITEM_RE: OnceLock<Regex> = OnceLock::new();
    let item_re =
        ITEM_RE.get_or_init(|| Regex::new(r"(?is)<item\b[^>]*>(.*?)</item>").expect("item regex"));
    let mut lines = Vec::<String>::new();
    let mut links = Vec::<String>::new();
    let mut domains = Vec::<String>::new();
    let mut raw_count = 0usize;
    for captures in item_re.captures_iter(body) {
        raw_count += 1;
        let item = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        let link = clean_text(&extract_xml_tag_value(item, "link"), 2_200);
        if link.is_empty() || !domain_allowed_for_scope(&link, allowed_domains, exclude_subdomains)
        {
            continue;
        }
        let rendered = render_search_row(
            &extract_xml_tag_value(item, "title"),
            &extract_xml_tag_value(item, "description"),
            &link,
        );
        if rendered.is_empty() {
            continue;
        }
        lines.push(rendered);
        links.push(link.clone());
        push_unique_link_domain(&mut domains, &link);
        if lines.len() >= top_k.max(1) {
            break;
        }
    }
    let content = clean_text(&lines.join("\n"), max_response_bytes.min(120_000));
    let ok = !content.is_empty();
    json!({
        "ok": ok,
        "summary": if ok {
            summarize_text(&content, 900)
        } else {
            crate::tool_output_match_filter::no_findings_user_copy().to_string()
        },
        "content": content,
        "links": links,
        "content_domains": domains,
        "provider_raw_count": raw_count,
        "provider_filtered_count": lines.len(),
        "error": if ok {
            Value::Null
        } else {
            Value::String("no_relevant_results".to_string())
        }
    })
}

fn looks_like_search_challenge_payload(summary: &str, content: &str) -> bool {
    let combined = format!("{summary}\n{content}").to_ascii_lowercase();
    if combined.is_empty() {
        return false;
    }
    [
        "unfortunately, bots use duckduckgo too",
        "please complete the following challenge",
        "select all squares containing a duck",
        "anomaly-modal",
        "images not loading?",
        "error-lite@duckduckgo.com",
    ]
    .iter()
    .any(|marker| combined.contains(marker))
}

fn payload_looks_like_search_challenge(payload: &Value) -> bool {
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        4_000,
    );
    looks_like_search_challenge_payload(&summary, &content)
}

fn looks_like_low_signal_search_payload(summary: &str, content: &str) -> bool {
    let lowered = clean_text(&format!("{summary}\n{content}"), 6_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if looks_like_search_challenge_payload(summary, content) {
        return true;
    }
    if lowered.contains("key findings for") && lowered.contains("potential sources:") {
        return true;
    }
    let marker_hits = [
        "duckduckgo all regions",
        "all regions argentina",
        "all regions australia",
        "all regions canada",
        "safe search",
        "any time",
        "at duckduckgo",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    marker_hits >= 2
}

fn payload_looks_low_signal_search(payload: &Value) -> bool {
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        4_000,
    );
    looks_like_low_signal_search_payload(&summary, &content)
}

fn fetch_with_curl(
    url: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    user_agent: &str,
) -> Value {
    let timeout_sec = ((timeout_ms as f64) / 1000.0).ceil() as u64;
    let output = Command::new("curl")
        .arg("-sS")
        .arg("-L")
        .arg("--compressed")
        .arg("--proto")
        .arg("=http,https")
        .arg("--connect-timeout")
        .arg(timeout_sec.max(1).to_string())
        .arg("--max-time")
        .arg(timeout_sec.max(1).to_string())
        .arg("-A")
        .arg(clean_text(user_agent, 260))
        .arg("-H")
        .arg(format!("Accept-Language: {DEFAULT_ACCEPT_LANGUAGE}"))
        .arg("-e")
        .arg(DEFAULT_REFERER)
        .arg("-w")
        .arg("\n__STATUS__:%{http_code}\n__CTYPE__:%{content_type}")
        .arg(url)
        .output();

    match output {
        Ok(run) => {
            let stdout = String::from_utf8_lossy(&run.stdout).to_string();
            let stderr = clean_text(&String::from_utf8_lossy(&run.stderr), 320);
            let status_marker = "\n__STATUS__:";
            let ctype_marker = "\n__CTYPE__:";
            let (body_and_status, content_type) = match stdout.rsplit_once(ctype_marker) {
                Some((left, right)) => (left.to_string(), clean_text(right, 120)),
                None => (stdout, String::new()),
            };
            let (body_raw, status_raw) = match body_and_status.rsplit_once(status_marker) {
                Some((left, right)) => (left.to_string(), clean_text(right, 12)),
                None => (body_and_status, "0".to_string()),
            };
            let status_code = status_raw.parse::<i64>().unwrap_or(0);
            let body = clip_bytes(&body_raw, max_response_bytes.max(256));
            let status_ok = (200..400).contains(&status_code);
            json!({
                "ok": run.status.success() && status_ok,
                "status_code": status_code,
                "content_type": content_type,
                "body": body,
                "stderr": if stderr.is_empty() { Value::Null } else { Value::String(stderr) },
                "user_agent": clean_text(user_agent, 260)
            })
        }
        Err(err) => json!({
            "ok": false,
            "status_code": 0,
            "content_type": "",
            "body": "",
            "stderr": format!("curl_spawn_failed:{err}"),
            "user_agent": clean_text(user_agent, 260)
        }),
    }
}

fn fetch_serper_with_curl(
    api_key: &str,
    query: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    user_agent: &str,
    top_k: usize,
) -> Value {
    let timeout_sec = ((timeout_ms as f64) / 1000.0).ceil() as u64;
    let payload = json!({
        "q": clean_text(query, 900),
        "num": top_k.clamp(1, 12)
    });
    let payload_raw = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    let output = Command::new("curl")
        .arg("-sS")
        .arg("-L")
        .arg("--compressed")
        .arg("--proto")
        .arg("=http,https")
        .arg("--connect-timeout")
        .arg(timeout_sec.max(1).to_string())
        .arg("--max-time")
        .arg(timeout_sec.max(1).to_string())
        .arg("-A")
        .arg(clean_text(user_agent, 260))
        .arg("-H")
        .arg("Accept: application/json")
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-H")
        .arg(format!("X-API-KEY: {}", clean_text(api_key, 600)))
        .arg("-d")
        .arg(payload_raw)
        .arg("-w")
        .arg("\n__STATUS__:%{http_code}\n__CTYPE__:%{content_type}")
        .arg(SERPER_SEARCH_URL)
        .output();
    match output {
        Ok(run) => {
            let stdout = String::from_utf8_lossy(&run.stdout).to_string();
            let stderr = clean_text(&String::from_utf8_lossy(&run.stderr), 320);
            let status_marker = "\n__STATUS__:";
            let ctype_marker = "\n__CTYPE__:";
            let (body_and_status, content_type) = match stdout.rsplit_once(ctype_marker) {
                Some((left, right)) => (left.to_string(), clean_text(right, 120)),
                None => (stdout, String::new()),
            };
            let (body_raw, status_raw) = match body_and_status.rsplit_once(status_marker) {
                Some((left, right)) => (left.to_string(), clean_text(right, 12)),
                None => (body_and_status, "0".to_string()),
            };
            let status_code = status_raw.parse::<i64>().unwrap_or(0);
            let body = clip_bytes(&body_raw, max_response_bytes.max(256));
            let status_ok = (200..300).contains(&status_code);
            json!({
                "ok": run.status.success() && status_ok,
                "status_code": status_code,
                "content_type": content_type,
                "body": body,
                "stderr": if stderr.is_empty() { Value::Null } else { Value::String(stderr) },
                "user_agent": clean_text(user_agent, 260)
            })
        }
        Err(err) => json!({
            "ok": false,
            "status_code": 0,
            "content_type": "",
            "body": "",
            "stderr": format!("serper_curl_spawn_failed:{err}"),
            "user_agent": clean_text(user_agent, 260)
        }),
    }
}

fn is_retryable_fetch_result(row: &Value) -> bool {
    let status = row.get("status_code").and_then(Value::as_i64).unwrap_or(0);
    if matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504) {
        return true;
    }
    let error = clean_text(row.get("stderr").and_then(Value::as_str).unwrap_or(""), 220)
        .to_ascii_lowercase();
    error.contains("timed out")
        || error.contains("timeout")
        || error.contains("econnreset")
        || error.contains("temporarily unavailable")
        || error.contains("could not resolve host")
        || error.contains("empty reply")
}

