fn run_action_family_dashboard_github(root: &Path, normalized: &str, payload: &Value) -> LaneResult {
    match normalized {
        "dashboard.github.issue.create" => {
            let title = match sanitize_dashboard_issue_title(payload) {
                Ok(value) => value,
                Err(error) => {
                    return LaneResult {
                        ok: false,
                        status: 400,
                        argv: vec!["dashboard.github.issue.create".to_string()],
                        payload: Some(canonical_action_error_payload(
                            "github_issue_create",
                            error,
                            400,
                            None,
                        )),
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
                        payload: Some(canonical_action_error_payload(
                            "github_issue_create",
                            error,
                            400,
                            None,
                        )),
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
                        payload: Some(canonical_action_error_payload(
                            "github_issue_create",
                            error,
                            400,
                            None,
                        )),
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
                        payload: Some(canonical_action_error_payload(
                            "github_issue_create",
                            "github_issue_auth_missing",
                            401,
                            Some("no github auth token, please input your token first"),
                        )),
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
                            payload: Some(canonical_action_error_payload(
                                "github_issue_create",
                                "github_issue_transport_error",
                                502,
                                None,
                            )),
                        }
                    }
                }
                Ok((status, _)) => {
                    let code = github_issue_http_error_code(status);
                    LaneResult {
                        ok: false,
                        status: status as i32,
                        argv: vec!["dashboard.github.issue.create".to_string()],
                        payload: Some(canonical_action_error_payload(
                            "github_issue_create",
                            code,
                            status as i32,
                            None,
                        )),
                    }
                }
                Err((error, status)) => LaneResult {
                    ok: false,
                    status: status as i32,
                    argv: vec!["dashboard.github.issue.create".to_string()],
                    payload: Some(canonical_action_error_payload(
                        "github_issue_create",
                        &error,
                        status as i32,
                        None,
                    )),
                },
            }
        }
        _ => run_action_family_dashboard_troubleshooting(root, normalized, payload),
    }
}
