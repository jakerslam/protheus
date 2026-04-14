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

fn web_input_lacks_explicit_query_pack(input: &Value) -> bool {
    input.get("queries")
        .and_then(Value::as_array)
        .map(|rows| rows.is_empty())
        .unwrap_or(true)
}

fn shallow_workspace_plus_web_compare_query(query: &str, user_message: &str) -> bool {
    let current_query = clean_text(query, 600);
    if current_query.is_empty() {
        return true;
    }
    let lowered = current_query.to_ascii_lowercase();
    if lowered == clean_text(user_message, 600).to_ascii_lowercase()
        || lowered.contains("compare this system")
    {
        return true;
    }
    lowered.contains("openclaw")
        && !lowered.contains("docs")
        && !lowered.contains("architecture")
        && !lowered.contains("site:")
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
            workspace_plus_web_comparison_queries_from_message(user_message)
                .map(|(workspace_query, _)| workspace_query)
                .or_else(|| {
                    workspace_analyze_intent_from_message(user_message, &lowered).and_then(
                        |(_, payload)| {
                            payload
                                .get("query")
                                .and_then(Value::as_str)
                                .map(|value| clean_text(value, 800))
                        },
                    )
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
        if let Some(comparison_payload) =
            workspace_plus_web_comparison_web_payload_from_message(user_message)
        {
            let current_query = clean_text(
                normalized_input
                    .get("query")
                    .or_else(|| normalized_input.get("q"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                600,
            );
            let lacks_query_pack = web_input_lacks_explicit_query_pack(&normalized_input);
            if lacks_query_pack || shallow_workspace_plus_web_compare_query(&current_query, user_message) {
                if !normalized_input.is_object() {
                    normalized_input = json!({});
                }
                for key in ["queries", "source", "aperture"] {
                    if let Some(value) = comparison_payload.get(key) {
                        normalized_input[key] = value.clone();
                    }
                }
                if shallow_workspace_plus_web_compare_query(&current_query, user_message) {
                    if let Some(value) = comparison_payload.get("query") {
                        normalized_input["query"] = value.clone();
                    }
                }
            }
        }
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
            if normalized_input
                .get("queries")
                .and_then(Value::as_array)
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                if let Some(framework_payload) = framework_catalog_web_payload_from_query(&cleaned_query) {
                    for key in ["query", "queries", "source", "aperture"] {
                        if let Some(value) = framework_payload.get(key) {
                            normalized_input[key] = value.clone();
                        }
                    }
                }
            }
        }
    }
    normalized_input
}

fn workspace_plus_web_tool_leg_for_name(normalized_name: &str) -> &'static str {
    match normalized_name {
        "workspace_analyze" | "workspace_scan" | "analyze_workspace" | "terminal_exec" => {
            "workspace"
        }
        "batch_query"
        | "batch-query"
        | "web_search"
        | "search_web"
        | "search"
        | "web_query" => "web",
        _ => "",
    }
}

fn response_tool_row_has_workspace_plus_web_leg(row: &Value, leg: &str) -> bool {
    let normalized = normalize_tool_name(row.get("name").and_then(Value::as_str).unwrap_or(""));
    !normalized.is_empty() && workspace_plus_web_tool_leg_for_name(&normalized) == leg
}

