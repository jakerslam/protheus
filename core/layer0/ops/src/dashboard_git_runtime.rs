// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn clean_text(value: &str, max_len: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn branch_is_safe_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' || ch == '/'
}

pub fn normalize_branch_name(value: &str) -> String {
    let mut out = String::new();
    let mut prev_slash = false;
    for ch in clean_text(value, 160).chars() {
        let normalized = if branch_is_safe_char(ch) { ch } else { '-' };
        if normalized == '/' {
            if prev_slash {
                continue;
            }
            prev_slash = true;
        } else {
            prev_slash = false;
        }
        out.push(normalized);
    }
    out.trim_matches(|ch| ch == '-' || ch == '.' || ch == '/')
        .to_string()
}

fn branch_slug(branch: &str) -> String {
    normalize_branch_name(branch).replace('/', "__")
}

pub fn agent_git_trees_dir(root: &Path) -> PathBuf {
    let repo_name = root
        .file_name()
        .and_then(|v| v.to_str())
        .map(|v| clean_text(v, 80))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "workspace".to_string());
    root.parent()
        .unwrap_or(root)
        .join("agent_git_trees")
        .join(repo_name)
}

pub fn workspace_for_agent_branch(root: &Path, agent_id: &str, branch: &str) -> PathBuf {
    agent_git_trees_dir(root)
        .join(clean_text(agent_id, 160))
        .join(branch_slug(branch))
}

fn run_git(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .map_err(|err| format!("git_spawn_failed:{err}"))
}

pub fn git_current_branch(root: &Path, fallback: &str) -> String {
    if let Ok(output) = run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"]) {
        if output.status.success() {
            let branch = clean_text(&String::from_utf8_lossy(&output.stdout), 120);
            if !branch.is_empty() {
                return branch;
            }
        }
    }
    let cleaned = normalize_branch_name(fallback);
    if cleaned.is_empty() {
        "main".to_string()
    } else {
        cleaned
    }
}

pub fn git_main_branch(root: &Path, fallback: &str) -> String {
    if let Ok(output) = run_git(root, &["show-ref", "--verify", "--quiet", "refs/heads/main"]) {
        if output.status.success() {
            return "main".to_string();
        }
    }
    git_current_branch(root, fallback)
}

pub fn git_branch_exists(root: &Path, branch: &str) -> bool {
    let cleaned = normalize_branch_name(branch);
    if cleaned.is_empty() {
        return false;
    }
    run_git(
        root,
        &["show-ref", "--verify", "--quiet", &format!("refs/heads/{cleaned}")],
    )
    .map(|output| output.status.success())
    .unwrap_or(false)
}

pub fn git_workspace_ready(root: &Path, workspace: &Path) -> bool {
    if !workspace.exists() || !workspace.is_dir() {
        return false;
    }
    Command::new("git")
        .args([
            "-C",
            &workspace.to_string_lossy(),
            "rev-parse",
            "--is-inside-work-tree",
        ])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn list_git_branches(root: &Path, limit: usize, fallback_main: &str) -> (String, Vec<String>) {
    let cap = limit.clamp(8, 2000);
    let mut rows = Vec::<String>::new();
    if let Ok(output) = run_git(
        root,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(refname:short)",
            "refs/heads",
        ],
    ) {
        if output.status.success() {
            rows = String::from_utf8_lossy(&output.stdout)
                .split('\n')
                .map(normalize_branch_name)
                .filter(|row| !row.is_empty())
                .collect();
        }
    }
    let main = git_main_branch(root, fallback_main);
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<String>::new();
    if !main.is_empty() {
        seen.insert(main.clone());
        out.push(main.clone());
    }
    for branch in rows {
        if seen.insert(branch.clone()) {
            out.push(branch);
        }
        if out.len() >= cap {
            break;
        }
    }
    (main, out)
}

pub fn workspace_rel(root: &Path, workspace: &Path) -> String {
    workspace
        .strip_prefix(root)
        .ok()
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}

pub fn switch_agent_worktree(
    root: &Path,
    agent_id: &str,
    branch: &str,
    require_new: bool,
) -> Value {
    let cleaned_branch = normalize_branch_name(branch);
    if cleaned_branch.is_empty() {
        return json!({"ok": false, "error": "branch_required"});
    }
    if cleaned_branch == "main" || cleaned_branch == "master" {
        return json!({
            "ok": true,
            "branch": cleaned_branch,
            "kind": "master",
            "workspace_dir": root.to_string_lossy().to_string(),
            "workspace_rel": "",
            "ready": true,
            "created": false,
            "error": ""
        });
    }

    let workspace = workspace_for_agent_branch(root, agent_id, &cleaned_branch);
    if git_workspace_ready(root, &workspace) {
        return json!({
            "ok": true,
            "branch": cleaned_branch,
            "kind": "isolated",
            "workspace_dir": workspace.to_string_lossy().to_string(),
            "workspace_rel": workspace_rel(root, &workspace),
            "ready": true,
            "created": false,
            "error": ""
        });
    }

    if workspace.exists() && workspace.is_dir() {
        let _ = fs::remove_dir_all(&workspace);
    }
    if let Some(parent) = workspace.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let workspace_str = workspace.to_string_lossy().to_string();
    let branch_exists = git_branch_exists(root, &cleaned_branch);
    let mut args = vec!["worktree", "add", "--force"];
    if require_new && branch_exists {
        return json!({
            "ok": false,
            "error": "branch_already_exists",
            "branch": cleaned_branch,
            "workspace_dir": workspace_str,
            "ready": false
        });
    }
    if branch_exists {
        args.push(&workspace_str);
        args.push(&cleaned_branch);
    } else {
        args.push("-b");
        args.push(&cleaned_branch);
        args.push(&workspace_str);
        args.push("HEAD");
    }
    let mut output = run_git(root, &args);
    if output.as_ref().map(|out| !out.status.success()).unwrap_or(true) {
        let _ = run_git(root, &["worktree", "prune", "--expire=now"]);
        output = run_git(root, &args);
    }
    if output.as_ref().map(|out| !out.status.success()).unwrap_or(true)
        || !git_workspace_ready(root, &workspace)
    {
        let detail = output
            .ok()
            .map(|out| {
                clean_text(
                    &format!(
                        "{} {}",
                        String::from_utf8_lossy(&out.stdout),
                        String::from_utf8_lossy(&out.stderr)
                    ),
                    280,
                )
            })
            .filter(|row| !row.is_empty())
            .unwrap_or_else(|| "git_worktree_add_failed".to_string());
        return json!({
            "ok": false,
            "error": detail,
            "branch": cleaned_branch,
            "workspace_dir": workspace_str,
            "ready": false
        });
    }

    json!({
        "ok": true,
        "branch": cleaned_branch,
        "kind": "isolated",
        "workspace_dir": workspace_str,
        "workspace_rel": workspace_rel(root, &workspace),
        "ready": true,
        "created": true,
        "error": ""
    })
}
