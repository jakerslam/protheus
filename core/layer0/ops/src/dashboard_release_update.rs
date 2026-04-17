// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
        (Some((am, an, ap, apre)), Some((bm, bn, bp, bpre))) => am
            .cmp(&bm)
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
            }),
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

fn release_check_base(current_version: &str) -> Value {
    json!({
        "type": "dashboard_release_check",
        "current_version": current_version,
        "release_url": REPO_RELEASE_URL,
        "update_channel": "git_semver_tags",
        "runtime_web_tooling": release_runtime_web_tooling_snapshot()
    })
}

fn release_runtime_web_tooling_auth_sources() -> Vec<String> {
    let env_candidates = [
        "BRAVE_API_KEY",
        "EXA_API_KEY",
        "TAVILY_API_KEY",
        "PERPLEXITY_API_KEY",
        "SERPAPI_API_KEY",
        "GOOGLE_SEARCH_API_KEY",
        "GOOGLE_CSE_ID",
        "FIRECRAWL_API_KEY",
        "XAI_API_KEY",
        "MOONSHOT_API_KEY",
        "OPENAI_API_KEY",
    ];
    let mut sources = Vec::<String>::new();
    for env_name in env_candidates {
        let present = std::env::var(env_name)
            .ok()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        if present {
            sources.push(format!("env:{env_name}"));
        }
    }
    sources
}

fn release_runtime_web_tooling_snapshot() -> Value {
    let auth_sources = release_runtime_web_tooling_auth_sources();
    json!({
        "strict_auth_required": std::env::var("INFRING_WEB_TOOLING_STRICT_AUTH")
            .ok()
            .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "y" | "on"))
            .unwrap_or(true),
        "auth_present": !auth_sources.is_empty(),
        "auth_sources": auth_sources
    })
}

pub fn check_update(root: &Path) -> Value {
    let local = read_package_version(root);
    let mut payload = release_check_base(&local);
    match latest_remote_tag(root) {
        Ok(remote) => {
            let has_update = semver_cmp(&remote, &local).is_gt();
            payload["ok"] = json!(true);
            payload["latest_version"] = json!(remote);
            payload["has_update"] = json!(has_update);
            payload
        }
        Err(error) => {
            payload["ok"] = json!(false);
            payload["error"] = json!(error);
            payload["diagnostic_code"] = json!("release_remote_tag_unavailable");
            payload
        }
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
            "message": "Refusing update apply on dirty workspace.",
            "runtime_web_tooling": release_runtime_web_tooling_snapshot()
        });
    }

    let fetch = Command::new("git")
        .args(["fetch", "--all", "--tags"])
        .current_dir(root)
        .output();
    let Ok(fetch) = fetch else {
        return json!({
            "ok": false,
            "type":"dashboard_release_apply",
            "error":"git_fetch_failed",
            "runtime_web_tooling": release_runtime_web_tooling_snapshot()
        });
    };
    if !fetch.status.success() {
        return json!({
            "ok": false,
            "type": "dashboard_release_apply",
            "error": "git_fetch_status_nonzero",
            "exit_code": fetch.status.code().unwrap_or(1),
            "runtime_web_tooling": release_runtime_web_tooling_snapshot()
        });
    }

    let pull = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(root)
        .output();
    let Ok(pull) = pull else {
        return json!({
            "ok": false,
            "type":"dashboard_release_apply",
            "error":"git_pull_failed",
            "runtime_web_tooling": release_runtime_web_tooling_snapshot()
        });
    };
    let ok = pull.status.success();
    json!({
        "ok": ok,
        "type": "dashboard_release_apply",
        "exit_code": pull.status.code().unwrap_or(if ok {0} else {1}),
        "stdout": clean_text(&String::from_utf8_lossy(&pull.stdout), 4000),
        "stderr": clean_text(&String::from_utf8_lossy(&pull.stderr), 4000),
        "post_check": check_update(root),
        "runtime_web_tooling": release_runtime_web_tooling_snapshot()
    })
}

fn update_apply_spawn_spec() -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-NonInteractive".to_string(),
                "-Command".to_string(),
                "git fetch --all --tags; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; git pull --ff-only".to_string(),
            ],
        )
    } else {
        (
            "sh".to_string(),
            vec![
                "-lc".to_string(),
                "git fetch --all --tags && git pull --ff-only".to_string(),
            ],
        )
    }
}

