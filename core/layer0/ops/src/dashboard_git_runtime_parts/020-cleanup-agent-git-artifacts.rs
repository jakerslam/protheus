
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
