
fn usage() {
    println!("github-repo-collector-kernel commands:");
    println!("  protheus-ops github-repo-collector-kernel run --payload-base64=<json>");
    println!(
        "  protheus-ops github-repo-collector-kernel resolve-run-params --payload-base64=<json>"
    );
    println!("  protheus-ops github-repo-collector-kernel resolve-auth --payload-base64=<json>");
    println!(
        "  protheus-ops github-repo-collector-kernel prepare-repo-activity --payload-base64=<json>"
    );
    println!("  protheus-ops github-repo-collector-kernel build-repo-activity-fetch-plan --payload-base64=<json>");
    println!("  protheus-ops github-repo-collector-kernel finalize-repo-activity --payload-base64=<json>");
    println!(
        "  protheus-ops github-repo-collector-kernel collect-repo-activity --payload-base64=<json>"
    );
    println!("  protheus-ops github-repo-collector-kernel build-pr-review-fetch-plan --payload-base64=<json>");
    println!("  protheus-ops github-repo-collector-kernel build-pr-review --payload-base64=<json>");
    println!(
        "  protheus-ops github-repo-collector-kernel collect-pr-review --payload-base64=<json>"
    );
    println!("  protheus-ops github-repo-collector-kernel file-risk-flags --payload-base64=<json>");
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    lane_utils::payload_obj(value)
}

fn owner_repo(payload: &Map<String, Value>) -> (String, String) {
    (
        support::clean_text(payload.get("owner").and_then(Value::as_str), 160),
        support::clean_text(payload.get("repo").and_then(Value::as_str), 160),
    )
}

fn owner_repo_pr(payload: &Map<String, Value>) -> (String, String, u64) {
    let (owner, repo) = owner_repo(payload);
    let pr_number = support::as_u64(payload.get("pr"), 0);
    (owner, repo, pr_number)
}

fn handle_resolve_run_params(payload: &Map<String, Value>) -> Value {
    let (owner, repo) = owner_repo(payload);
    let pr = support::as_u64(payload.get("pr"), 0);
    let max_items = support::as_u64(payload.get("max_items"), 10).clamp(1, 50);
    let min_hours = support::as_f64(payload.get("min_hours"), 4.0).clamp(0.0, 24.0 * 365.0);
    let force = support::as_bool(payload.get("force"), false);
    let timeout_ms = support::as_u64(payload.get("timeout_ms"), 15_000).clamp(1_000, 120_000);
    let mode = if pr > 0 { "pr_review" } else { "repo_activity" };
    if owner.is_empty() || repo.is_empty() {
        return json!({
            "ok": false,
            "error": "missing_owner_or_repo",
            "mode": mode
        });
    }
    json!({
        "ok": true,
        "owner": owner,
        "repo": repo,
        "pr": if pr > 0 { Value::Number(pr.into()) } else { Value::Null },
        "mode": mode,
        "max_items": max_items,
        "min_hours": min_hours,
        "force": force,
        "timeout_ms": timeout_ms
    })
}

fn handle_file_risk_flags(payload: &Map<String, Value>) -> Value {
    let files = payload
        .get("files")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    json!({"ok": true, "risk_flags": support::file_risk_flags(&files)})
}

fn handle_prepare_repo_activity(root: &Path, payload: &Map<String, Value>) -> Value {
    let (owner, repo) = owner_repo(payload);
    if owner.is_empty() || repo.is_empty() {
        return json!({"ok": false, "error": "missing_owner_or_repo"});
    }

    let force = support::as_bool(payload.get("force"), false);
    let min_hours = support::as_f64(payload.get("min_hours"), 4.0).clamp(0.0, 24.0 * 365.0);
    let key = support::cache_key(&owner, &repo);
    let fp = support::cache_path(root, payload, &key);
    let cache = support::load_cache(&fp);

    let hours_since = support::cache_last_run(&cache)
        .map(|dt| (Utc::now() - dt).num_seconds() as f64 / 3600.0)
        .unwrap_or(f64::INFINITY);

    let skipped = !force && hours_since < min_hours;
    if skipped {
        return json!({
            "ok": true,
            "success": true,
            "skipped": true,
            "reason": "cadence",
            "hours_since_last": (hours_since * 100.0).round() / 100.0,
            "min_hours": min_hours,
            "cache_key": key,
            "cache_path": fp.display().to_string(),
            "cache": cache,
        });
    }

    json!({
        "ok": true,
        "success": true,
        "skipped": false,
        "cache_key": key,
        "cache_path": fp.display().to_string(),
        "cache": cache,
    })
}

fn handle_build_repo_activity_fetch_plan(payload: &Map<String, Value>) -> Value {
    let (owner, repo) = owner_repo(payload);
    if owner.is_empty() || repo.is_empty() {
        return json!({"ok": false, "error": "missing_owner_or_repo"});
    }

    let base_url = format!("https://api.github.com/repos/{owner}/{repo}");
    json!({
        "ok": true,
        "owner": owner,
        "repo": repo,
        "base_url": base_url,
        "requests": [
            {
                "key": "release",
                "url": format!("{}/releases/latest", base_url),
                "required": false,
                "accept": "application/vnd.github+json",
            },
            {
                "key": "commits",
                "url": format!("{}/commits?per_page=5", base_url),
                "required": false,
                "accept": "application/vnd.github+json",
            },
            {
                "key": "pulls",
                "url": format!("{}/pulls?state=open&per_page=5", base_url),
                "required": false,
                "accept": "application/vnd.github+json",
            }
        ]
    })
}
