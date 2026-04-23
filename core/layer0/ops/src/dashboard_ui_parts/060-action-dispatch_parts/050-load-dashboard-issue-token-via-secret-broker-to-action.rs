
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
    let marker = "__INFRING_STATUS__:";
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
    _payload: &Value,
) -> Result<(u16, Value), (String, u16)> {
    #[cfg(test)]
    {
        if let Some(status) = _payload
            .get("__github_issue_mock_status")
            .and_then(Value::as_u64)
            .map(|raw| raw.clamp(0, u16::MAX as u64) as u16)
        {
            let mock_body = _payload
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
        .arg("\n__INFRING_STATUS__:%{http_code}\n")
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

include!("001-run_action_family_app_arm_parts/001-run_action_family_app_arm_001.rs");
include!("001-run_action_family_app_arm_parts/002-run_action_family_app_arm_002.rs");
include!("009-run_action_family_dashboard_agent_parts/010-dashboard-agent-task-total-size-to-dashboard-agent-task-shared-a.rs");
include!("009-run_action_family_dashboard_agent_parts/020-action-family-dashboard-agent.rs");

fn run_action_family_app(root: &Path, normalized: &str, payload: &Value) -> LaneResult {
    let primary = run_action_family_app_arm_001(root, normalized, payload);
    if primary.ok {
        return primary;
    }
    let secondary = run_action_family_app_arm_002(root, normalized, payload);
    if secondary.ok {
        return secondary;
    }
    run_action_family_dashboard_agent(root, normalized, payload)
}
fn run_action(root: &Path, action: &str, payload: &Value) -> LaneResult {
    let normalized = clean_text(action, 80);
    run_action_family_app(root, normalized.as_str(), payload)
}
