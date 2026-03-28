// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::Command;

const REPO_RELEASE_URL: &str = "https://github.com/protheuslabs/InfRing.git";

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn read_package_version(root: &Path) -> String {
    let body = fs::read_to_string(root.join("package.json")).unwrap_or_default();
    serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|v| v.get("version").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| "0.0.0".to_string())
}

fn parse_semver_parts(raw: &str) -> Option<(i64, i64, i64, String)> {
    let value = raw.trim().trim_start_matches('v');
    let mut split = value.splitn(2, '-');
    let core = split.next().unwrap_or("");
    let pre = split.next().unwrap_or("").to_string();
    let nums = core
        .split('.')
        .map(|row| row.parse::<i64>().ok())
        .collect::<Vec<_>>();
    if nums.len() != 3 || nums.iter().any(|v| v.is_none()) {
        return None;
    }
    Some((nums[0]?, nums[1]?, nums[2]?, pre))
}

fn semver_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let pa = parse_semver_parts(a);
    let pb = parse_semver_parts(b);
    match (pa, pb) {
        (Some((am, an, ap, apre)), Some((bm, bn, bp, bpre))) => {
            am.cmp(&bm)
                .then(an.cmp(&bn))
                .then(ap.cmp(&bp))
                .then_with(|| {
                    if apre.is_empty() && !bpre.is_empty() {
                        std::cmp::Ordering::Greater
                    } else if !apre.is_empty() && bpre.is_empty() {
                        std::cmp::Ordering::Less
                    } else {
                        apre.cmp(&bpre)
                    }
                })
        }
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => a.cmp(b),
    }
}

fn latest_remote_tag(root: &Path) -> Result<String, String> {
    let out = Command::new("git")
        .arg("ls-remote")
        .arg("--tags")
        .arg("--refs")
        .arg(REPO_RELEASE_URL)
        .current_dir(root)
        .output()
        .map_err(|err| format!("git_ls_remote_failed:{}", clean_text(&err.to_string(), 200)))?;
    if !out.status.success() {
        return Err(format!(
            "git_ls_remote_status:{}",
            out.status.code().unwrap_or(1)
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut tags = stdout
        .lines()
        .filter_map(|row| row.split('\t').nth(1))
        .filter_map(|refname| refname.strip_prefix("refs/tags/"))
        .map(|row| clean_text(row, 80))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if tags.is_empty() {
        return Err("no_tags_found".to_string());
    }
    tags.sort_by(|a, b| semver_cmp(a, b));
    Ok(tags.last().cloned().unwrap_or_else(|| "v0.0.0".to_string()))
}

pub fn check_update(root: &Path) -> Value {
    let local = read_package_version(root);
    match latest_remote_tag(root) {
        Ok(remote) => {
            let has_update = semver_cmp(&remote, &local).is_gt();
            json!({
                "ok": true,
                "type": "dashboard_release_check",
                "current_version": local,
                "latest_version": remote,
                "has_update": has_update
            })
        }
        Err(error) => json!({
            "ok": false,
            "type": "dashboard_release_check",
            "current_version": local,
            "error": error
        }),
    }
}

fn git_worktree_dirty(root: &Path) -> bool {
    let out = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(root)
        .output();
    match out {
        Ok(out) if out.status.success() => !String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        _ => true,
    }
}

pub fn apply_update(root: &Path) -> Value {
    if git_worktree_dirty(root) {
        return json!({
            "ok": false,
            "type": "dashboard_release_apply",
            "error": "worktree_not_clean",
            "message": "Refusing update apply on dirty workspace."
        });
    }

    let fetch = Command::new("git")
        .args(["fetch", "--all", "--tags"])
        .current_dir(root)
        .output();
    let Ok(fetch) = fetch else {
        return json!({"ok": false, "type":"dashboard_release_apply", "error":"git_fetch_failed"});
    };
    if !fetch.status.success() {
        return json!({
            "ok": false,
            "type": "dashboard_release_apply",
            "error": "git_fetch_status_nonzero",
            "exit_code": fetch.status.code().unwrap_or(1)
        });
    }

    let pull = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(root)
        .output();
    let Ok(pull) = pull else {
        return json!({"ok": false, "type":"dashboard_release_apply", "error":"git_pull_failed"});
    };
    let ok = pull.status.success();
    json!({
        "ok": ok,
        "type": "dashboard_release_apply",
        "exit_code": pull.status.code().unwrap_or(if ok {0} else {1}),
        "stdout": clean_text(&String::from_utf8_lossy(&pull.stdout), 4000),
        "stderr": clean_text(&String::from_utf8_lossy(&pull.stderr), 4000),
        "post_check": check_update(root)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_compare_orders_versions() {
        assert!(semver_cmp("v0.5.0", "v0.4.9").is_gt());
        assert!(semver_cmp("v1.0.0", "v1.0.0-alpha").is_gt());
        assert!(semver_cmp("v0.5.0-alpha", "v0.5.0").is_lt());
    }

    #[test]
    fn parse_semver_supports_prerelease() {
        let row = parse_semver_parts("v0.5.0-alpha").expect("parse");
        assert_eq!(row.0, 0);
        assert_eq!(row.1, 5);
        assert_eq!(row.2, 0);
        assert_eq!(row.3, "alpha");
    }
}
