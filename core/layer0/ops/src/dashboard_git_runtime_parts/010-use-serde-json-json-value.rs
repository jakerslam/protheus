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

fn run_git_in_workspace(
    root: &Path,
    workspace: &Path,
    args: &[&str],
) -> Result<std::process::Output, String> {
    Command::new("git")
        .arg("-C")
        .arg(workspace)
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
    if let Ok(output) = run_git(
        root,
        &["show-ref", "--verify", "--quiet", "refs/heads/main"],
    ) {
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
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{cleaned}"),
        ],
    )
    .map(|output| output.status.success())
    .unwrap_or(false)
}

pub fn git_workspace_ready(root: &Path, workspace: &Path) -> bool {
    if !workspace.exists() || !workspace.is_dir() {
        return false;
    }
    run_git_in_workspace(root, workspace, &["rev-parse", "--is-inside-work-tree"])
        .map(|output| output.status.success())
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

fn git_workspace_branch(root: &Path, workspace: &Path) -> Option<String> {
    if !git_workspace_ready(root, workspace) {
        return None;
    }
    let output =
        run_git_in_workspace(root, workspace, &["rev-parse", "--abbrev-ref", "HEAD"]).ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = normalize_branch_name(&String::from_utf8_lossy(&output.stdout));
    if branch.is_empty() {
        None
    } else {
        Some(branch)
    }
}

fn git_error_detail(output: std::process::Output, fallback: &str) -> String {
    let detail = clean_text(
        &format!(
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
        280,
    );
    if detail.is_empty() {
        fallback.to_string()
    } else {
        detail
    }
}

fn git_optional_error_detail(output: Option<std::process::Output>, fallback: &str) -> String {
    output
        .map(|row| git_error_detail(row, fallback))
        .unwrap_or_else(|| fallback.to_string())
}

fn remove_workspace_dir_fallback(
    workspace: &Path,
    workspace_str: &str,
    removed_workspace_dirs: &mut Vec<String>,
) -> bool {
    if fs::remove_dir_all(workspace).is_ok() {
        removed_workspace_dirs.push(workspace_str.to_string());
        true
    } else {
        false
    }
}