pub fn dispatch_update_apply(root: &Path) -> Value {
    if git_worktree_dirty(root) {
        return json!({
            "ok": false,
            "type": "dashboard_release_apply",
            "error": "worktree_not_clean",
            "message": "Refusing update apply on dirty workspace.",
            "runtime_web_tooling": release_runtime_web_tooling_snapshot()
        });
    }
    let current_version = read_package_version(root);
    let (program, args) = update_apply_spawn_spec();
    let mut command = Command::new(&program);
    command
        .args(&args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match command.spawn() {
        Ok(child) => json!({
            "ok": true,
            "type": "dashboard_release_apply",
            "queued": true,
            "dispatch_mode": "detached_subprocess",
            "pid": child.id(),
            "command": program,
            "argv": args,
            "current_version": current_version,
            "runtime_web_tooling": release_runtime_web_tooling_snapshot()
        }),
        Err(err) => json!({
            "ok": false,
            "type": "dashboard_release_apply",
            "error": format!("update_apply_spawn_failed:{}", clean_text(&err.to_string(), 200)),
            "runtime_web_tooling": release_runtime_web_tooling_snapshot()
        }),
    }
}

fn is_dashboard_daemon_executable(exe: &Path) -> bool {
    let name = exe
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    name.contains("infringd") || name.contains("protheusd")
}

fn dashboard_system_action_args_for_exe(exe: &Path, action: &str) -> Result<Vec<String>, String> {
    let normalized = clean_text(action, 40).to_ascii_lowercase();
    match normalized.as_str() {
        "restart" => {
            if is_dashboard_daemon_executable(exe) {
                Ok(vec!["restart".to_string(), "--json".to_string()])
            } else {
                Ok(vec![
                    "daemon-control".to_string(),
                    "restart".to_string(),
                    "--json".to_string(),
                    "--dashboard-open=0".to_string(),
                ])
            }
        }
        "shutdown" | "stop" => {
            if is_dashboard_daemon_executable(exe) {
                Ok(vec!["stop".to_string(), "--json".to_string()])
            } else {
                Ok(vec![
                    "daemon-control".to_string(),
                    "stop".to_string(),
                    "--json".to_string(),
                ])
            }
        }
        other => Err(format!("unknown_dashboard_system_action:{other}")),
    }
}

fn dashboard_system_action_executables() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(current) = std::env::current_exe() {
        candidates.push(current);
    }
    candidates.push(PathBuf::from("infring-ops"));
    candidates.push(PathBuf::from("protheus-ops"));
    candidates
}

pub fn dispatch_system_action(root: &Path, action: &str) -> Value {
    let normalized = clean_text(action, 40).to_ascii_lowercase();
    let mut last_error = String::new();
    for exe in dashboard_system_action_executables() {
        let args = match dashboard_system_action_args_for_exe(&exe, &normalized) {
            Ok(args) => args,
            Err(err) => {
                last_error = err;
                continue;
            }
        };
        let mut command = Command::new(&exe);
        command
            .args(&args)
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        match command.spawn() {
            Ok(child) => {
                return json!({
                    "ok": true,
                    "type": "dashboard_system_action",
                    "action": normalized,
                    "dispatch_mode": "detached_subprocess",
                    "pid": child.id(),
                    "command": exe.file_name().and_then(|value| value.to_str()).unwrap_or_default(),
                    "argv": args,
                });
            }
            Err(err) => {
                last_error = format!(
                    "dashboard_system_action_spawn_failed:{}:{}",
                    exe.display(),
                    clean_text(&err.to_string(), 200)
                );
            }
        }
    }
    json!({
        "ok": false,
        "type": "dashboard_system_action",
        "action": normalized,
        "error": if last_error.is_empty() {
            "dashboard_system_action_spawn_unavailable".to_string()
        } else {
            last_error
        }
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

    #[test]
    fn dashboard_system_action_routes_ops_binary_through_daemon_control() {
        let exe = PathBuf::from("infring-ops");
        assert_eq!(
            dashboard_system_action_args_for_exe(&exe, "restart").expect("restart args"),
            vec![
                "daemon-control".to_string(),
                "restart".to_string(),
                "--json".to_string(),
                "--dashboard-open=0".to_string()
            ]
        );
        assert_eq!(
            dashboard_system_action_args_for_exe(&exe, "shutdown").expect("shutdown args"),
            vec![
                "daemon-control".to_string(),
                "stop".to_string(),
                "--json".to_string()
            ]
        );
    }

    #[test]
    fn dashboard_system_action_routes_daemon_binary_directly() {
        let exe = PathBuf::from("infringd");
        assert_eq!(
            dashboard_system_action_args_for_exe(&exe, "restart").expect("restart args"),
            vec!["restart".to_string(), "--json".to_string()]
        );
        assert_eq!(
            dashboard_system_action_args_for_exe(&exe, "shutdown").expect("shutdown args"),
            vec!["stop".to_string(), "--json".to_string()]
        );
    }

    #[test]
    fn update_apply_spawn_spec_includes_git_fetch_and_pull() {
        let (_program, args) = update_apply_spawn_spec();
        let joined = args.join(" ");
        assert!(joined.contains("git fetch --all --tags"));
        assert!(joined.contains("git pull --ff-only"));
    }

    #[test]
    fn dispatch_update_apply_rejects_dirty_worktree() {
        let root = tempfile::tempdir().expect("tempdir");
        fs::write(
            root.path().join("package.json"),
            r#"{"version":"0.3.10-alpha"}"#,
        )
        .expect("package");
        let payload = dispatch_update_apply(root.path());
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("worktree_not_clean")
        );
    }

    #[test]
    fn release_check_base_includes_release_contract_fields() {
        let payload = release_check_base("0.0.0");
        assert_eq!(
            payload.get("release_url").and_then(Value::as_str),
            Some(REPO_RELEASE_URL)
        );
        assert_eq!(
            payload.get("update_channel").and_then(Value::as_str),
            Some("git_semver_tags")
        );
    }
}
