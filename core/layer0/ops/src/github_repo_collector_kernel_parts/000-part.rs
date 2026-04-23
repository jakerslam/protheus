// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::Utc;
use serde_json::{json, Map, Value};
use std::path::Path;

use crate::contract_lane_utils as lane_utils;
use crate::github_repo_collector_kernel_support as support;

fn usage() {
    println!("github-repo-collector-kernel commands:");
    println!("  infring-ops github-repo-collector-kernel run --payload-base64=<json>");
    println!(
        "  infring-ops github-repo-collector-kernel resolve-run-params --payload-base64=<json>"
    );
    println!("  infring-ops github-repo-collector-kernel resolve-auth --payload-base64=<json>");
    println!(
        "  infring-ops github-repo-collector-kernel prepare-repo-activity --payload-base64=<json>"
    );
    println!("  infring-ops github-repo-collector-kernel build-repo-activity-fetch-plan --payload-base64=<json>");
    println!("  infring-ops github-repo-collector-kernel finalize-repo-activity --payload-base64=<json>");
    println!(
        "  infring-ops github-repo-collector-kernel collect-repo-activity --payload-base64=<json>"
    );
    println!("  infring-ops github-repo-collector-kernel build-pr-review-fetch-plan --payload-base64=<json>");
    println!("  infring-ops github-repo-collector-kernel build-pr-review --payload-base64=<json>");
    println!(
        "  infring-ops github-repo-collector-kernel collect-pr-review --payload-base64=<json>"
    );
    println!("  infring-ops github-repo-collector-kernel file-risk-flags --payload-base64=<json>");
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
