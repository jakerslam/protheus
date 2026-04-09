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

pub fn cleanup_agent_git_artifacts(
    root: &Path,
    agent_id: &str,
    branch_hint: Option<&str>,
) -> Value {
    let cleaned_agent = clean_text(agent_id, 160);
    if cleaned_agent.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }

    let agent_workspace_root = agent_git_trees_dir(root).join(&cleaned_agent);
    let mut workspace_paths = Vec::<PathBuf>::new();
    if let Ok(entries) = fs::read_dir(&agent_workspace_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                workspace_paths.push(path);
            }
        }
    }
    workspace_paths.sort();

    let mut candidate_branches = HashSet::<String>::new();
    if let Some(hint) = branch_hint {
        let normalized = normalize_branch_name(hint);
        if !normalized.is_empty() {
            candidate_branches.insert(normalized);
        }
    }
    for workspace in &workspace_paths {
        if let Some(branch) = git_workspace_branch(root, workspace) {
            candidate_branches.insert(branch);
        }
    }

    let mut removed_worktrees = Vec::<String>::new();
    let mut removed_workspace_dirs = Vec::<String>::new();
    let mut errors = Vec::<Value>::new();
    for workspace in &workspace_paths {
        let workspace_str = workspace.to_string_lossy().to_string();
        let remove_result = run_git(root, &["worktree", "remove", "--force", &workspace_str]);
        match remove_result {
            Ok(output) if output.status.success() => {
                removed_worktrees.push(workspace_str.clone());
            }
            Ok(output) => {
                if remove_workspace_dir_fallback(
                    workspace,
                    &workspace_str,
                    &mut removed_workspace_dirs,
                ) {
                } else {
                    errors.push(json!({
                        "stage": "worktree_remove",
                        "workspace_dir": workspace_str,
                        "error": git_error_detail(output, "git_worktree_remove_failed")
                    }));
                }
            }
            Err(err) => {
                if remove_workspace_dir_fallback(
                    workspace,
                    &workspace_str,
                    &mut removed_workspace_dirs,
                ) {
                } else {
                    errors.push(json!({
                        "stage": "worktree_remove",
                        "workspace_dir": workspace_str,
                        "error": clean_text(&err, 280)
                    }));
                }
            }
        }
    }
    let _ = run_git(root, &["worktree", "prune", "--expire=now"]);
    if let Ok(mut entries) = fs::read_dir(&agent_workspace_root) {
        if entries.next().is_none() {
            let _ = fs::remove_dir(&agent_workspace_root);
        }
    }

    let mut deleted_branches = Vec::<String>::new();
    let mut skipped_protected_branches = Vec::<String>::new();
    let mut skipped_missing_branches = Vec::<String>::new();
    let mut attempted_branches = candidate_branches.into_iter().collect::<Vec<_>>();
    attempted_branches.sort();
    let mut current_branch = git_current_branch(root, "main");
    let main_branch = git_main_branch(root, &current_branch);
    for branch in &attempted_branches {
        if branch == "main" || branch == "master" {
            skipped_protected_branches.push(branch.clone());
            continue;
        }
        if !git_branch_exists(root, branch) {
            skipped_missing_branches.push(branch.clone());
            continue;
        }
        if current_branch == *branch
            && main_branch != *branch
            && git_branch_exists(root, &main_branch)
        {
            if let Ok(output) = run_git(root, &["checkout", &main_branch]) {
                if output.status.success() {
                    current_branch = main_branch.clone();
                } else {
                    errors.push(json!({
                        "stage": "branch_checkout",
                        "branch": branch,
                        "error": git_error_detail(output, "git_checkout_main_failed")
                    }));
                }
            }
        }
        match run_git(root, &["branch", "-D", branch]) {
            Ok(output) if output.status.success() => {
                deleted_branches.push(branch.clone());
            }
            Ok(output) => {
                errors.push(json!({
                    "stage": "branch_delete",
                    "branch": branch,
                    "error": git_error_detail(output, "git_branch_delete_failed")
                }));
            }
            Err(err) => {
                errors.push(json!({
                    "stage": "branch_delete",
                    "branch": branch,
                    "error": clean_text(&err, 280)
                }));
            }
        }
    }

    json!({
        "ok": errors.is_empty(),
        "type": "dashboard_agent_git_cleanup",
        "agent_id": cleaned_agent,
        "workspace_root": agent_workspace_root.to_string_lossy().to_string(),
        "removed_worktrees": removed_worktrees,
        "removed_workspace_dirs": removed_workspace_dirs,
        "attempted_branches": attempted_branches,
        "deleted_branches": deleted_branches,
        "skipped_protected_branches": skipped_protected_branches,
        "skipped_missing_branches": skipped_missing_branches,
        "errors": errors
    })
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
    if output
        .as_ref()
        .map(|out| !out.status.success())
        .unwrap_or(true)
    {
        let _ = run_git(root, &["worktree", "prune", "--expire=now"]);
        output = run_git(root, &args);
    }
    if output
        .as_ref()
        .map(|out| !out.status.success())
        .unwrap_or(true)
        || !git_workspace_ready(root, &workspace)
    {
        let detail = git_optional_error_detail(output.ok(), "git_worktree_add_failed");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn init_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let status = Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .status()
            .expect("git init");
        assert!(status.success(), "git init should succeed");
        fs::write(dir.path().join("README.md"), "seed\n").expect("write seed file");
        let status = Command::new("git")
            .args(["add", "README.md"])
            .current_dir(dir.path())
            .status()
            .expect("git add");
        assert!(status.success(), "git add should succeed");
        let status = Command::new("git")
            .args([
                "-c",
                "user.name=Codex Test",
                "-c",
                "user.email=codex@test.local",
                "commit",
                "-m",
                "init",
            ])
            .current_dir(dir.path())
            .status()
            .expect("git commit");
        assert!(status.success(), "git commit should succeed");
        dir
    }

    #[test]
    fn cleanup_agent_git_artifacts_removes_worktree_and_branch() {
        let root = init_repo();
        let branch = "agent-test-feature";
        let switched = switch_agent_worktree(root.path(), "agent-test", branch, true);
        assert_eq!(switched.get("ok").and_then(Value::as_bool), Some(true));
        assert!(git_branch_exists(root.path(), branch));
        let workspace = workspace_for_agent_branch(root.path(), "agent-test", branch);
        assert!(workspace.exists());

        let cleanup = cleanup_agent_git_artifacts(root.path(), "agent-test", Some(branch));
        assert_eq!(cleanup.get("ok").and_then(Value::as_bool), Some(true));
        assert!(!git_branch_exists(root.path(), branch));
        assert!(!workspace.exists());
    }

    #[test]
    fn cleanup_agent_git_artifacts_preserves_protected_main_branch() {
        let root = init_repo();
        let main_branch = git_current_branch(root.path(), "main");
        let cleanup = cleanup_agent_git_artifacts(root.path(), "agent-main", Some(&main_branch));
        assert_eq!(cleanup.get("ok").and_then(Value::as_bool), Some(true));
        assert!(git_branch_exists(root.path(), &main_branch));
    }
}
