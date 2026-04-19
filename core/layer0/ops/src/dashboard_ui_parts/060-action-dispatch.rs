// FILE_SIZE_EXCEPTION: reason=Single action-dispatch function with dense branch graph; split deferred pending semantic extraction; owner=jay; expires=2026-04-23
fn clean_chat_text_preserve_layout(value: &str, max_len: usize) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|ch| *ch == '\n' || *ch == '\t' || !ch.is_control())
        .take(max_len)
        .collect::<String>()
}

fn assistant_runtime_access_denied(assistant_lower: &str) -> bool {
    const DENIED_SIGNATURES: [&str; 7] = [
        "don't have access",
        "do not have access",
        "cannot access",
        "without system monitoring",
        "text-based ai assistant",
        "cannot directly interface",
        "no access to",
    ];
    DENIED_SIGNATURES
        .iter()
        .any(|signature| assistant_lower.contains(signature))
}

fn runtime_sync_requested(input_lower: &str) -> bool {
    input_lower.contains("report runtime sync now")
        || ((input_lower.contains("queue depth")
            || input_lower.contains("cockpit blocks")
            || input_lower.contains("conduit signals"))
            && (input_lower.contains("runtime")
                || input_lower.contains("sync")
                || input_lower.contains("status")
                || input_lower.contains("what changed")))
}

fn app_chat_has_explicit_web_intent(lowered: &str) -> bool {
    lowered.contains("web search")
        || lowered.contains("websearch")
        || lowered.contains("search the web")
        || lowered.contains("search online")
        || lowered.contains("find information")
        || lowered.contains("finding information")
        || lowered.contains("look it up")
        || lowered.contains("look this up")
        || lowered.contains("search again")
        || lowered.contains("best chili recipes")
}

