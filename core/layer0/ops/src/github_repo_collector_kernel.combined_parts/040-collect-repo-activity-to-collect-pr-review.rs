
fn handle_collect_repo_activity(
    root: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let (owner, repo) = owner_repo(payload);
    if owner.is_empty() || repo.is_empty() {
        return Ok(json!({
            "ok": false,
            "success": false,
            "eye": "github_repo",
            "mode": "repo_activity",
            "error": "missing_owner_or_repo"
        }));
    }
    let max_items = support::as_u64(payload.get("max_items"), 10).clamp(1, 50);
    let min_hours = support::as_f64(payload.get("min_hours"), 4.0).clamp(0.0, 24.0 * 365.0);
    let force = support::as_bool(payload.get("force"), false);

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

    handle_finalize_repo_activity(
        root,
        payload_obj(&json!({
            "owner": owner,
            "repo": repo,
            "max_items": max_items,
            "min_hours": min_hours,
            "auth_mode": payload.get("auth_mode").cloned().unwrap_or(Value::String("unauthenticated".to_string())),
            "state_dir": payload.get("state_dir").cloned().unwrap_or(Value::Null),
            "cache": prepared.get("cache").cloned().unwrap_or(Value::Null),
            "release_json": payload.get("release_json").cloned().unwrap_or(Value::Null),
            "release_bytes": payload.get("release_bytes").cloned().unwrap_or(Value::from(0)),
            "commits_json": payload.get("commits_json").cloned().unwrap_or(Value::Array(Vec::new())),
            "commits_bytes": payload.get("commits_bytes").cloned().unwrap_or(Value::from(0)),
            "pulls_json": payload.get("pulls_json").cloned().unwrap_or(Value::Array(Vec::new())),
            "pulls_bytes": payload.get("pulls_bytes").cloned().unwrap_or(Value::from(0))
        })),
    )
}

fn handle_collect_pr_review(payload: &Map<String, Value>) -> Value {
    let (owner, repo, pr_number) = owner_repo_pr(payload);
    if owner.is_empty() || repo.is_empty() || pr_number == 0 {
        return json!({
            "ok": false,
            "success": false,

            "eye": "github_repo",
            "mode": "pr_review",
            "error": "missing_owner_repo_or_pr"
        });
    }
    let pr_fetch_error =
        support::clean_text(payload.get("pr_fetch_error").and_then(Value::as_str), 120);
    if !pr_fetch_error.is_empty() {
        return json!({
            "ok": false,
            "success": false,
            "eye": "github_repo",
            "mode": "pr_review",
            "owner": owner,
            "repo": repo,
            "pr": pr_number,
            "error": format!("pr_fetch_failed:{pr_fetch_error}")
        });
    }
    let files_fetch_error = support::clean_text(
        payload.get("files_fetch_error").and_then(Value::as_str),
        120,
    );
    if !files_fetch_error.is_empty() {
        return json!({
            "ok": false,
            "success": false,
            "eye": "github_repo",
            "mode": "pr_review",
            "owner": owner,
            "repo": repo,
            "pr": pr_number,
            "error": format!("files_fetch_failed:{files_fetch_error}")
        });
    }

    let has_pr = payload.get("pr_json").and_then(Value::as_object).is_some();
    let has_files = payload
        .get("files_json")
        .and_then(Value::as_array)
        .is_some();
    if !has_pr || !has_files {
        return json!({
            "ok": false,
            "success": false,
            "eye": "github_repo",
            "mode": "pr_review",
            "owner": owner,
            "repo": repo,
            "pr": pr_number,
            "error": "missing_required_pr_payload"
        });
    }

    handle_build_pr_review(payload_obj(&json!({
        "owner": owner,
        "repo": repo,
        "pr": pr_number,
        "auth_mode": payload.get("auth_mode").cloned().unwrap_or(Value::String("unauthenticated".to_string())),
        "pr_json": payload.get("pr_json").cloned().unwrap_or(Value::Null),
        "files_json": payload.get("files_json").cloned().unwrap_or(Value::Array(Vec::new())),
        "pr_bytes": payload.get("pr_bytes").cloned().unwrap_or(Value::from(0)),
        "files_bytes": payload.get("files_bytes").cloned().unwrap_or(Value::from(0))
    })))
}