fn draft_response_implies_retryable_web_failure(draft_response: &str) -> bool {
    let lowered = clean_text(draft_response, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let mentions_web = lowered.contains("web search")
        || lowered.contains("search function")
        || lowered.contains("search capabilities")
        || lowered.contains("tool execution")
        || lowered.contains("function call format");
    if !mentions_web {
        return false;
    }
    lowered.contains("not currently operational")
        || lowered.contains("isn't currently operational")
        || lowered.contains("isn't currently")
        || lowered.contains("aren't available")
        || lowered.contains("not available")
        || lowered.contains("unable to")
        || lowered.contains("can't")
        || lowered.contains("cannot")
        || lowered.contains("rejected")
        || lowered.contains("failed")
        || response_looks_like_tool_ack_without_findings(&lowered)
}

fn fallback_live_web_query_from_failed_draft(user_message: &str, draft_response: &str) -> String {
    if let Some(query) = natural_web_search_query_from_message(draft_response) {
        return clean_text(&query, 600);
    }
    if let Some(query) = extract_leading_quoted_natural_web_query(draft_response, 600) {
        return clean_text(&query, 600);
    }
    let cleaned = clean_text(user_message, 600);
    if !cleaned.is_empty() {
        return cleaned;
    }
    "latest ai developments".to_string()
}

fn fallback_live_web_intent_from_failed_draft(
    user_message: &str,
    draft_response: &str,
) -> Option<(String, Value)> {
    if !draft_response_implies_retryable_web_failure(draft_response) {
        return None;
    }
    let query = fallback_live_web_query_from_failed_draft(user_message, draft_response);
    Some((
        "batch_query".to_string(),
        json!({
            "source": "web",
            "query": query,
            "aperture": "medium"
        }),
    ))
}

fn latent_tool_candidate_completion_cards(
    root: &Path,
    snapshot: &Value,
    actor_agent_id: &str,
    existing: Option<&Value>,
    user_message: &str,
    draft_response: &str,
    latent_tool_candidates: &Value,
    response_tools: &[Value],
) -> Vec<Value> {
    let requires_workspace_plus_web =
        workspace_plus_web_comparison_queries_from_message(user_message).is_some();
    let requires_live_web = natural_web_intent_from_user_message(user_message).is_some()
        || draft_response_implies_retryable_web_failure(draft_response);
    if !requires_workspace_plus_web && !requires_live_web {
        return Vec::new();
    }
    let mut workspace_done = response_tools
        .iter()
        .any(|row| response_tool_row_has_workspace_plus_web_leg(row, "workspace"));
    let mut web_done = response_tools
        .iter()
        .any(|row| response_tool_row_has_workspace_plus_web_leg(row, "web"));
    if (requires_workspace_plus_web && workspace_done && web_done)
        || (requires_live_web && !requires_workspace_plus_web && web_done)
    {
        return Vec::new();
    }
    let mut extra_cards = Vec::<Value>::new();
    for row in latent_tool_candidates
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
    {
        let normalized_name =
            normalize_tool_name(row.get("tool").and_then(Value::as_str).unwrap_or(""));
        if normalized_name.is_empty() {
            continue;
        }
        let leg = workspace_plus_web_tool_leg_for_name(&normalized_name);
        if requires_workspace_plus_web {
            if leg.is_empty()
                || (leg == "workspace" && workspace_done)
                || (leg == "web" && web_done)
            {
                continue;
            }
        } else if requires_live_web {
            if leg != "web" || web_done {
                continue;
            }
        }
        let proposed_input = row
            .get("proposed_input")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let input_for_call =
            normalize_inline_tool_execution_input(&normalized_name, &proposed_input, user_message);
        let payload = execute_tool_call_with_recovery(
            root,
            snapshot,
            actor_agent_id,
            existing,
            &normalized_name,
            &input_for_call,
        );
        let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let result_text = summarize_tool_payload(&normalized_name, &payload);
        let card_status = tool_card_status_from_payload(&payload);
        extra_cards.push(json!({
            "id": format!("tool-{}-latent-{}", normalized_name, extra_cards.len()),
            "name": normalized_name,
            "input": trim_text(&input_for_call.to_string(), 4000),
            "result": trim_text(&result_text, 24_000),
            "is_error": !ok,
            "blocked": card_status == "blocked" || card_status == "policy_denied",
            "status": card_status,
            "tool_attempt_receipt": payload
                .pointer("/tool_pipeline/tool_attempt_receipt")
                .cloned()
                .unwrap_or(Value::Null)
        }));
        if leg == "workspace" {
            workspace_done = true;
        } else if leg == "web" {
            web_done = true;
        }
        if requires_workspace_plus_web && workspace_done && web_done {
            break;
        }
        if requires_live_web && web_done {
            break;
        }
    }
    if requires_live_web && !requires_workspace_plus_web && !web_done {
        let fallback_intent = natural_web_intent_from_user_message(user_message)
            .or_else(|| fallback_live_web_intent_from_failed_draft(user_message, draft_response));
        if let Some((fallback_tool, fallback_input)) = fallback_intent {
            let normalized_name = normalize_tool_name(&fallback_tool);
            if !normalized_name.is_empty()
                && workspace_plus_web_tool_leg_for_name(&normalized_name) == "web"
            {
                let input_for_call = normalize_inline_tool_execution_input(
                    &normalized_name,
                    &fallback_input,
                    user_message,
                );
                let payload = execute_tool_call_with_recovery(
                    root,
                    snapshot,
                    actor_agent_id,
                    existing,
                    &normalized_name,
                    &input_for_call,
                );
                let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
                let result_text = summarize_tool_payload(&normalized_name, &payload);
                let card_status = tool_card_status_from_payload(&payload);
                extra_cards.push(json!({
                    "id": format!("tool-{}-latent-{}", normalized_name, extra_cards.len()),
                    "name": normalized_name,
                    "input": trim_text(&input_for_call.to_string(), 4000),
                    "result": trim_text(&result_text, 24_000),
                    "is_error": !ok,
                    "blocked": card_status == "blocked" || card_status == "policy_denied",
                    "status": card_status,
                    "tool_attempt_receipt": payload
                        .pointer("/tool_pipeline/tool_attempt_receipt")
                        .cloned()
                        .unwrap_or(Value::Null)
                }));
            }
        }
    }
    extra_cards
}
