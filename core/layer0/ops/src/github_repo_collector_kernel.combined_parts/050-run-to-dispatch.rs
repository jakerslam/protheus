
fn handle_run(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let normalized = handle_resolve_run_params(payload);
    if normalized.get("ok").and_then(Value::as_bool) != Some(true) {
        return Ok(json!({
            "ok": false,
            "success": false,
            "eye": "github_repo",
            "mode": normalized.get("mode").cloned().unwrap_or(Value::String("repo_activity".to_string())),
            "error": normalized.get("error").cloned().unwrap_or(Value::String("missing_owner_or_repo".to_string()))
        }));
    }

    let normalized_obj = payload_obj(&normalized);
    let owner = support::clean_text(normalized_obj.get("owner").and_then(Value::as_str), 160);
    let repo = support::clean_text(normalized_obj.get("repo").and_then(Value::as_str), 160);
    let pr = support::as_u64(normalized_obj.get("pr"), 0);
    let max_items = support::as_u64(normalized_obj.get("max_items"), 10).clamp(1, 50);
    let min_hours = support::as_f64(normalized_obj.get("min_hours"), 4.0).clamp(0.0, 24.0 * 365.0);
    let force = support::as_bool(normalized_obj.get("force"), false);
    let timeout_ms =
        support::as_u64(normalized_obj.get("timeout_ms"), 15_000).clamp(1_000, 120_000);

    let auth = support::resolve_auth(payload);
    let auth_mode =
        support::clean_token(auth.get("mode").and_then(Value::as_str), "unauthenticated");
    let auth_headers = support::auth_headers_from(&auth);

    if pr > 0 {
        let plan = handle_build_pr_review_fetch_plan(payload_obj(&json!({
            "owner": owner,
            "repo": repo,
            "pr": pr
        })));
        let requests = plan
            .get("requests")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let mut pr_json = Value::Null;
        let mut files_json = Value::Null;
        let mut pr_bytes = 0u64;
        let mut files_bytes = 0u64;
        let mut pr_error = String::new();
        let mut files_error = String::new();

        for request in requests {
            let request_obj = match request.as_object() {
                Some(v) => v,
                None => continue,
            };
            let key = support::clean_text(request_obj.get("key").and_then(Value::as_str), 40);
            let url = support::clean_text(request_obj.get("url").and_then(Value::as_str), 800);
            if key.is_empty() || url.is_empty() {
                continue;
            }
            match support::curl_fetch_with_status(
                &url,
                timeout_ms,
                &auth_headers,
                "application/vnd.github+json",
            ) {
                Ok((status, body, bytes)) => {
                    if status >= 400 {
                        let code = support::http_status_to_code(status).to_string();
                        if key == "pr" {
                            pr_error = code;
                        } else if key == "files" {
                            files_error = code;
                        }
                        continue;
                    }
                    if key == "pr" {
                        pr_json = support::parse_json_or_null(&body);
                        pr_bytes = bytes;
                    } else if key == "files" {
                        files_json = support::parse_json_or_null(&body);
                        files_bytes = bytes;
                    }
                }
                Err(err) => {
                    let code = support::clean_text(Some(&err), 120)
                        .split(':')
                        .next()
                        .unwrap_or("collector_error")
                        .to_string();
                    if key == "pr" {
                        pr_error = code;
                    } else if key == "files" {
                        files_error = code;
                    }
                }
            }
        }

        return Ok(handle_collect_pr_review(payload_obj(&json!({
            "owner": owner,
            "repo": repo,
            "pr": pr,
            "auth_mode": auth_mode,
            "pr_json": pr_json,
            "files_json": files_json,
            "pr_bytes": pr_bytes,
            "files_bytes": files_bytes,
            "pr_fetch_error": if pr_error.is_empty() { Value::Null } else { Value::String(pr_error) },
            "files_fetch_error": if files_error.is_empty() { Value::Null } else { Value::String(files_error) }
        }))));
    }

    let prepared = handle_prepare_repo_activity(
        root,
        payload_obj(&json!({
            "owner": owner,
            "repo": repo,
            "min_hours": min_hours,
            "force": force,
            "state_dir": payload.get("state_dir").cloned().unwrap_or(Value::Null)
        })),
    );
    if prepared.get("ok").and_then(Value::as_bool) != Some(true) {
        return Ok(prepared);
    }
    if prepared.get("skipped").and_then(Value::as_bool) == Some(true) {
        return Ok(prepared);
    }

    let plan = handle_build_repo_activity_fetch_plan(payload_obj(&json!({
        "owner": owner,
        "repo": repo
    })));
    let requests = plan
        .get("requests")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut release_json = Value::Null;
    let mut commits_json = Value::Array(Vec::new());
    let mut pulls_json = Value::Array(Vec::new());
    let mut release_bytes = 0u64;
    let mut commits_bytes = 0u64;
    let mut pulls_bytes = 0u64;

    for request in requests {
        let request_obj = match request.as_object() {
            Some(v) => v,
            None => continue,
        };
        let key = support::clean_text(request_obj.get("key").and_then(Value::as_str), 40);
        let url = support::clean_text(request_obj.get("url").and_then(Value::as_str), 800);
        if key.is_empty() || url.is_empty() {
            continue;
        }
        if let Ok((status, body, bytes)) = support::curl_fetch_with_status(
            &url,
            timeout_ms,
            &auth_headers,
            "application/vnd.github+json",
        ) {
            if status >= 400 {
                continue;
            }
            let parsed = support::parse_json_or_null(&body);
            if key == "release" {
                release_json = parsed;
                release_bytes = bytes;
            } else if key == "commits" {
                commits_json = parsed;
                commits_bytes = bytes;
            } else if key == "pulls" {
                pulls_json = parsed;
                pulls_bytes = bytes;
            }
        }
    }

    handle_finalize_repo_activity(
        root,
        payload_obj(&json!({
            "owner": owner,
            "repo": repo,
            "max_items": max_items,
            "min_hours": min_hours,
            "auth_mode": auth_mode,
            "state_dir": payload.get("state_dir").cloned().unwrap_or(Value::Null),
            "cache": prepared.get("cache").cloned().unwrap_or(Value::Null),
            "release_json": release_json,
            "release_bytes": release_bytes,
            "commits_json": commits_json,
            "commits_bytes": commits_bytes,
            "pulls_json": pulls_json,
            "pulls_bytes": pulls_bytes
        })),
    )
}

fn dispatch(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "run" => handle_run(root, payload),
        "resolve-run-params" => Ok(handle_resolve_run_params(payload)),
        "resolve-auth" => Ok(support::resolve_auth(payload)),
        "prepare-repo-activity" => Ok(handle_prepare_repo_activity(root, payload)),
        "build-repo-activity-fetch-plan" => Ok(handle_build_repo_activity_fetch_plan(payload)),
        "finalize-repo-activity" => handle_finalize_repo_activity(root, payload),
        "collect-repo-activity" => handle_collect_repo_activity(root, payload),
        "build-pr-review-fetch-plan" => Ok(handle_build_pr_review_fetch_plan(payload)),
        "build-pr-review" => Ok(handle_build_pr_review(payload)),
        "collect-pr-review" => Ok(handle_collect_pr_review(payload)),
        "file-risk-flags" => Ok(handle_file_risk_flags(payload)),
        _ => Err("github_repo_collector_kernel_unknown_command".to_string()),
    }
}
