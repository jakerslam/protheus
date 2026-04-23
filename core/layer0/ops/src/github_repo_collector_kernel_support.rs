// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::contract_lane_utils as lane_utils;

pub fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    crate::contract_lane_utils::clean_text(raw, max_len)
}

pub fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    lane_utils::clean_token(raw, fallback)
}

pub fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

pub fn sha16(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex::encode(digest)[..16].to_string()
}

pub fn as_u64(value: Option<&Value>, fallback: u64) -> u64 {
    value.and_then(Value::as_u64).unwrap_or(fallback)
}

pub fn as_f64(value: Option<&Value>, fallback: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(fallback)
}

pub fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn sanitize_cache_key_token(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
}

pub fn cache_key(owner: &str, repo: &str) -> String {
    format!(
        "github_repo_{}_{}",
        sanitize_cache_key_token(owner),
        sanitize_cache_key_token(repo)
    )
}

pub fn cache_path(root: &Path, payload: &Map<String, Value>, key: &str) -> PathBuf {
    resolve_state_dir(root, payload)
        .join("github_repo_cache")
        .join(format!("{key}.json"))
}

fn resolve_state_dir(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let raw = clean_text(payload.get("state_dir").and_then(Value::as_str), 320);
    if raw.is_empty() {
        root.join("local/workspace/state")
    } else {
        let candidate = PathBuf::from(raw);
        if candidate.is_absolute() {
            candidate
        } else {
            root.join(candidate)
        }
    }
}

pub fn load_cache(path: &Path) -> Value {
    let raw = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return json!({"last_run": Value::Null, "seen_ids": []}),
    };
    let parsed = serde_json::from_str::<Value>(&raw)
        .unwrap_or_else(|_| json!({"last_run": Value::Null, "seen_ids": []}));
    let last_run = parsed
        .as_object()
        .and_then(|o| o.get("last_run"))
        .and_then(Value::as_str)
        .map(|s| clean_text(Some(s), 64))
        .filter(|s| !s.is_empty())
        .map(Value::String)
        .unwrap_or(Value::Null);
    let seen_ids = parsed
        .as_object()
        .and_then(|o| o.get("seen_ids"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|s| clean_text(Some(s), 64))
                .filter(|s| !s.is_empty())
                .rev()
                .take(1000)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(Value::String)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "last_run": last_run,
        "seen_ids": seen_ids,
    })
}

pub fn save_cache(path: &Path, cache: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("github_repo_kernel_create_dir_failed:{err}"))?;
    }
    let pretty = serde_json::to_string_pretty(cache)
        .map_err(|err| format!("github_repo_kernel_encode_failed:{err}"))?;
    fs::write(path, format!("{pretty}\n"))
        .map_err(|err| format!("github_repo_kernel_write_failed:{err}"))
}

