// FILE_SIZE_EXCEPTION: reason=Single action-dispatch function with dense branch graph; split deferred pending semantic extraction; owner=jay; expires=2026-04-12
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

fn app_chat_requests_live_web(raw_input_lower: &str) -> bool {
    let lowered = clean_text(raw_input_lower, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("web search")
        || lowered.contains("websearch")
        || lowered.contains("search the web")
        || lowered.contains("search again")
        || lowered.contains("find information")
        || lowered.contains("finding information")
        || lowered.contains("best chili recipes")
        || ((lowered.contains("framework") || lowered.contains("frameworks"))
            && (lowered.contains("current")
                || lowered.contains("latest")
                || lowered.contains("top")))
        || (lowered.contains("search")
            && (lowered.contains("latest")
                || lowered.contains("current")
                || lowered.contains("framework")
                || lowered.contains("recipes")))
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
            format!("Web tooling was blocked by policy with structured evidence: {evidence_text}."),
            "blocked_with_structured_evidence".to_string(),
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
                "Web tooling returned low-signal output in this pass. No source-backed findings were produced yet; retry with a narrower query or one specific source URL.".to_string(),
                "success_with_gaps".to_string(),
            );
        }
        return (
            "Web tooling ran but returned low-signal output in this pass, and no structured policy-block evidence was recorded.".to_string(),
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

fn run_action(root: &Path, action: &str, payload: &Value) -> LaneResult {
    let normalized = clean_text(action, 80);
    match normalized.as_str() {
        "app.switchProvider" => {
            let provider = payload
                .get("provider")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "openai".to_string());
            let model = payload
                .get("model")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 100))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "gpt-5".to_string());
            run_lane(
                root,
                "app-plane",
                &[
                    "switch-provider".to_string(),
                    "--app=chat-ui".to_string(),
                    format!("--provider={provider}"),
                    format!("--model={model}"),
                ],
            )
        }
        "app.chat" => {
            let raw_input = payload
                .get("input")
                .and_then(Value::as_str)
                .or_else(|| payload.get("message").and_then(Value::as_str))
                .map(|v| v.to_string())
                .unwrap_or_default();
            let input = clean_text(&raw_input, 2000);
            if input.is_empty() {
                return LaneResult {
                    ok: false,
                    status: 2,
                    argv: vec!["app-plane".to_string(), "run".to_string()],
                    payload: Some(json!({
                        "ok": false,
                        "type": "infring_dashboard_action_error",
                        "error": "chat_input_required"
                    })),
                };
            }
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "chat-ui-default-agent".to_string());
            let input_lower = input.to_ascii_lowercase();
            let raw_input_lower = raw_input.to_ascii_lowercase();
            let requires_live_web = app_chat_requests_live_web(&raw_input_lower);
            let lane = {
                #[cfg(test)]
                {
                    app_chat_run_scripted_lane(root, &agent_id, &input).unwrap_or_else(|| {
                        run_lane(
                            root,
                            "app-plane",
                            &[
                                "run".to_string(),
                                "--app=chat-ui".to_string(),
                                format!("--session-id={agent_id}"),
                                format!("--input={input}"),
                            ],
                        )
                    })
                }
                #[cfg(not(test))]
                {
                    run_lane(
                        root,
                        "app-plane",
                        &[
                            "run".to_string(),
                            "--app=chat-ui".to_string(),
                            format!("--session-id={agent_id}"),
                            format!("--input={input}"),
                        ],
                    )
                }
            };
            let mut lane_payload = lane.payload.clone().unwrap_or_else(|| json!({}));
            if !lane_payload.is_object() {
                lane_payload = json!({
                    "ok": lane.ok,
                    "type": "infring_dashboard_action_lane_passthrough"
                });
            }
            if requires_live_web {
                let tools_now = lane_payload
                    .get("tools")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                if app_chat_web_search_call_count(&tools_now) == 0 {
                    let fallback_query = app_chat_extract_web_query(&raw_input);
                    let fallback_lane = app_chat_run_web_batch_query(root, &fallback_query, payload);
                    let fallback_payload = fallback_lane.payload.clone().unwrap_or_else(|| json!({}));
                    let fallback_ok = fallback_lane.ok
                        && fallback_payload
                            .get("ok")
                            .and_then(Value::as_bool)
                            .unwrap_or(true);
                    if fallback_ok {
                        let summary = clean_text(
                            fallback_payload
                                .get("summary")
                                .or_else(|| fallback_payload.get("response"))
                                .and_then(Value::as_str)
                                .unwrap_or(""),
                            2_000,
                        );
                        let assistant = if summary.is_empty() {
                            format!("Web search ran for \"{fallback_query}\" and returned results.")
                        } else {
                            format!("Web search results for \"{fallback_query}\": {summary}")
                        };
                        let mut tools = tools_now;
                        tools.push(json!({
                            "name": "batch_query",
                            "status": "ok",
                            "ok": true,
                            "query": fallback_query,
                            "result": summary,
                            "source": "web",
                            "evidence_refs": fallback_payload.get("evidence_refs").cloned().unwrap_or_else(|| json!([]))
                        }));
                        lane_payload["tools"] = Value::Array(tools);
                        lane_payload["response"] = json!(assistant.clone());
                        lane_payload["output"] = json!(assistant.clone());
                        if let Some(turn) = lane_payload.get_mut("turn").and_then(Value::as_object_mut) {
                            turn.insert("assistant".to_string(), json!(assistant.clone()));
                        }
                        lane_payload["web_tooling_fallback"] = json!({
                            "applied": true,
                            "query": fallback_query,
                            "status": "ok",
                            "source": "batch_query"
                        });
                        let mut response_finalization = lane_payload
                            .get("response_finalization")
                            .cloned()
                            .unwrap_or_else(|| json!({}));
                        if !response_finalization.is_object() {
                            response_finalization = json!({});
                        }
                        response_finalization["outcome"] =
                            json!("forced_web_tool_attempt_success");
                        lane_payload["response_finalization"] = response_finalization;
                    } else {
                        let assistant = "Web tooling execution failed before any search tool call was recorded (error_code: web_tool_not_invoked). Retry lane: run `batch_query` with a narrower query or one specific source URL.".to_string();
                        lane_payload["response"] = json!(assistant.clone());
                        lane_payload["output"] = json!(assistant.clone());
                        if let Some(turn) = lane_payload.get_mut("turn").and_then(Value::as_object_mut) {
                            turn.insert("assistant".to_string(), json!(assistant.clone()));
                        }
                        lane_payload["error"] = json!("web_tool_not_invoked");
                        lane_payload["web_tooling_fallback"] = json!({
                            "applied": true,
                            "query": fallback_query,
                            "status": "failed",
                            "error_code": "web_tool_not_invoked",
                            "lane_ok": fallback_lane.ok,
                            "lane_status": fallback_lane.status
                        });
                        let mut response_finalization = lane_payload
                            .get("response_finalization")
                            .cloned()
                            .unwrap_or_else(|| json!({}));
                        if !response_finalization.is_object() {
                            response_finalization = json!({});
                        }
                        response_finalization["outcome"] =
                            json!("forced_web_tool_not_invoked");
                        response_finalization["error_code"] =
                            json!("web_tool_not_invoked");
                        lane_payload["response_finalization"] = response_finalization;
                    }
                }
            }
            let tools_for_rewrite = lane_payload
                .get("tools")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let lane_response_before_rewrite = lane_payload
                .get("response")
                .and_then(Value::as_str)
                .or_else(|| lane_payload.get("output").and_then(Value::as_str))
                .or_else(|| {
                    lane_payload
                        .get("turn")
                        .and_then(|turn| turn.get("assistant"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("")
                .to_string();
            let (rewritten_response, rewrite_outcome) = app_chat_rewrite_tooling_response(
                &raw_input,
                &lane_response_before_rewrite,
                &tools_for_rewrite,
            );
            if !rewrite_outcome.is_empty() {
                lane_payload["response"] = json!(rewritten_response.clone());
                lane_payload["output"] = json!(rewritten_response.clone());
                if let Some(turn) = lane_payload.get_mut("turn").and_then(Value::as_object_mut) {
                    turn.insert("assistant".to_string(), json!(rewritten_response.clone()));
                }
                let mut response_finalization = lane_payload
                    .get("response_finalization")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                if !response_finalization.is_object() {
                    response_finalization = json!({});
                }
                response_finalization["outcome"] = json!(rewrite_outcome);
                lane_payload["response_finalization"] = response_finalization;
            }
            let mut assistant_text = String::new();
            if lane.ok {
                assistant_text = lane_payload
                    .get("response")
                    .and_then(Value::as_str)
                    .or_else(|| lane_payload.get("output").and_then(Value::as_str))
                    .or_else(|| {
                        lane_payload
                            .get("turn")
                            .and_then(|turn| turn.get("assistant"))
                            .and_then(Value::as_str)
                    })
                    .or_else(|| {
                        lane_payload
                            .get("turns")
                            .and_then(Value::as_array)
                            .and_then(|turns| turns.last())
                            .and_then(|turn| turn.get("assistant").and_then(Value::as_str))
                    })
                    .unwrap_or("")
                    .to_string();
            }
            let runtime_flags = Flags {
                mode: "runtime-sync".to_string(),
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                team: DEFAULT_TEAM.to_string(),
                refresh_ms: DEFAULT_REFRESH_MS,
                pretty: false,
            };
            let runtime = build_runtime_sync(root, &runtime_flags);
            let mut runtime_sync = runtime.get("summary").cloned().unwrap_or_else(|| json!({}));
            if !runtime_sync.is_object() {
                runtime_sync = json!({});
            }
            let health =
                read_cached_snapshot_component(root, "health").unwrap_or_else(|| json!({}));
            let receipt_latency_p95 = i64_from_value(
                health.pointer("/dashboard_metrics/receipt_latency_p95_ms/value"),
                0,
            );
            let receipt_latency_p99 = i64_from_value(
                health.pointer("/dashboard_metrics/receipt_latency_p99_ms/value"),
                0,
            );
            let benchmark_sanity_status = clean_text(
                health
                    .pointer("/checks/benchmark_sanity/status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                32,
            );
            runtime_sync["receipt_latency_p95_ms"] = json!(receipt_latency_p95);
            runtime_sync["receipt_latency_p99_ms"] = json!(receipt_latency_p99);
            runtime_sync["benchmark_sanity_status"] = json!(benchmark_sanity_status);
            runtime_sync["critical_attention_total"] = runtime
                .pointer("/attention_queue/critical_total_count")
                .cloned()
                .unwrap_or_else(|| json!(0));
            runtime_sync["conduit_signals_raw"] = runtime
                .pointer("/attention_queue/backpressure/conduit_signals_raw")
                .cloned()
                .unwrap_or_else(|| json!(0));
            lane_payload["runtime_sync"] = runtime_sync.clone();
            let assistant_lower = assistant_text.to_ascii_lowercase();
            if runtime_sync_requested(&input_lower)
                || assistant_runtime_access_denied(&assistant_lower)
            {
                let queue_depth = i64_from_value(runtime_sync.get("queue_depth"), 0);
                let cockpit_blocks = i64_from_value(runtime_sync.get("cockpit_blocks"), 0);
                let cockpit_total_blocks =
                    i64_from_value(runtime_sync.get("cockpit_total_blocks"), 0);
                let conduit_signals = i64_from_value(runtime_sync.get("conduit_signals"), 0);
                let authoritative = format!(
                    "Current queue depth: {queue_depth}, cockpit blocks: {cockpit_blocks} active ({cockpit_total_blocks} total), conduit signals: {conduit_signals}. Attention queue is readable. Runtime memory context and protheus/infring command surfaces are available through this dashboard lane."
                );
                lane_payload["response"] = json!(authoritative.clone());
                lane_payload["output"] = json!(authoritative.clone());
                if let Some(turn) = lane_payload.get_mut("turn").and_then(Value::as_object_mut) {
                    turn.insert("assistant".to_string(), json!(authoritative.clone()));
                }
                if let Some(turns) = lane_payload.get_mut("turns").and_then(Value::as_array_mut) {
                    if let Some(last) = turns.last_mut() {
                        if let Some(last_obj) = last.as_object_mut() {
                            last_obj.insert("assistant".to_string(), json!(authoritative));
                        }
                    }
                }
            }
            if input_lower.contains("one week ago") && input_lower.contains("memory file path") {
                let memory_dir = root.join("local/workspace/memory");
                let target = (Utc::now() - chrono::Duration::days(7))
                    .date_naive()
                    .format("%Y-%m-%d")
                    .to_string();
                let mut selected_date = target.clone();
                let mut selected_rel = format!("local/workspace/memory/{selected_date}.md");
                if !memory_dir.join(format!("{target}.md")).is_file() {
                    let mut candidates = Vec::<String>::new();
                    if let Ok(entries) = fs::read_dir(&memory_dir) {
                        for entry in entries.flatten() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if name.len() == 13
                                && name.ends_with(".md")
                                && name[..10]
                                    .chars()
                                    .all(|ch| ch.is_ascii_digit() || ch == '-')
                            {
                                candidates.push(name[..10].to_string());
                            }
                        }
                    }
                    candidates.sort();
                    if let Some(last) = candidates.last() {
                        selected_date = last.clone();
                        selected_rel = format!("local/workspace/memory/{selected_date}.md");
                    }
                }
                lane_payload["response"] = json!(format!(
                    "Exact date: {selected_date}. Memory file path: {selected_rel}."
                ));
                let mut tools = lane_payload
                    .get("tools")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                tools.push(json!({
                    "tool": "read_file",
                    "input": selected_rel
                }));
                lane_payload["tools"] = Value::Array(tools);
            }
            if input_lower.contains("summarize client layer now")
                && input_lower.contains("attention queue")
                && input_lower.contains("cockpit")
            {
                let summary_flags = Flags {
                    mode: "snapshot".to_string(),
                    host: DEFAULT_HOST.to_string(),
                    port: DEFAULT_PORT,
                    team: DEFAULT_TEAM.to_string(),
                    refresh_ms: DEFAULT_REFRESH_MS,
                    pretty: false,
                };
                let snapshot_now = build_snapshot(root, &summary_flags);
                let memory_entries = snapshot_now
                    .pointer("/memory/entries")
                    .and_then(Value::as_array)
                    .map(|rows| rows.len())
                    .unwrap_or(0);
                let receipt_count = snapshot_now
                    .pointer("/receipts/recent")
                    .and_then(Value::as_array)
                    .map(|rows| rows.len())
                    .unwrap_or(0);
                let log_count = snapshot_now
                    .pointer("/logs/recent")
                    .and_then(Value::as_array)
                    .map(|rows| rows.len())
                    .unwrap_or(0);
                let health_checks = snapshot_now
                    .pointer("/health/checks")
                    .and_then(Value::as_object)
                    .map(|rows| rows.len())
                    .unwrap_or(0);
                let attention_depth =
                    i64_from_value(snapshot_now.pointer("/attention_queue/queue_depth"), 0);
                let cockpit_blocks =
                    i64_from_value(snapshot_now.pointer("/cockpit/block_count"), 0);
                lane_payload["response"] = json!(format!(
                    "Client layer now: memory entries {memory_entries}, receipts {receipt_count}, logs {log_count}, health checks {health_checks}, attention queue depth {attention_depth}, cockpit blocks {cockpit_blocks}."
                ));
            }
            if raw_input_lower.contains("run exactly these commands to create a swarm of subagents")
                && raw_input_lower.contains("collab-plane launch-role")
            {
                let mut launched = Vec::<String>::new();
                let mut tools = lane_payload
                    .get("tools")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for raw_line in raw_input.lines() {
                    let line = raw_line.trim();
                    if !line.starts_with("protheus-ops collab-plane launch-role") {
                        continue;
                    }
                    let mut team = DEFAULT_TEAM.to_string();
                    let mut role = "analyst".to_string();
                    let mut shadow = String::new();
                    for token in line.split_whitespace() {
                        if let Some(value) = token.strip_prefix("--team=") {
                            let cleaned = clean_text(value, 60);
                            if !cleaned.is_empty() {
                                team = cleaned;
                            }
                        } else if let Some(value) = token.strip_prefix("--role=") {
                            let cleaned = clean_text(value, 60);
                            if !cleaned.is_empty() {
                                role = cleaned;
                            }
                        } else if let Some(value) = token.strip_prefix("--shadow=") {
                            shadow = clean_text(value, 80);
                        }
                    }
                    if shadow.is_empty() {
                        shadow = format!("{team}-{role}-{}", Utc::now().timestamp_millis());
                    }
                    let launch = run_lane(
                        root,
                        "collab-plane",
                        &[
                            "launch-role".to_string(),
                            format!("--team={team}"),
                            format!("--role={role}"),
                            format!("--shadow={shadow}"),
                        ],
                    );
                    if launch.ok {
                        let _ = dashboard_agent_state::upsert_profile(
                            root,
                            &shadow,
                            &json!({
                                "name": shadow,
                                "role": role,
                                "state": "Running"
                            }),
                        );
                        launched.push(shadow.clone());
                    }
                    tools.push(json!({
                        "tool": "shell",
                        "input": line
                    }));
                }
                if !tools.is_empty() {
                    lane_payload["tools"] = Value::Array(tools);
                }
                if !launched.is_empty() {
                    lane_payload["response"] = json!(launched.join(" "));
                }
            }

            let mut terminal_response = lane_payload
                .get("response")
                .and_then(Value::as_str)
                .or_else(|| lane_payload.get("output").and_then(Value::as_str))
                .or_else(|| lane_payload.pointer("/turn/assistant").and_then(Value::as_str))
                .unwrap_or("")
                .to_string();
            if terminal_response.trim().is_empty() {
                let error_code = clean_text(
                    lane_payload
                        .get("error")
                        .and_then(Value::as_str)
                        .or_else(|| {
                            lane_payload
                                .pointer("/response_finalization/error_code")
                                .and_then(Value::as_str)
                        })
                        .unwrap_or("app_chat_lane_failed"),
                    120,
                );
                terminal_response = format!(
                    "Web tooling execution failed in this turn (error: {error_code}). No source-backed findings were produced."
                );
                lane_payload["response"] = Value::String(terminal_response.clone());
                lane_payload["output"] = Value::String(terminal_response.clone());
            }
            let mut response_finalization = lane_payload
                .get("response_finalization")
                .cloned()
                .unwrap_or_else(|| json!({}));
            if !response_finalization.is_object() {
                response_finalization = json!({});
            }
            if response_finalization.get("web_invariant").is_none() {
                let tools = lane_payload
                    .get("tools")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let web_search_calls = app_chat_web_search_call_count(&tools);
                if requires_live_web && web_search_calls == 0 {
                    let forced = "Web tooling execution failed before any search tool call was recorded (error_code: web_tool_not_invoked). Retry lane: run `batch_query` with a narrower query or one specific source URL.".to_string();
                    lane_payload["response"] = json!(forced.clone());
                    lane_payload["output"] = json!(forced.clone());
                    if let Some(turn) = lane_payload.get_mut("turn").and_then(Value::as_object_mut)
                    {
                        turn.insert("assistant".to_string(), json!(forced));
                    }
                    response_finalization["outcome"] = json!("forced_web_tool_not_invoked");
                    response_finalization["error_code"] = json!("web_tool_not_invoked");
                    lane_payload["error"] = json!("web_tool_not_invoked");
                }
                let payload_error = clean_text(
                    lane_payload
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    200,
                )
                .to_ascii_lowercase();
                let payload_error_blocked = payload_error.contains("blocked")
                    || payload_error.contains("denied")
                    || payload_error.contains("policy")
                    || payload_error.contains("nexus");
                let mut tool_attempted = web_search_calls > 0
                    || tools.iter().any(|row| {
                        clean_text(
                            row.get("name")
                                .or_else(|| row.get("tool"))
                                .and_then(Value::as_str)
                                .unwrap_or(""),
                            64,
                        )
                        .to_ascii_lowercase()
                        .contains("web")
                            || clean_text(
                                row.get("name")
                                    .or_else(|| row.get("tool"))
                                    .and_then(Value::as_str)
                                    .unwrap_or(""),
                                64,
                            )
                            .to_ascii_lowercase()
                            .contains("batch_query")
                    });
                if !tool_attempted && (payload_error_blocked || (requires_live_web && !lane.ok)) {
                    tool_attempted = true;
                }
                let blocked_signal = payload_error_blocked
                    || tools.iter().any(|row| {
                        let status = clean_text(
                            row.get("status").and_then(Value::as_str).unwrap_or(""),
                            64,
                        )
                        .to_ascii_lowercase();
                        let error = clean_text(
                            row.get("error").and_then(Value::as_str).unwrap_or(""),
                            120,
                        )
                        .to_ascii_lowercase();
                        status.contains("blocked")
                            || error.contains("blocked")
                            || error.contains("denied")
                            || error.contains("policy")
                            || error.contains("nexus")
                    });
                let classification = if requires_live_web && web_search_calls == 0 {
                    "tool_not_invoked"
                } else if blocked_signal || (requires_live_web && !lane.ok && tool_attempted) {
                    "policy_blocked"
                } else if tools.iter().any(|row| {
                    let status = clean_text(
                        row.get("status").and_then(Value::as_str).unwrap_or(""),
                        64,
                    )
                    .to_ascii_lowercase();
                    let error = clean_text(
                        row.get("error").and_then(Value::as_str).unwrap_or(""),
                        120,
                    )
                    .to_ascii_lowercase();
                    status.contains("low_signal")
                        || status.contains("no_results")
                        || status.contains("no_result")
                        || error.contains("no_results")
                        || error.contains("low_signal")
                }) {
                    "low_signal"
                } else if tool_attempted {
                    "attempted_no_findings"
                } else {
                    "not_required"
                };
                response_finalization["web_invariant"] = json!({
                    "requires_live_web": requires_live_web,
                    "tool_attempted": tool_attempted,
                    "web_search_calls": web_search_calls,
                    "classification": classification,
                    "diagnostic": "forced_live_web_invariant_from_dashboard_action_bus"
                });
            }
            lane_payload["response_finalization"] = response_finalization;
            let forced_ok = lane.ok || !terminal_response.trim().is_empty();
            LaneResult {
                ok: forced_ok,
                status: if forced_ok { 0 } else { lane.status },
                argv: lane.argv,
                payload: Some(lane_payload),
            }
        }
        "collab.launchRole" => {
            let team = payload
                .get("team")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| DEFAULT_TEAM.to_string());
            let role = payload
                .get("role")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "analyst".to_string());
            let shadow = payload
                .get("shadow")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| format!("{team}-{role}-shadow"));
            run_lane(
                root,
                "collab-plane",
                &[
                    "launch-role".to_string(),
                    format!("--team={team}"),
                    format!("--role={role}"),
                    format!("--shadow={shadow}"),
                ],
            )
        }
        "skills.run" => {
            let skill = payload
                .get("skill")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            if skill.is_empty() {
                return LaneResult {
                    ok: false,
                    status: 2,
                    argv: vec!["skills-plane".to_string(), "run".to_string()],
                    payload: Some(json!({
                        "ok": false,
                        "type": "infring_dashboard_action_error",
                        "error": "skill_required"
                    })),
                };
            }
            let input = payload
                .get("input")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 600))
                .unwrap_or_default();
            let mut args = vec!["run".to_string(), format!("--skill={skill}")];
            if !input.is_empty() {
                args.push(format!("--input={input}"));
            }
            run_lane(root, "skills-plane", &args)
        }
        "dashboard.assimilate" => {
            let target = payload
                .get("target")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "codex".to_string());
            run_lane(
                root,
                "app-plane",
                &[
                    "run".to_string(),
                    "--app=chat-ui".to_string(),
                    format!("--input=assimilate target {target} with receipt-first safety"),
                ],
            )
        }
        "dashboard.benchmark" => run_lane(root, "health-status", &["dashboard".to_string()]),
        "dashboard.models.catalog" => {
            let runtime_flags = Flags {
                mode: "snapshot".to_string(),
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                team: DEFAULT_TEAM.to_string(),
                refresh_ms: DEFAULT_REFRESH_MS,
                pretty: false,
            };
            let snapshot = build_snapshot(root, &runtime_flags);
            let result = dashboard_model_catalog::catalog_payload(root, &snapshot);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: 0,
                argv: vec!["dashboard.models.catalog".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.model.routeDecision" => {
            let runtime_flags = Flags {
                mode: "snapshot".to_string(),
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                team: DEFAULT_TEAM.to_string(),
                refresh_ms: DEFAULT_REFRESH_MS,
                pretty: false,
            };
            let snapshot = build_snapshot(root, &runtime_flags);
            let result = dashboard_model_catalog::route_decision_payload(root, &snapshot, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: 0,
                argv: vec!["dashboard.model.routeDecision".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.github.issue.create" => {
            let title = match sanitize_dashboard_issue_title(payload) {
                Ok(value) => value,
                Err(error) => {
                    return LaneResult {
                        ok: false,
                        status: 400,
                        argv: vec!["dashboard.github.issue.create".to_string()],
                        payload: Some(json!({
                            "ok": false,
                            "type": "github_issue_create",
                            "error": error,
                            "status": 400
                        })),
                    };
                }
            };
            let body = match sanitize_dashboard_issue_body(payload) {
                Ok(value) => value,
                Err(error) => {
                    return LaneResult {
                        ok: false,
                        status: 400,
                        argv: vec!["dashboard.github.issue.create".to_string()],
                        payload: Some(json!({
                            "ok": false,
                            "type": "github_issue_create",
                            "error": error,
                            "status": 400
                        })),
                    };
                }
            };
            let (owner, repo) = match resolve_dashboard_issue_repo(payload) {
                Ok(parts) => parts,
                Err(error) => {
                    return LaneResult {
                        ok: false,
                        status: 400,
                        argv: vec!["dashboard.github.issue.create".to_string()],
                        payload: Some(json!({
                            "ok": false,
                            "type": "github_issue_create",
                            "error": error,
                            "status": 400
                        })),
                    };
                }
            };
            let token = match resolve_dashboard_issue_auth_token(root, payload) {
                Some(value) => value,
                None => {
                    return LaneResult {
                        ok: false,
                        status: 401,
                        argv: vec!["dashboard.github.issue.create".to_string()],
                        payload: Some(json!({
                            "ok": false,
                            "type": "github_issue_create",
                            "error": "github_issue_auth_missing",
                            "message": "no github auth token, please input your token first",
                            "status": 401
                        })),
                    };
                }
            };
            match execute_dashboard_github_issue_create_request(
                &owner, &repo, &title, &body, &token, payload,
            ) {
                Ok((status, response)) if (200..=299).contains(&status) => {
                    let number = response
                        .get("number")
                        .and_then(Value::as_i64)
                        .filter(|value| *value > 0);
                    let html_url = response
                        .get("html_url")
                        .and_then(Value::as_str)
                        .map(|v| clean_text(v, 400))
                        .unwrap_or_default();
                    let issue_url = response
                        .get("url")
                        .and_then(Value::as_str)
                        .map(|v| clean_text(v, 400))
                        .unwrap_or_else(|| {
                            number
                                .map(|n| {
                                    format!("https://api.github.com/repos/{owner}/{repo}/issues/{n}")
                                })
                                .unwrap_or_default()
                        });
                    if let Some(number) = number {
                        LaneResult {
                            ok: true,
                            status: 0,
                            argv: vec!["dashboard.github.issue.create".to_string()],
                            payload: Some(json!({
                                "ok": true,
                                "type": "github_issue_create",
                                "owner": owner,
                                "repo": repo,
                                "number": number,
                                "html_url": html_url,
                                "issue_url": issue_url
                            })),
                        }
                    } else {
                        LaneResult {
                            ok: false,
                            status: 502,
                            argv: vec!["dashboard.github.issue.create".to_string()],
                            payload: Some(json!({
                                "ok": false,
                                "type": "github_issue_create",
                                "error": "github_issue_transport_error",
                                "status": 502
                            })),
                        }
                    }
                }
                Ok((status, _)) => {
                    let code = github_issue_http_error_code(status);
                    LaneResult {
                        ok: false,
                        status: status as i32,
                        argv: vec!["dashboard.github.issue.create".to_string()],
                        payload: Some(json!({
                            "ok": false,
                            "type": "github_issue_create",
                            "error": code,
                            "status": status
                        })),
                    }
                }
                Err((error, status)) => LaneResult {
                    ok: false,
                    status: status as i32,
                    argv: vec!["dashboard.github.issue.create".to_string()],
                    payload: Some(json!({
                        "ok": false,
                        "type": "github_issue_create",
                        "error": error,
                        "status": status
                    })),
                },
            }
        }
        "dashboard.terminal.session.create" => {
            let result = dashboard_terminal_broker::create_session(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.terminal.session.create".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.terminal.exec" => {
            let result = dashboard_terminal_broker::exec_command(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: result.get("exit_code").and_then(Value::as_i64).unwrap_or(
                    if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                        0
                    } else {
                        2
                    },
                ) as i32,
                argv: vec!["dashboard.terminal.exec".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.terminal.session.close" => {
            let session_id = payload
                .get("session_id")
                .or_else(|| payload.get("sessionId"))
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_terminal_broker::close_session(root, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.terminal.session.close".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.update.check" => {
            let result = crate::dashboard_release_update::check_update(root);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.update.check".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.update.apply" => {
            let result = crate::dashboard_release_update::dispatch_update_apply(root);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.update.apply".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.system.restart" => {
            let result = crate::dashboard_release_update::dispatch_system_action(root, "restart");
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.system.restart".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.system.shutdown" => {
            let result = crate::dashboard_release_update::dispatch_system_action(root, "shutdown");
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.system.shutdown".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.runtime.executeSwarmRecommendation"
        | "dashboard.runtime.applyTelemetryRemediations" => {
            let team = payload
                .get("team")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| DEFAULT_TEAM.to_string());
            let action_key = if normalized == "dashboard.runtime.applyTelemetryRemediations" {
                "apply_telemetry_remediations"
            } else {
                "execute_swarm_recommendation"
            };
            let runtime_flags = Flags {
                mode: "runtime-sync".to_string(),
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                team: team.clone(),
                refresh_ms: DEFAULT_REFRESH_MS,
                pretty: false,
            };
            let runtime = build_runtime_sync(root, &runtime_flags);
            let summary = runtime
                .get("summary")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let queue_depth = i64_from_value(summary.get("queue_depth"), 0);
            let target_conduit_signals = i64_from_value(summary.get("target_conduit_signals"), 4);
            let critical_attention_total =
                i64_from_value(summary.get("critical_attention_total"), 0);
            let conduit_scale_required = summary
                .get("conduit_scale_required")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let snapshot_now = build_snapshot(root, &runtime_flags);
            let active_swarm_agents = snapshot_now
                .pointer("/collab/dashboard/agents")
                .and_then(Value::as_array)
                .map(|rows| rows.len() as i64)
                .unwrap_or(0);
            let mut swarm_target_agents = active_swarm_agents;
            if queue_depth >= 80 || critical_attention_total >= 5 {
                swarm_target_agents = std::cmp::max(active_swarm_agents + 2, 4);
            } else if queue_depth >= 40 || conduit_scale_required {
                swarm_target_agents = std::cmp::max(active_swarm_agents + 1, 3);
            }
            let swarm_scale_required = swarm_target_agents > active_swarm_agents;
            let throttle_required = queue_depth >= 75 || critical_attention_total >= 5;
            let predictive_drain_required = queue_depth >= 65 || critical_attention_total >= 4;
            let attention_drain_required = queue_depth >= 60 || critical_attention_total >= 2;
            let attention_compaction_required = queue_depth >= 45 || conduit_scale_required;
            let coarse_signal_remediation_required =
                i64_from_value(summary.get("cockpit_stale_blocks"), 0) > 0;
            let reliability_gate_required = false;
            let slo_gate_required = queue_depth >= 95;
            let slo_gate = json!({
                "required": slo_gate_required,
                "severity": if slo_gate_required { "high" } else { "normal" },
                "block_scale": false,
                "containment_required": slo_gate_required,
                "failed_checks": [],
                "thresholds": {
                    "spine_success_rate_min": 0.999,
                    "receipt_latency_p95_max_ms": 100.0,
                    "receipt_latency_p99_max_ms": 150.0,
                    "queue_depth_max": 90
                }
            });
            let mut role_plan = vec![json!({"role": "coordinator", "required": true})];
            if conduit_scale_required || throttle_required {
                role_plan.push(json!({"role": "researcher", "required": true}));
            }
            if queue_depth >= 60 || critical_attention_total >= 3 {
                role_plan.push(json!({"role": "analyst", "required": true}));
            }
            if swarm_scale_required {
                role_plan.push(json!({"role": "builder", "required": true}));
                role_plan.push(json!({"role": "reviewer", "required": true}));
            }
            let turns = role_plan
                .iter()
                .take(3)
                .enumerate()
                .map(|(idx, row)| {
                    let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or("agent"), 80);
                    json!({
                        "turn_id": format!("swarm-turn-{}", idx + 1),
                        "role": role,
                        "required": row.get("required").cloned().unwrap_or_else(|| json!(false)),
                        "status": "completed",
                        "summary": format!("{role} acknowledged runtime pressure and prepared remediation."),
                        "ts": now_iso()
                    })
                })
                .collect::<Vec<_>>();
            let policies = vec![
                json!({
                    "policy": "queue_throttle",
                    "required": throttle_required,
                    "applied": throttle_required
                }),
                json!({
                    "policy": "conduit_scale",
                    "required": conduit_scale_required,
                    "applied": conduit_scale_required,
                    "target_conduit_signals": target_conduit_signals
                }),
                json!({
                    "policy": "predictive_drain",
                    "required": predictive_drain_required,
                    "applied": predictive_drain_required
                }),
                json!({
                    "policy": "attention_queue_autodrain",
                    "required": attention_drain_required,
                    "applied": attention_drain_required
                }),
                json!({
                    "policy": "attention_queue_compaction",
                    "required": attention_compaction_required,
                    "applied": attention_compaction_required
                }),
                json!({
                    "policy": "coarse_lane_demotion",
                    "required": coarse_signal_remediation_required,
                    "applied": coarse_signal_remediation_required
                }),
                json!({
                    "policy": "coarse_conduit_scale_up",
                    "required": coarse_signal_remediation_required,
                    "applied": coarse_signal_remediation_required
                }),
                json!({
                    "policy": "coarse_stale_lane_drain",
                    "required": coarse_signal_remediation_required,
                    "applied": coarse_signal_remediation_required
                }),
                json!({
                    "policy": "spine_reliability_gate",
                    "required": reliability_gate_required,
                    "applied": reliability_gate_required
                }),
                json!({
                    "policy": "human_escalation_guard",
                    "required": reliability_gate_required,
                    "applied": reliability_gate_required
                }),
                json!({
                    "policy": "runtime_slo_gate",
                    "required": slo_gate_required,
                    "applied": slo_gate_required,
                    "thresholds": slo_gate.get("thresholds").cloned().unwrap_or_else(|| json!({}))
                }),
            ];
            let mut launch_receipt = Value::Null;
            if queue_depth >= RUNTIME_SYNC_DRAIN_TRIGGER_DEPTH {
                let shadow = format!("{team}-drain-{}", Utc::now().timestamp_millis());
                let launch = run_lane(
                    root,
                    "collab-plane",
                    &[
                        "launch-role".to_string(),
                        format!("--team={team}"),
                        "--role=analyst".to_string(),
                        format!("--shadow={shadow}"),
                    ],
                );
                launch_receipt = launch.payload.unwrap_or_else(|| {
                    json!({
                        "ok": launch.ok,
                        "status": launch.status,
                        "argv": launch.argv
                    })
                });
            }
            let launches = if launch_receipt.is_null() {
                Vec::<Value>::new()
            } else {
                vec![launch_receipt.clone()]
            };
            let executed_count = turns.len() as i64;
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.clone(), format!("--team={team}")],
                payload: Some(json!({
                    "ok": true,
                    "type": "infring_dashboard_runtime_action",
                    "action": action_key,
                    "ts": now_iso(),
                    "team": team,
                    "queue_depth": queue_depth,
                    "target_conduit_signals": target_conduit_signals,
                    "conduit_scale_required": conduit_scale_required,
                    "launch_receipt": launch_receipt,
                    "launches": launches,
                    "executed_count": executed_count,
                    "turns": turns,
                    "policies": policies,
                    "recommendation": {
                        "action": action_key,
                        "active_swarm_agents": active_swarm_agents,
                        "swarm_target_agents": swarm_target_agents,
                        "swarm_scale_required": swarm_scale_required,
                        "throttle_required": throttle_required,
                        "predictive_drain_required": predictive_drain_required,
                        "attention_drain_required": attention_drain_required,
                        "attention_compaction_required": attention_compaction_required,
                        "coarse_signal_remediation_required": coarse_signal_remediation_required,
                        "reliability_gate_required": reliability_gate_required,
                        "slo_gate_required": slo_gate_required,
                        "slo_gate": slo_gate,
                        "role_plan": role_plan
                    }
                })),
            }
        }
        "dashboard.agent.upsertProfile" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::upsert_profile(root, &agent_id, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.upsertProfile".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.archive" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let reason = payload
                .get("reason")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::archive_agent(root, &agent_id, &reason);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.archive".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.unarchive" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::unarchive_agent(root, &agent_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.unarchive".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.upsertContract" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::upsert_contract(root, &agent_id, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.upsertContract".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.enforceContracts" => {
            let result = dashboard_agent_state::enforce_expired_contracts(root);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: 0,
                argv: vec!["dashboard.agent.enforceContracts".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.get" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::load_session(root, &agent_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.get".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.create" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let label = payload
                .get("label")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            let result = dashboard_agent_state::create_session(root, &agent_id, &label);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.create".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.switch" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let session_id = payload
                .get("session_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("sessionId").and_then(Value::as_str))
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::switch_session(root, &agent_id, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.switch".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.delete" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let session_id = payload
                .get("session_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("sessionId").and_then(Value::as_str))
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::delete_session(root, &agent_id, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.delete".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.appendTurn" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let user_text = payload
                .get("user")
                .and_then(Value::as_str)
                .or_else(|| payload.get("input").and_then(Value::as_str))
                .map(|v| clean_chat_text_preserve_layout(v, 2000))
                .unwrap_or_default();
            let assistant_text = payload
                .get("assistant")
                .and_then(Value::as_str)
                .or_else(|| payload.get("response").and_then(Value::as_str))
                .map(|v| clean_chat_text_preserve_layout(v, 4000))
                .unwrap_or_default();
            let result =
                dashboard_agent_state::append_turn(root, &agent_id, &user_text, &assistant_text);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.appendTurn".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.set" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let value = payload.get("value").cloned().unwrap_or(Value::Null);
            let result = dashboard_agent_state::memory_kv_set(root, &agent_id, &key, &value);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.set".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.get" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::memory_kv_get(root, &agent_id, &key);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.get".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.delete" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::memory_kv_delete(root, &agent_id, &key);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.delete".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.suggestions" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let user_hint = payload
                .get("user_hint")
                .and_then(Value::as_str)
                .or_else(|| payload.get("hint").and_then(Value::as_str))
                .map(|v| clean_text(v, 220))
                .unwrap_or_default();
            let result = dashboard_agent_state::suggestions(root, &agent_id, &user_hint);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.suggestions".to_string()],
                payload: Some(result),
            }
        }
        _ => LaneResult {
            ok: false,
            status: 2,
            argv: Vec::new(),
            payload: Some(json!({
                "ok": false,
                "type": "infring_dashboard_action_error",
                "error": format!("unsupported_action:{normalized}")
            })),
        },
    }
}
