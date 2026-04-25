
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

fn duckduckgo_instant_answer_url(query: &str) -> String {
    let cleaned = clean_text(query, 600);
    let encoded = urlencoding::encode(&cleaned);
    format!("https://api.duckduckgo.com/?q={encoded}&format=json&no_html=1&skip_disambig=1")
}

fn first_related_topic_summary(rows: &[Value]) -> Option<(String, String)> {
    for row in rows {
        let text = clean_text(row.get("Text").and_then(Value::as_str).unwrap_or(""), 1_600);
        let locator = clean_text(
            row.get("FirstURL").and_then(Value::as_str).unwrap_or(""),
            2_200,
        );
        if !text.is_empty() {
            return Some((text, locator));
        }
        if let Some(children) = row.get("Topics").and_then(Value::as_array) {
            if let Some(found) = first_related_topic_summary(children) {
                return Some(found);
            }
        }
    }
    None
}

fn candidate_from_duckduckgo_instant_payload(
    query: &str,
    fallback_url: &str,
    payload: &Value,
) -> Result<Candidate, String> {
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Err(clean_text(
            payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("duckduckgo_instant_fetch_failed"),
            220,
        ));
    }
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        64_000,
    );
    let decoded = serde_json::from_str::<Value>(&content).unwrap_or(Value::Null);
    let decoded_is_empty_shell = looks_like_empty_duckduckgo_instant_shell(&decoded);
    let mut snippet = clean_text(
        decoded
            .get("AbstractText")
            .and_then(Value::as_str)
            .unwrap_or(""),
        1_800,
    );
    if snippet.is_empty() {
        snippet = clean_text(
            decoded.get("Answer").and_then(Value::as_str).unwrap_or(""),
            1_200,
        );
    }
    if snippet.is_empty() {
        snippet = clean_text(
            decoded
                .get("Definition")
                .and_then(Value::as_str)
                .unwrap_or(""),
            1_800,
        );
    }
    let mut locator = clean_text(
        decoded
            .get("AbstractURL")
            .and_then(Value::as_str)
            .unwrap_or(""),
        2_200,
    );
    if snippet.is_empty() {
        if let Some(related) = decoded.get("RelatedTopics").and_then(Value::as_array) {
            if let Some((related_text, related_locator)) = first_related_topic_summary(related) {
                snippet = related_text;
                if locator.is_empty() {
                    locator = related_locator;
                }
            }
        }
    }
    if snippet.is_empty() {
        let summary = clean_text(
            payload.get("summary").and_then(Value::as_str).unwrap_or(""),
            1_200,
        );
        if !summary.is_empty()
            && !decoded_is_empty_shell
            && !looks_like_ack_only(&summary)
            && !looks_like_low_signal_search_summary(&summary)
        {
            snippet = summary;
        }
    }
    if snippet.is_empty() {
        return Err("duckduckgo_instant_no_usable_summary".to_string());
    }
    let mut title = clean_text(
        decoded.get("Heading").and_then(Value::as_str).unwrap_or(""),
        160,
    );
    if title.is_empty() {
        title = format!("Instant web result for {}", clean_text(query, 120));
    }
    if locator.is_empty() {
        locator = clean_text(fallback_url, 2_200);
    }
    Ok(Candidate {
        source_kind: "web_duckduckgo_instant".to_string(),
        title,
        locator,
        snippet: snippet.clone(),
        excerpt_hash: sha256_hex(&snippet),
        timestamp: Some(crate::now_iso()),
        permissions: Some("public_web".to_string()),
        status_code: payload
            .get("status_code")
            .and_then(Value::as_i64)
            .unwrap_or(0),
    })
}
