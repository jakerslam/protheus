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

fn normalize_inline_tool_execution_input(
    normalized_name: &str,
    input: &Value,
    user_message: &str,
) -> Value {
    let mut normalized_input = input.clone();
    if normalized_name == "workspace_analyze" {
        let lowered = clean_text(user_message, 600).to_ascii_lowercase();
        let hydrated_query = clean_text(
            normalized_input
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or(""),
            800,
        );
        let fallback_query = if hydrated_query.is_empty() {
            workspace_analyze_intent_from_message(user_message, &lowered)
                .and_then(|(_, payload)| {
                    payload
                        .get("query")
                        .and_then(Value::as_str)
                        .map(|value| clean_text(value, 800))
                })
                .unwrap_or_default()
        } else {
            hydrated_query
        };
        if !fallback_query.is_empty() {
            if !normalized_input.is_object() {
                normalized_input = json!({});
            }
            normalized_input["query"] = json!(fallback_query);
            if normalized_input
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                normalized_input["path"] = json!(".");
            }
            if normalized_input.get("full").is_none() {
                normalized_input["full"] = json!(true);
            }
        }
    }
    if matches!(
        normalized_name,
        "batch_query" | "batch-query" | "web_search" | "search_web" | "search" | "web_query"
    ) {
        let raw_query = clean_text(
            normalized_input
                .get("query")
                .or_else(|| normalized_input.get("q"))
                .and_then(Value::as_str)
                .unwrap_or(user_message),
            600,
        );
        let cleaned_query = natural_web_search_query_from_message(&raw_query)
            .unwrap_or_else(|| strip_wrapped_natural_web_query(&raw_query, 600));
        if !cleaned_query.is_empty() {
            if !normalized_input.is_object() {
                normalized_input = json!({});
            }
            normalized_input["query"] = json!(cleaned_query);
            if normalized_input
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                normalized_input["source"] = json!("web");
            }
            if normalized_input
                .get("aperture")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                normalized_input["aperture"] = json!("medium");
            }
        }
    }
    normalized_input
}

fn tool_result_hidden_artifact_value(payload: &Value, key: &str) -> Option<Value> {
    let value = payload
        .get(key)
        .or_else(|| payload.pointer(&format!("/tool_pipeline/raw_payload/{key}")))?;
    match key {
        "search_results" | "provider_results" => value.as_array().and_then(|rows| {
            let projected = rows
                .iter()
                .filter_map(project_hidden_tool_result_row)
                .take(6)
                .collect::<Vec<_>>();
            (!projected.is_empty()).then(|| Value::Array(projected))
        }),
        "evidence_refs" => value.as_array().map(|rows| {
            Value::Array(rows.iter().take(6).cloned().collect::<Vec<_>>())
        }),
        "tool_result_quality" => value.is_object().then(|| value.clone()),
        _ => None,
    }
}

fn project_hidden_tool_result_row(value: &Value) -> Option<Value> {
    match value {
        Value::String(raw) => {
            let snippet = trim_text(raw.trim(), 1_200);
            (!snippet.is_empty()).then(|| Value::String(snippet))
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for key in [
                "source_kind",
                "title",
                "locator",
                "snippet",
                "summary",
                "content_preview",
                "score",
                "timestamp",
                "permissions",
                "status_code",
                "query",
                "stage",
                "provider",
                "status",
                "error",
                "links",
            ] {
                let Some(value) = map.get(key) else { continue };
                match value {
                    Value::String(raw) => {
                        let cleaned = trim_text(raw.trim(), if key == "locator" { 2_200 } else { 1_200 });
                        if !cleaned.is_empty() {
                            out.insert(key.to_string(), Value::String(cleaned));
                        }
                    }
                    Value::Number(_) => {
                        out.insert(key.to_string(), value.clone());
                    }
                    Value::Bool(_) => {
                        out.insert(key.to_string(), value.clone());
                    }
                    Value::Array(rows) if key == "links" => {
                        let links = rows
                            .iter()
                            .filter_map(Value::as_str)
                            .map(|raw| trim_text(raw.trim(), 2_200))
                            .filter(|raw| !raw.is_empty())
                            .take(4)
                            .map(Value::String)
                            .collect::<Vec<_>>();
                        if !links.is_empty() {
                            out.insert(key.to_string(), Value::Array(links));
                        }
                    }
                    _ => {}
                }
            }
            (!out.is_empty()).then(|| Value::Object(out))
        }
        _ => None,
    }
}

