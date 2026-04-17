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
        source_kind: "web".to_string(),
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

fn candidate_from_search_payload(query: &str, payload: &Value) -> Result<Candidate, String> {
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Err(clean_text(
            payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("adapter_failed"),
            200,
        ));
    }
    let raw_summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        1800,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        6_000,
    );
    let mut locator = first_non_search_engine_link(payload);
    if locator.is_empty() {
        locator = clean_text(
            payload
                .get("requested_url")
                .or_else(|| payload.pointer("/receipt/requested_url"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            2200,
        );
    }
    let content_normalized =
        normalize_snippet_text(&normalize_htmlish_content_for_snippet(&content), query, &locator);
    let summary = normalize_snippet_text(&raw_summary, query, &locator);
    let summary_low_signal = looks_like_low_signal_search_summary(&summary);
    let content_empty_duckduckgo_shell =
        looks_like_empty_duckduckgo_instant_shell_text(&content_normalized);
    let summary_defers_to_content = summary_should_defer_to_content(&raw_summary);
    let domains = extract_domains_from_text(
        if content.is_empty() {
            &raw_summary
        } else {
            &content
        },
        5,
    );
    let mut snippet =
        if !summary.is_empty()
            && !summary_defers_to_content
            && !looks_like_ack_only(&summary)
            && !summary_low_signal
        {
            summary.clone()
        } else {
            String::new()
        };
    if snippet.is_empty()
        && !content_normalized.is_empty()
        && !looks_like_ack_only(&content_normalized)
        && !content_empty_duckduckgo_shell
    {
        snippet = trim_words(&content_normalized, 56);
    }
    if snippet.is_empty()
        && !summary.is_empty()
        && !summary_defers_to_content
        && !looks_like_ack_only(&summary)
        && !summary_low_signal
    {
        snippet = trim_words(&summary, 56);
    }
    if snippet.is_empty() {
        return Err("no_usable_summary".to_string());
    }
    if looks_like_source_only_snippet(&snippet) {
        return Err("no_usable_summary".to_string());
    }
    let locator_domain = extract_domains_from_text(&locator, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    let title = if !locator_domain.is_empty() && !is_search_engine_domain(&locator_domain) {
        format!("Web result from {}", clean_text(&locator_domain, 120))
    } else if let Some(first_domain) = domains.first() {
        format!("Web result from {}", clean_text(first_domain, 120))
    } else if locator.is_empty() {
        format!("Web result for {}", clean_text(query, 120))
    } else {
        format!("Web result from {}", clean_text(&locator, 120))
    };
    Ok(Candidate {
        source_kind: "web".to_string(),
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