fn app_chat_is_meta_diagnostic_request(lowered: &str) -> bool {
    if lowered.is_empty() {
        return false;
    }
    if app_chat_has_explicit_web_intent(lowered) {
        return false;
    }
    if [
        "that was just a test",
        "that was a test",
        "did you do the web request",
        "did you try it",
        "where did that come from",
        "where the hell did that come from",
        "you returned no result",
        "you hallucinated",
        "answer the question",
    ]
    .iter()
    .any(|marker| lowered.contains(*marker))
    {
        return true;
    }
    let meta_hits = [
        "what happened",
        "workflow",
        "tool call",
        "web tooling",
        "hallucination",
        "hallucinated",
        "training data",
        "context issue",
        "last response",
        "previous response",
        "system issue",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if meta_hits == 0 {
        return false;
    }
    let signal_terms = lowered
        .split_whitespace()
        .filter(|token| token.len() >= 3)
        .count();
    meta_hits >= 2 || signal_terms <= 7
}

fn app_chat_requests_live_web(raw_input_lower: &str) -> bool {
    let lowered = clean_text(raw_input_lower, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if app_chat_has_explicit_web_intent(&lowered) {
        return true;
    }
    if app_chat_is_meta_diagnostic_request(&lowered) {
        return false;
    }
    ((lowered.contains("framework") || lowered.contains("frameworks"))
        && (lowered.contains("current")
            || lowered.contains("latest")
            || lowered.contains("top")
            || lowered.contains("best")))
        || (lowered.contains("search")
            && (lowered.contains("latest")
                || lowered.contains("current")
                || lowered.contains("framework")
                || lowered.contains("recipes")
                || lowered.contains("update")))
}

fn app_chat_extract_web_query(raw_input: &str) -> String {
    let cleaned = clean_text(raw_input, 600);
    if cleaned.is_empty() {
        return "latest public web updates".to_string();
    }
    if let Some(start) = cleaned.find('"') {
        if let Some(end_rel) = cleaned[start + 1..].find('"') {
            let quoted = clean_text(&cleaned[start + 1..start + 1 + end_rel], 320);
            if !quoted.is_empty() {
                return quoted;
            }
        }
    }
    let lowered = cleaned.to_ascii_lowercase();
    for marker in ["about ", "for "] {
        if let Some(idx) = lowered.rfind(marker) {
            let candidate = clean_text(&cleaned[idx + marker.len()..], 320);
            if !candidate.is_empty() {
                return candidate;
            }
        }
    }
    cleaned
}

fn app_chat_alignment_terms(text: &str, max_terms: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for token in clean_text(text, 2_000)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
    {
        if token.len() < 3 {
            continue;
        }
        if matches!(
            token,
            "the"
                | "and"
                | "for"
                | "with"
                | "this"
                | "that"
                | "from"
                | "into"
                | "what"
                | "when"
                | "where"
                | "why"
                | "how"
                | "about"
                | "just"
                | "again"
                | "please"
                | "best"
                | "top"
                | "give"
                | "show"
                | "find"
                | "search"
                | "web"
                | "results"
                | "result"
        ) {
            continue;
        }
        if out.iter().any(|existing| existing == token) {
            continue;
        }
        out.push(token.to_string());
        if out.len() >= max_terms {
            break;
        }
    }
    out
}

fn app_chat_web_result_matches_query(query: &str, output: &str) -> bool {
    let query_terms = app_chat_alignment_terms(query, 16);
    if query_terms.len() < 2 {
        return true;
    }
    let lowered = clean_text(output, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let matched = query_terms
        .iter()
        .filter(|term| lowered.contains(term.as_str()))
        .count();
    let required_hits = 2.min(query_terms.len());
    if matched >= required_hits {
        return true;
    }
    let ratio = (matched as f64) / (query_terms.len() as f64);
    let ratio_floor = if query_terms.len() >= 6 { 0.40 } else { 0.34 };
    ratio >= ratio_floor
}

fn app_chat_contains_irrelevant_dump(raw_input: &str, response: &str) -> bool {
    let user_lowered = clean_text(raw_input, 1_200).to_ascii_lowercase();
    let response_lowered = clean_text(response, 16_000).to_ascii_lowercase();
    if response_lowered.is_empty() {
        return false;
    }

    let role_preamble_hits = [
        "i am an expert in the field",
        "my role is to provide",
        "the user has provided",
        "my task is to refine",
        "workflow metadata",
        "the error: context collapse",
    ]
    .iter()
    .filter(|marker| response_lowered.contains(**marker))
    .count();
    if role_preamble_hits >= 2
        && !user_lowered.contains("system prompt")
        && !user_lowered.contains("role prompt")
        && !user_lowered.contains("prompt")
    {
        return true;
    }

    let competitive_dump_hits = [
        "given a tree",
        "input specification",
        "output specification",
        "sample input",
        "sample output",
        "#include <stdio.h>",
        "int main()",
        "public class",
        "translate the following java code",
        "intelligent recommendation",
        "smart recommendations",
    ]
    .iter()
    .filter(|marker| response_lowered.contains(**marker))
    .count();
    competitive_dump_hits >= 3
        && !user_lowered.contains("translate")
        && !user_lowered.contains("python function")
        && !user_lowered.contains("java code")
}

fn app_chat_tool_name_is_web_search(name: &str) -> bool {
    let lowered = clean_text(name, 120).to_ascii_lowercase();
    lowered.contains("web_search")
        || lowered.contains("search_web")
        || lowered.contains("web_query")
        || lowered.contains("batch_query")
        || lowered == "search"
        || lowered.contains("web_fetch")
}

fn app_chat_web_search_call_count(tools: &[Value]) -> usize {
    tools.iter()
        .filter(|row| {
            app_chat_tool_name_is_web_search(
                row.get("name")
                    .or_else(|| row.get("tool"))
                    .or_else(|| row.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            )
        })
        .count()
}

fn app_chat_run_web_batch_query(root: &Path, query: &str, _payload: &Value) -> LaneResult {
    #[cfg(test)]
    {
        if let Some(mock) = _payload.get("__mock_web_batch_query") {
            let mut mock_payload = if mock.is_object() { mock.clone() } else { json!({}) };
            if mock_payload.get("type").is_none() {
                mock_payload["type"] = json!("batch_query");
            }
            if mock_payload.get("query").is_none() {
                mock_payload["query"] = json!(clean_text(query, 320));
            }
            let ok = mock_payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
            return LaneResult {
                ok,
                status: if ok { 0 } else { 1 },
                argv: vec![
                    "batch-query".to_string(),
                    "--source=web".to_string(),
                    format!("--query={}", clean_text(query, 320)),
                    "--aperture=medium".to_string(),
                ],
                payload: Some(mock_payload),
            };
        }
    }
    run_lane(
        root,
        "batch-query",
        &[
            "--source=web".to_string(),
            format!("--query={}", clean_text(query, 320)),
            "--aperture=medium".to_string(),
        ],
    )
}

#[cfg(test)]
fn app_chat_run_scripted_lane(root: &Path, agent_id: &str, input: &str) -> Option<LaneResult> {
    let path = root.join("client/runtime/local/state/ui/infring_dashboard/test_chat_script.json");
    let mut script = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}));
    let mut step = None::<Value>;
    if let Some(queue) = script.get_mut("queue").and_then(Value::as_array_mut) {
        if !queue.is_empty() {
            step = Some(queue.remove(0));
        }
    }
    let step = step?;
    let mut lane_payload = if step.is_object() { step } else { json!({}) };
    let response = clean_text(
        lane_payload
            .get("response")
            .or_else(|| lane_payload.get("output"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        32_000,
    );
    if lane_payload.get("response").is_none() {
        lane_payload["response"] = Value::String(response.clone());
    }
    if lane_payload.get("output").is_none() {
        lane_payload["output"] = Value::String(response.clone());
    }
    if lane_payload.pointer("/turn/assistant").is_none() {
        lane_payload["turn"] = json!({
            "assistant": response,
            "user": clean_text(input, 2_000),
            "session_id": clean_text(agent_id, 140)
        });
    }
    if !lane_payload.get("tools").map(Value::is_array).unwrap_or(false) {
        lane_payload["tools"] = Value::Array(Vec::new());
    }
    if lane_payload.get("ok").is_none() {
        lane_payload["ok"] = Value::Bool(true);
    }
    if lane_payload.get("type").is_none() {
        lane_payload["type"] = json!("app_plane_chat_ui");
    }
    if let Some(obj) = script.as_object_mut() {
        if !obj.get("calls").map(Value::is_array).unwrap_or(false) {
            obj.insert("calls".to_string(), Value::Array(Vec::new()));
        }
        if let Some(rows) = obj.get_mut("calls").and_then(Value::as_array_mut) {
            rows.push(json!({
                "action": "app.chat",
                "agent_id": clean_text(agent_id, 140),
                "input": clean_text(input, 2_000),
                "ts": crate::now_iso()
            }));
        }
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(body) = serde_json::to_string_pretty(&script) {
        let _ = std::fs::write(&path, body);
    }
    Some(LaneResult {
        ok: lane_payload
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        status: 0,
        argv: vec![
            "app-plane".to_string(),
            "run".to_string(),
            "--app=chat-ui".to_string(),
            format!("--session-id={}", clean_text(agent_id, 140)),
            format!("--input={}", clean_text(input, 2_000)),
        ],
        payload: Some(lane_payload),
    })
}

fn app_chat_tool_blocked_signal(row: &Value) -> bool {
    let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 120)
        .to_ascii_lowercase();
    let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 240)
        .to_ascii_lowercase();
    let ty = clean_text(row.get("type").and_then(Value::as_str).unwrap_or(""), 240)
        .to_ascii_lowercase();
    row.get("blocked").and_then(Value::as_bool).unwrap_or(false)
        || status.contains("blocked")
        || status.contains("policy")
        || error.contains("blocked")
        || error.contains("permission")
        || error.contains("denied")
        || ty.contains("blocked")
        || ty.contains("policy")
}

fn app_chat_speculative_blocker_copy(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    lowered.contains("security controls")
        || lowered.contains("allowlists")
        || lowered.contains("proper authorization")
        || lowered.contains("invalid response attempt")
        || lowered.contains("preventing any web search operations")
}

fn app_chat_deferred_terminal_copy(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    lowered.starts_with("i'll get you an update")
        || lowered.contains("i'll get you an update on")
        || lowered.contains("would you like me to retry with a narrower query")
        || lowered.contains("would you like me to try a more specific query")
}

fn canonical_web_tooling_error_code(raw: &str) -> String {
    let cleaned = clean_text(raw, 140).to_ascii_lowercase();
    if cleaned.is_empty() {
        return "web_tool_error".to_string();
    }
    if cleaned.starts_with("web_tool_") {
        return cleaned;
    }
    crate::tool_output_match_filter::normalize_web_tooling_error_code(&cleaned)
}

fn canonical_action_error_payload(
    kind: &str,
    error_code: &str,
    status: i32,
    message: Option<&str>,
) -> Value {
    let code = clean_text(error_code, 140);
    let code = if code.is_empty() {
        "action_error".to_string()
    } else {
        code
    };
    let mut payload = json!({
        "ok": false,
        "type": kind,
        "error": code,
        "error_code": code,
        "status": status.max(0)
    });
    if let Some(message) = message {
        let cleaned = clean_chat_text_preserve_layout(message, 400);
        if !cleaned.is_empty() {
            payload["message"] = Value::String(cleaned);
        }
    }
    payload
}

fn app_chat_framework_gap_summary(raw_input: &str, tools: &[Value]) -> Option<String> {
    let input_lower = clean_text(raw_input, 1_000).to_ascii_lowercase();
    let joined = tools
        .iter()
        .map(|row| {
            [
                clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 1_000),
                clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 120),
                clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 240),
            ]
            .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if !(input_lower.contains("framework") || joined.contains("framework")) {
        return None;
    }
    let known = [
        "langgraph",
        "crewai",
        "autogen",
        "openai agents sdk",
        "smolagents",
    ];
    let mut found = Vec::<String>::new();
    let mut missing = Vec::<String>::new();
    for name in known {
        if joined.contains(name) {
            found.push(name.to_string());
        } else {
            missing.push(name.to_string());
        }
    }
    if found.is_empty() && missing.is_empty() {
        return None;
    }
    Some(format!(
        "Found: {}. Missing in this pass: {}.",
        if found.is_empty() {
            "none".to_string()
        } else {
            found.join(", ")
        },
        if missing.is_empty() {
            "none".to_string()
        } else {
            missing.join(", ")
        }
    ))
}

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

fn load_dashboard_issue_token_via_secret_broker(root: &Path, secret_id: &str) -> Option<String> {
    if secret_id.trim().is_empty() {
        return None;
    }
    let broker_payload = json!({
        "secret_id": secret_id,
        "with_audit": true
    });
    let lane = run_lane(
        root,
        "secret-broker-kernel",
        &[
            "load-secret".to_string(),
            format!("--payload={broker_payload}"),
        ],
    );
    if !lane.ok {
        return None;
    }
    lane.payload
        .as_ref()
        .and_then(|value| value.get("payload"))
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
        .map(|raw| raw.trim().to_string())
        .filter(|token| !token.is_empty())
}

fn resolve_dashboard_issue_auth_token(root: &Path, payload: &Value) -> Option<String> {
    #[cfg(test)]
    {
        if payload
            .get("__github_issue_mock_auth_missing")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return None;
        }
        let mock_token = payload
            .get("__github_issue_mock_token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if !mock_token.is_empty() {
            return Some(mock_token);
        }
    }
    let app_token = std::env::var("GITHUB_APP_INSTALLATION_TOKEN")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|token| !token.is_empty());
    if app_token.is_some() {
        return app_token;
    }
    let github_token = std::env::var("GITHUB_TOKEN")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|token| !token.is_empty());
    if github_token.is_some() {
        return github_token;
    }
    let secret_id = resolve_dashboard_issue_secret_id(payload);
    load_dashboard_issue_token_via_secret_broker(root, &secret_id)
}