pub fn seen_ids_set(cache: &Value) -> HashSet<String> {
    cache
        .as_object()
        .and_then(|o| o.get("seen_ids"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|s| clean_text(Some(s), 64))
                .filter(|s| !s.is_empty())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default()
}

pub fn cache_last_run(cache: &Value) -> Option<DateTime<Utc>> {
    cache
        .as_object()
        .and_then(|o| o.get("last_run"))
        .and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

pub fn resolve_auth(payload: &Map<String, Value>) -> Value {
    let env_app_installation_token = std::env::var("GITHUB_APP_INSTALLATION_TOKEN").ok();
    let app_installation_token = clean_text(
        payload
            .get("github_app_installation_token")
            .or_else(|| payload.get("app_installation_token"))
            .and_then(Value::as_str)
            .or(env_app_installation_token.as_deref()),
        400,
    );
    if !app_installation_token.is_empty() {
        return json!({
            "ok": true,
            "mode": "github_app_installation_token",
            "headers": {
                "Authorization": format!("Bearer {}", app_installation_token)
            }
        });
    }

    let env_pat = std::env::var("GITHUB_TOKEN").ok();
    let pat = clean_text(
        payload
            .get("github_token")
            .or_else(|| payload.get("token"))
            .and_then(Value::as_str)
            .or(env_pat.as_deref()),
        400,
    );
    if !pat.is_empty() {
        return json!({
            "ok": true,
            "mode": "pat",
            "headers": {
                "Authorization": format!("Bearer {}", pat)
            }
        });
    }

    json!({"ok": true, "mode": "unauthenticated", "headers": {}})
}

pub fn auth_headers_from(auth: &Value) -> Vec<(String, String)> {
    auth.get("headers")
        .and_then(Value::as_object)
        .map(|headers| {
            headers
                .iter()
                .map(|(k, v)| (clean_text(Some(k), 120), clean_text(v.as_str(), 400)))
                .filter(|(k, v)| !k.is_empty() && !v.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub fn http_status_to_code(status: u64) -> &'static str {
    match status {
        401 => "auth_unauthorized",
        403 => "auth_forbidden",
        404 => "http_404",
        408 => "timeout",
        429 => "rate_limited",
        500..=u64::MAX => "http_5xx",
        400..=499 => "http_4xx",
        _ => "http_error",
    }
}

pub fn classify_curl_transport_error(stderr: &str) -> String {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("could not resolve host")
        || lower.contains("name or service not known")
        || lower.contains("temporary failure in name resolution")
    {
        return "dns_unreachable".to_string();
    }
    if lower.contains("connection refused") {
        return "connection_refused".to_string();
    }
    if lower.contains("operation timed out")
        || lower.contains("connection timed out")
        || lower.contains("timed out")
    {
        return "timeout".to_string();
    }
    if lower.contains("ssl") || lower.contains("tls") || lower.contains("certificate") {
        return "tls_error".to_string();
    }
    "collector_error".to_string()
}

pub fn curl_fetch_with_status(
    url: &str,
    timeout_ms: u64,
    headers: &[(String, String)],
    accept: &str,
) -> Result<(u64, String, u64), String> {
    let timeout_secs = ((timeout_ms.max(1_000) as f64) / 1_000.0).ceil() as u64;
    let mut cmd = Command::new("curl");
    cmd.arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg(timeout_secs.to_string())
        .arg("-H")
        .arg("User-Agent: Infring-Eyes/1.0")
        .arg("-H")
        .arg(format!("Accept: {accept}"));
    for (k, v) in headers {
        cmd.arg("-H").arg(format!("{k}: {v}"));
    }
    cmd.arg("-w")
        .arg("\n__INFRING_STATUS__:%{http_code}\n")
        .arg(url);

    let output = cmd
        .output()
        .map_err(|err| format!("collector_fetch_spawn_failed:{err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let code = classify_curl_transport_error(&stderr);
        return Err(format!("{code}:{}", clean_text(Some(&stderr), 220)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let marker = "\n__INFRING_STATUS__:";
    let marker_pos = stdout
        .rfind(marker)
        .ok_or_else(|| "collector_fetch_missing_status_marker".to_string())?;
    let body = stdout[..marker_pos].to_string();
    let status_raw = stdout[(marker_pos + marker.len())..]
        .lines()
        .next()
        .unwrap_or("0")
        .trim()
        .to_string();
    let status = status_raw.parse::<u64>().unwrap_or(0);
    let bytes = body.as_bytes().len() as u64;
    Ok((status, body, bytes))
}

pub fn parse_json_or_null(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw).unwrap_or(Value::Null)
}

pub fn file_risk_flags(rows: &[Value]) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    let mut total_delta = 0u64;
    let mut has_security = false;
    let mut has_schema = false;

    for row in rows {
        let obj = match row.as_object() {
            Some(v) => v,
            None => continue,
        };
        total_delta =
            total_delta.saturating_add(obj.get("changes").and_then(Value::as_u64).unwrap_or(0));
        let filename = obj
            .get("filename")
            .and_then(Value::as_str)
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if filename.contains("security")
            || filename.contains("auth")
            || filename.contains("token")
            || filename.contains("secret")
            || filename.contains("vault")
            || filename.contains("policy")
        {
            has_security = true;
        }
        if filename.contains("migration")
            || filename.contains("schema")
            || filename.contains(".sql")
        {
            has_schema = true;
        }
    }

    if rows.len() >= 40 || total_delta >= 2000 {
        out.push(Value::String("large_diff".to_string()));
    }
    if has_security {
        out.push(Value::String("security_sensitive_paths".to_string()));
    }
    if has_schema {
        out.push(Value::String("schema_or_data_migration".to_string()));
    }
    out
}

fn canonical_repo_url(owner: &str, repo: &str, raw: Option<&str>, suffix: &str) -> String {
    let cleaned = clean_text(raw, 500);
    if cleaned.starts_with("https://github.com/") { cleaned } else { format!("https://github.com/{owner}/{repo}/{suffix}") }
}

pub fn map_release_item(
    owner: &str,
    repo: &str,
    release: &Map<String, Value>,
    seen: &HashSet<String>,
) -> Option<Value> {
    let tag = clean_text(release.get("tag_name").and_then(Value::as_str), 80);
    if tag.is_empty() {
        return None;
    }
    let id = sha16(&format!("release-{owner}-{repo}-{tag}"));
    if seen.contains(&id) {
        return None;
    }
    Some(json!({
        "id": id,
        "collected_at": now_iso(),
        "url": canonical_repo_url(owner, repo, release.get("html_url").and_then(Value::as_str), &format!("releases/tag/{tag}")),
        "title": format!("{owner}/{repo}: {tag}"),
        "description": clean_text(Some(&format!(
            "Release: {}. {}",
            clean_text(release.get("name").and_then(Value::as_str).or_else(|| release.get("tag_name").and_then(Value::as_str)), 120),
            clean_text(release.get("body").and_then(Value::as_str), 200)
        )), 280),
        "type": "release",
        "tag_name": tag,
        "published_at": clean_text(release.get("published_at").and_then(Value::as_str), 120),
        "author": clean_text(release.get("author").and_then(Value::as_object).and_then(|o| o.get("login")).and_then(Value::as_str), 120),
        "signal_type": "repo_release",
        "signal": true,
        "source": "github_repo",
        "repo": format!("{owner}/{repo}"),
        "tags": ["github", "release", "software"],
        "topics": ["repo_activity", "releases"]
    }))
}

pub fn map_commit_items(
    owner: &str,
    repo: &str,
    commits: &[Value],
    seen: &HashSet<String>,
) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    for commit in commits.iter().take(3) {
        let obj = match commit.as_object() {
            Some(v) => v,
            None => continue,
        };
        let sha = clean_text(obj.get("sha").and_then(Value::as_str), 40);
        if sha.is_empty() {
            continue;
        }
        let id = sha16(&format!("commit-{owner}-{repo}-{sha}"));
        if seen.contains(&id) {
            continue;
        }
        let msg = obj
            .get("commit")
            .and_then(Value::as_object)
            .and_then(|c| c.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .split('\n')
            .next()
            .unwrap_or("");
        let author = obj
            .get("commit")
            .and_then(Value::as_object)
            .and_then(|c| c.get("author"))
            .and_then(Value::as_object);
        out.push(json!({
            "id": id,
            "collected_at": now_iso(),
            "url": canonical_repo_url(owner, repo, obj.get("html_url").and_then(Value::as_str), &format!("commit/{sha}")),
            "title": format!("{owner}/{repo}: {}", clean_text(Some(msg), 90)),
            "description": clean_text(Some(&format!(
                "Commit by {}",
                clean_text(author.and_then(|a| a.get("name")).and_then(Value::as_str), 120)
            )), 220),
            "type": "commit",
            "sha": clean_text(Some(&sha), 16),
            "author": clean_text(author.and_then(|a| a.get("name")).and_then(Value::as_str), 120),
            "date": clean_text(author.and_then(|a| a.get("date")).and_then(Value::as_str), 120),
            "signal_type": "repo_commit",
            "signal": false,
            "source": "github_repo",
            "repo": format!("{owner}/{repo}"),
            "tags": ["github", "commit"],
            "topics": ["repo_activity", "development"]
        }));
    }
    out
}

pub fn map_pr_items(
    owner: &str,
    repo: &str,
    pulls: &[Value],
    seen: &HashSet<String>,
) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    for pr in pulls.iter().take(3) {
        let obj = match pr.as_object() {
            Some(v) => v,
            None => continue,
        };
        let number = obj.get("number").and_then(Value::as_u64).unwrap_or(0);
        if number == 0 {
            continue;
        }
        let updated_at = clean_text(obj.get("updated_at").and_then(Value::as_str), 120);
        let id = sha16(&format!("pr-{owner}-{repo}-{number}-{updated_at}"));
        if seen.contains(&id) {
            continue;
        }
        let user = obj.get("user").and_then(Value::as_object);
        out.push(json!({
            "id": id,
            "collected_at": now_iso(),
            "url": canonical_repo_url(owner, repo, obj.get("html_url").and_then(Value::as_str), &format!("pull/{number}")),
            "title": format!("{owner}/{repo} PR #{number}: {}", clean_text(obj.get("title").and_then(Value::as_str), 120)),
            "description": clean_text(Some(&format!(
                "Open PR by {}; draft={}",
                clean_text(user.and_then(|u| u.get("login")).and_then(Value::as_str), 120),
                obj.get("draft").and_then(Value::as_bool).unwrap_or(false)
            )), 220),
            "type": "pull_request",
            "pr": number,
            "author": clean_text(user.and_then(|u| u.get("login")).and_then(Value::as_str), 120),
            "date": updated_at,
            "signal_type": "repo_pr_open",
            "signal": true,
            "source": "github_repo",
            "repo": format!("{owner}/{repo}"),
            "tags": ["github", "pull_request"],
            "topics": ["code_review", "repo_activity"]
        }));
    }
    out
}
