
fn handle_finalize_repo_activity(
    root: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let (owner, repo) = owner_repo(payload);
    if owner.is_empty() || repo.is_empty() {
        return Ok(json!({"ok": false, "error": "missing_owner_or_repo"}));
    }

    let max_items = support::as_u64(payload.get("max_items"), 10).clamp(1, 50) as usize;
    let min_hours = support::as_f64(payload.get("min_hours"), 4.0).clamp(0.0, 24.0 * 365.0);
    let auth_mode = support::clean_token(
        payload.get("auth_mode").and_then(Value::as_str),
        "unauthenticated",
    );

    let key = support::cache_key(&owner, &repo);
    let fp = support::cache_path(root, payload, &key);
    let cache = payload
        .get("cache")
        .cloned()
        .unwrap_or_else(|| support::load_cache(&fp));
    let seen = support::seen_ids_set(&cache);

    let mut items = Vec::<Value>::new();
    if let Some(release) = payload.get("release_json").and_then(Value::as_object) {
        if let Some(item) = support::map_release_item(&owner, &repo, release, &seen) {
            items.push(item);
        }
    }

    let commits = payload
        .get("commits_json")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    items.extend(support::map_commit_items(&owner, &repo, &commits, &seen));

    let pulls = payload
        .get("pulls_json")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    items.extend(support::map_pr_items(&owner, &repo, &pulls, &seen));

    let bytes = support::as_u64(payload.get("release_bytes"), 0)
        .saturating_add(support::as_u64(payload.get("commits_bytes"), 0))
        .saturating_add(support::as_u64(payload.get("pulls_bytes"), 0));

    let mut new_seen = cache
        .as_object()
        .and_then(|o| o.get("seen_ids"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for item in &items {
        if let Some(id) = item
            .as_object()
            .and_then(|o| o.get("id"))
            .and_then(Value::as_str)
        {
            new_seen.push(Value::String(support::clean_text(Some(id), 64)));
        }
    }
    if new_seen.len() > 1000 {
        let skip = new_seen.len() - 1000;
        new_seen = new_seen.into_iter().skip(skip).collect::<Vec<_>>();
    }
    let next_cache = json!({"last_run": support::now_iso(), "seen_ids": new_seen});
    support::save_cache(&fp, &next_cache)?;

    Ok(json!({
        "ok": true,
        "success": true,
        "eye": "github_repo",
        "mode": "repo_activity",
        "auth_mode": auth_mode,
        "owner": owner,
        "repo": repo,
        "items": items.into_iter().take(max_items).collect::<Vec<_>>(),
        "bytes": bytes,
        "duration_ms": 0,
        "requests": 3,
        "cadence_hours": min_hours,
        "sample": Value::Null,
        "cache_key": key,
        "cache_path": fp.display().to_string(),
    }))
}

fn handle_build_pr_review_fetch_plan(payload: &Map<String, Value>) -> Value {
    let (owner, repo, pr_number) = owner_repo_pr(payload);
    if owner.is_empty() || repo.is_empty() || pr_number == 0 {
        return json!({"ok": false, "error": "missing_owner_repo_or_pr"});
    }

    let base_url = format!("https://api.github.com/repos/{owner}/{repo}");
    json!({
        "ok": true,
        "owner": owner,
        "repo": repo,
        "pr": pr_number,
        "base_url": base_url,
        "requests": [
            {
                "key": "pr",
                "url": format!("{}/pulls/{}", base_url, pr_number),
                "required": true,
                "accept": "application/vnd.github+json",
            },
            {
                "key": "files",
                "url": format!("{}/pulls/{}/files?per_page=100", base_url, pr_number),
                "required": true,
                "accept": "application/vnd.github+json",
            }
        ]
    })
}

fn handle_build_pr_review(payload: &Map<String, Value>) -> Value {
    let (owner, repo, pr_number) = owner_repo_pr(payload);
    if owner.is_empty() || repo.is_empty() || pr_number == 0 {
        return json!({"ok": false, "error": "missing_owner_repo_or_pr"});
    }

    let auth_mode = support::clean_token(
        payload.get("auth_mode").and_then(Value::as_str),
        "unauthenticated",
    );
    let pr_obj = payload
        .get("pr_json")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let files = payload
        .get("files_json")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let head_sha = pr_obj
        .get("head")
        .and_then(Value::as_object)
        .and_then(|h| h.get("sha"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let review_id = support::sha16(&format!("pr_review:{owner}/{repo}#{pr_number}:{head_sha}"));

    let additions = support::as_u64(pr_obj.get("additions"), 0);
    let deletions = support::as_u64(pr_obj.get("deletions"), 0);
    let changed_files = support::as_u64(pr_obj.get("changed_files"), files.len() as u64);

    let file_sample = files
        .iter()
        .take(8)
        .filter_map(Value::as_object)
        .map(|row| {
            json!({
                "filename": support::clean_text(row.get("filename").and_then(Value::as_str), 200),
                "status": support::clean_text(row.get("status").and_then(Value::as_str), 40),
                "additions": support::as_u64(row.get("additions"), 0),
                "deletions": support::as_u64(row.get("deletions"), 0),
                "changes": support::as_u64(row.get("changes"), 0),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "ok": true,
        "success": true,
        "eye": "github_repo",
        "mode": "pr_review",
        "auth_mode": auth_mode,
        "owner": owner,
        "repo": repo,
        "pr": pr_number,
        "review": {
            "id": review_id,
            "title": support::clean_text(pr_obj.get("title").and_then(Value::as_str), 220),
            "url": support::clean_text(pr_obj.get("html_url").and_then(Value::as_str), 500),
            "state": support::clean_text(pr_obj.get("state").and_then(Value::as_str), 40),
            "draft": pr_obj.get("draft").and_then(Value::as_bool).unwrap_or(false),
            "author": support::clean_text(pr_obj.get("user").and_then(Value::as_object).and_then(|u| u.get("login")).and_then(Value::as_str), 120),
            "files_changed": changed_files,
            "additions": additions,
            "deletions": deletions,
            "risk_flags": support::file_risk_flags(&files),
            "file_sample": file_sample,
        },
        "bytes": support::as_u64(payload.get("pr_bytes"), 0).saturating_add(support::as_u64(payload.get("files_bytes"), 0)),
        "requests": 2,
        "duration_ms": 0
    })
}
