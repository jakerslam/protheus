
fn app_chat_rewrite_tooling_response(raw_input: &str, response: &str, tools: &[Value]) -> (String, String) {
    if tools.is_empty() {
        return (response.to_string(), String::new());
    }
    if crate::tool_output_match_filter::contains_forbidden_runtime_context_markers(response) {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "parse_failed",
                "web_tool_context_mismatch",
                None,
            ),
            "suppressed_context_leak_dump".to_string(),
        );
    }
    if app_chat_contains_irrelevant_dump(raw_input, response) {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "parse_failed",
                "web_tool_context_mismatch",
                Some("irrelevant_response_dump"),
            ),
            "suppressed_irrelevant_dump".to_string(),
        );
    }
    let blocked = tools.iter().any(app_chat_tool_blocked_signal);
    let low_signal = tools.iter().any(|row| {
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        status.contains("low_signal")
            || status.contains("low-signal")
            || status.contains("no_results")
            || status.contains("no_result")
    });
    let speculative = app_chat_speculative_blocker_copy(response);
    let deferred = app_chat_deferred_terminal_copy(response);
    let query_aligned = app_chat_web_result_matches_query(raw_input, response);
    if blocked {
        let mut evidence = Vec::<String>::new();
        for row in tools {
            let ty = clean_text(row.get("type").and_then(Value::as_str).unwrap_or(""), 120);
            let err = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160);
            if !ty.is_empty() {
                evidence.push(ty);
            }
            if !err.is_empty() {
                evidence.push(err);
            }
        }
        evidence.sort();
        evidence.dedup();
        let evidence_text = if evidence.is_empty() {
            "policy_blocked".to_string()
        } else {
            clean_text(&evidence.join(", "), 260)
        };
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "policy_blocked",
                "web_tool_policy_blocked",
                Some(&evidence_text),
            ),
            "blocked_with_structured_evidence".to_string(),
        );
    }
    if !blocked && !query_aligned {
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "provider_low_signal",
                "web_tool_low_signal",
                Some("query_result_mismatch"),
            ),
            "suppressed_query_mismatch".to_string(),
        );
    }
    if low_signal && (speculative || deferred) {
        if let Some(summary) = app_chat_framework_gap_summary(raw_input, tools) {
            return (
                format!("{summary} The web run completed with partial signal; a follow-up pass is needed for full coverage."),
                "success_with_gaps".to_string(),
            );
        }
        if deferred {
            return (
                crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                    "provider_low_signal",
                    "web_tool_low_signal",
                    None,
                ),
                "success_with_gaps".to_string(),
            );
        }
        return (
            crate::tool_output_match_filter::canonical_tooling_fallback_copy(
                "parse_failed",
                "web_tool_unverified_blocker_claim",
                None,
            ),
            "suppressed_unverified_blocker_claim".to_string(),
        );
    }
    (response.to_string(), String::new())
}

fn sanitize_dashboard_issue_title(payload: &Value) -> Result<String, &'static str> {
    let raw = payload
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let normalized = clean_chat_text_preserve_layout(raw, 120)
        .replace('\n', " ")
        .replace('\t', " ");
    let title = normalized.trim().to_string();
    if title.is_empty() {
        return Err("github_issue_title_required");
    }
    Ok(title)
}

fn sanitize_dashboard_issue_body(payload: &Value) -> Result<String, &'static str> {
    let raw = payload
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let body = clean_chat_text_preserve_layout(raw, 12_000)
        .trim()
        .to_string();
    if body.is_empty() {
        return Err("github_issue_body_required");
    }
    Ok(body)
}

fn github_repo_segment_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-')
}

fn parse_dashboard_repo_slug(raw: &str) -> Option<(String, String)> {
    let trimmed = raw.trim();
    let (owner_raw, repo_raw) = trimmed.split_once('/')?;
    if owner_raw.is_empty() || repo_raw.is_empty() || repo_raw.contains('/') {
        return None;
    }
    let owner = clean_text(owner_raw, 100);
    let repo = clean_text(repo_raw, 100);
    if !github_repo_segment_valid(&owner) || !github_repo_segment_valid(&repo) {
        return None;
    }
    Some((owner, repo))
}

fn resolve_dashboard_issue_repo(payload: &Value) -> Result<(String, String), &'static str> {
    let owner_payload = clean_text(
        payload
            .get("owner")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        100,
    );
    let repo_payload = clean_text(
        payload
            .get("repo")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        100,
    );
    if !owner_payload.is_empty() || !repo_payload.is_empty() {
        if owner_payload.is_empty()
            || repo_payload.is_empty()
            || !github_repo_segment_valid(&owner_payload)
            || !github_repo_segment_valid(&repo_payload)
        {
            return Err("github_issue_repo_invalid");
        }
        return Ok((owner_payload, repo_payload));
    }
    if let Ok(raw) = std::env::var("INFRING_GITHUB_ISSUE_REPO") {
        let cleaned = clean_text(&raw, 220);
        if !cleaned.is_empty() {
            return parse_dashboard_repo_slug(&cleaned).ok_or("github_issue_repo_invalid");
        }
    }
    if let Ok(raw) = std::env::var("GITHUB_REPOSITORY") {
        let cleaned = clean_text(&raw, 220);
        if !cleaned.is_empty() {
            return parse_dashboard_repo_slug(&cleaned).ok_or("github_issue_repo_invalid");
        }
    }
    Ok(("protheuslabs".to_string(), "InfRing".to_string()))
}

fn resolve_dashboard_issue_secret_id(payload: &Value) -> String {
    let from_payload = payload
        .get("token_ref")
        .or_else(|| payload.get("secret_ref"))
        .or_else(|| payload.get("secret_id"))
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 160))
        .unwrap_or_default();
    if !from_payload.is_empty() {
        return from_payload;
    }
    let from_env = std::env::var("INFRING_GITHUB_ISSUE_SECRET_ID")
        .ok()
        .map(|raw| clean_text(&raw, 160))
        .unwrap_or_default();
    if !from_env.is_empty() {
        return from_env;
    }
    "github_issue_token".to_string()
}