fn github_issue_http_error_code(status: u16) -> &'static str {
    match status {
        401 => "github_issue_http_401",
        403 => "github_issue_http_403",
        404 => "github_issue_http_404",
        422 => "github_issue_http_422",
        429 => "github_issue_http_429",
        500..=599 => "github_issue_http_5xx",
        _ => "github_issue_transport_error",
    }
}

fn parse_curl_http_status_and_body(stdout: &str) -> Option<(u16, String)> {
    let marker = "__PROTHEUS_STATUS__:";
    let idx = stdout.rfind(marker)?;
    let body_raw = stdout[..idx].trim().to_string();
    let status_raw = stdout[idx + marker.len()..].lines().next()?.trim();
    let status = status_raw.parse::<u16>().ok()?;
    Some((status, body_raw))
}

fn execute_dashboard_github_issue_create_request(
    owner: &str,
    repo: &str,
    title: &str,
    body: &str,
    token: &str,
    payload: &Value,
) -> Result<(u16, Value), (String, u16)> {
    #[cfg(test)]
    {
        if let Some(status) = payload
            .get("__github_issue_mock_status")
            .and_then(Value::as_u64)
            .map(|raw| raw.clamp(0, u16::MAX as u64) as u16)
        {
            let mock_body = payload
                .get("__github_issue_mock_body")
                .cloned()
                .unwrap_or_else(|| json!({}));
            return Ok((status, mock_body));
        }
    }
    let url = format!("https://api.github.com/repos/{owner}/{repo}/issues");
    let request_body = serde_json::to_string(&json!({
        "title": title,
        "body": body
    }))
    .map_err(|_| ("github_issue_transport_error".to_string(), 502))?;
    let mut cmd = std::process::Command::new("curl");
    cmd.arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg("30")
        .arg("-X")
        .arg("POST")
        .arg("-H")
        .arg("User-Agent: Infring-Dashboard/1.0")
        .arg("-H")
        .arg("Accept: application/vnd.github+json")
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-H")
        .arg(format!("Authorization: Bearer {token}"))
        .arg("-w")
        .arg("\n__PROTHEUS_STATUS__:%{http_code}\n")
        .arg("-d")
        .arg(request_body)
        .arg(url);
    let output = cmd
        .output()
        .map_err(|_| ("github_issue_transport_error".to_string(), 502))?;
    if !output.status.success() {
        return Err(("github_issue_transport_error".to_string(), 502));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let (status, response_raw) = parse_curl_http_status_and_body(&stdout)
        .ok_or_else(|| ("github_issue_transport_error".to_string(), 502))?;
    let response_json = if response_raw.is_empty() {
        json!({})
    } else {
        serde_json::from_str::<Value>(&response_raw)
            .map_err(|_| ("github_issue_transport_error".to_string(), status.max(500)))?
    };
    Ok((status, response_json))
}

include!("060-action-dispatch_parts/001-run_action_family_app.rs");
include!("060-action-dispatch_parts/002-run_action_family_collab.rs");
include!("060-action-dispatch_parts/003-run_action_family_skills.rs");
include!("060-action-dispatch_parts/004-run_action_family_dashboard_core.rs");
include!("060-action-dispatch_parts/005-run_action_family_dashboard_github.rs");
include!("060-action-dispatch_parts/006-run_action_family_dashboard_troubleshooting.rs");
include!("060-action-dispatch_parts/007-run_action_family_dashboard_terminal.rs");
include!("060-action-dispatch_parts/008-run_action_family_dashboard_system.rs");
include!("060-action-dispatch_parts/009-run_action_family_dashboard_agent.rs");
fn run_action(root: &Path, action: &str, payload: &Value) -> LaneResult {
    let normalized = clean_text(action, 80);
    run_action_family_app(root, normalized.as_str(), payload)
}