fn carry_hidden_tool_result_artifacts(card: &mut Value, payload: &Value) {
    let Some(obj) = card.as_object_mut() else { return };
    for key in [
        "search_results",
        "provider_results",
        "evidence_refs",
        "tool_result_quality",
    ] {
        if let Some(value) = tool_result_hidden_artifact_value(payload, key) {
            obj.insert(key.to_string(), value);
        }
    }
}

fn response_tool_card(
    id: String,
    tool_name: &str,
    input: &Value,
    payload: &Value,
    is_error: bool,
    status: &str,
) -> Value {
    let mut card = json!({
        "id": id,
        "name": normalize_tool_name(tool_name),
        "input": trim_text(&input.to_string(), 4000),
        "result": trim_text(&summarize_tool_payload(tool_name, payload), 24_000),
        "is_error": is_error,
        "blocked": status == "blocked" || status == "policy_denied",
        "status": status,
        "tool_attempt_receipt": payload
            .pointer("/tool_pipeline/tool_attempt_receipt")
            .cloned()
            .unwrap_or(Value::Null)
    });
    carry_hidden_tool_result_artifacts(&mut card, payload);
    card
}

#[cfg(test)]
mod response_tool_card_tests {
    use super::*;

    #[test]
    fn response_tool_card_carries_hidden_search_results_from_tool_pipeline() {
        let payload = json!({
            "tool_pipeline": {
                "tool_attempt_receipt": {"status": "ok"},
                "raw_payload": {
                    "search_results": [
                        {
                            "title": "LangGraph docs",
                            "locator": "https://docs.langchain.com/langgraph",
                            "snippet": "LangGraph documentation covers durable execution, checkpoints, and human-in-the-loop review for reliable agents.",
                            "score": 0.91
                        }
                    ],
                    "provider_results": [
                        {
                            "query": "Compare LangGraph vs CrewAI",
                            "stage": "primary",
                            "provider": "direct_http",
                            "summary": "Web search tooling is degraded (provider readiness mismatch). Retry after credentials or provider runtime are repaired.",
                            "error": "web_search_tool_surface_degraded",
                            "links": [
                                "https://docs.langchain.com/langgraph"
                            ],
                            "ok": false
                        }
                    ],
                    "evidence_refs": [
                        {"title": "LangGraph docs", "locator": "https://docs.langchain.com/langgraph", "score": 0.91}
                    ],
                    "tool_result_quality": {"version": "v1", "flags": ["insufficient_evidence"]}
                }
            }
        });

        let card = response_tool_card(
            "tool-direct-batch_query".to_string(),
            "batch_query",
            &json!({"query": "Compare LangGraph vs CrewAI"}),
            &payload,
            false,
            "no_results",
        );

        assert_eq!(
            card.pointer("/search_results/0/title").and_then(Value::as_str),
            Some("LangGraph docs")
        );
        assert_eq!(
            card.pointer("/evidence_refs/0/locator").and_then(Value::as_str),
            Some("https://docs.langchain.com/langgraph")
        );
        assert_eq!(
            card.pointer("/provider_results/0/provider")
                .and_then(Value::as_str),
            Some("direct_http")
        );
        assert_eq!(
            card.pointer("/tool_result_quality/version")
                .and_then(Value::as_str),
            Some("v1")
        );
    }
}

fn latent_tool_candidate_completion_cards(
    root: &Path,
    snapshot: &Value,
    actor_agent_id: &str,
    existing: Option<&Value>,
    user_message: &str,
    draft_response: &str,
    allow_draft_retry_fallback: bool,
    latent_tool_candidates: &Value,
    response_tools: &[Value],
) -> Vec<Value> {
    let _ = (
        root,
        snapshot,
        actor_agent_id,
        existing,
        user_message,
        draft_response,
        allow_draft_retry_fallback,
        latent_tool_candidates,
        response_tools,
    );
    // LLM-authoritative workflow mode: never execute semantic/latent tools as
    // supplemental cards. The model must request tools through the workflow CD.
    Vec::new()
}
