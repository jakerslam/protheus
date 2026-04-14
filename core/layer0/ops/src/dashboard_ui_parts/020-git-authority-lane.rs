fn run_git_authority(root: &Path, flags: &Flags, argv: &[String]) -> i32 {
    let action = arg_value(argv, "--git-action=").unwrap_or_default();
    if action.is_empty() {
        let payload = json!({
            "ok": false,
            "error": "git_action_required"
        });
        write_json_stdout(&payload, flags.pretty);
        return 2;
    }

    let fallback_branch =
        arg_value(argv, "--fallback-branch=").unwrap_or_else(|| "main".to_string());
    match action.as_str() {
        "current-branch" => {
            let branch = git_current_branch(root, &fallback_branch);
            write_json_stdout(
                &json!({
                    "ok": true,
                    "branch": branch
                }),
                flags.pretty,
            );
            0
        }
        "main-branch" => {
            let branch = git_main_branch(root, &fallback_branch);
            write_json_stdout(
                &json!({
                    "ok": true,
                    "branch": branch
                }),
                flags.pretty,
            );
            0
        }
        "branch-exists" => {
            let branch = normalize_branch_name(&arg_value(argv, "--branch=").unwrap_or_default());
            let exists = git_branch_exists(root, &branch);
            write_json_stdout(
                &json!({
                    "ok": true,
                    "branch": branch,
                    "exists": exists
                }),
                flags.pretty,
            );
            0
        }
        "list-branches" => {
            let limit = arg_usize(argv, "--limit=", 240, 8, 2000);
            let (main, branches) = list_git_branches(root, limit, &fallback_branch);
            write_json_stdout(
                &json!({
                    "ok": true,
                    "main_branch": main,
                    "branches": branches
                }),
                flags.pretty,
            );
            0
        }
        "list-tracked-files" => {
            let limit = arg_usize(argv, "--limit=", 500, 20, 10_000);
            let path_prefix =
                clean_text(&arg_value(argv, "--path-prefix=").unwrap_or_default(), 1024);
            let mut rows = Vec::<String>::new();
            if let Ok(output) = run_git(root, &["ls-files"]) {
                if output.status.success() {
                    rows = String::from_utf8_lossy(&output.stdout)
                        .split('\n')
                        .map(|line| clean_text(line, 1024))
                        .filter(|line| !line.is_empty())
                        .collect();
                }
            }
            if !path_prefix.is_empty() {
                rows.retain(|line| line.starts_with(&path_prefix));
            }
            let total_count = rows.len();
            let truncated = total_count > limit;
            if truncated {
                rows.truncate(limit);
            }
            write_json_stdout(
                &json!({
                    "ok": true,
                    "files": rows,
                    "count": total_count,
                    "truncated": truncated,
                    "path_prefix": path_prefix
                }),
                flags.pretty,
            );
            0
        }
        "workspace-ready" => {
            let raw_workspace = arg_value(argv, "--workspace=").unwrap_or_default();
            let workspace = resolve_absolute_path(root, &raw_workspace);
            let inside = is_agent_workspace_path(root, &workspace);
            let ready = inside && git_workspace_ready(root, &workspace);
            write_json_stdout(
                &json!({
                    "ok": true,
                    "workspace_dir": workspace.to_string_lossy().to_string(),
                    "inside_agent_tree": inside,
                    "ready": ready
                }),
                flags.pretty,
            );
            0
        }
        "ensure-workspace-ready" => {
            let branch = normalize_branch_name(&arg_value(argv, "--branch=").unwrap_or_default());
            let raw_workspace = arg_value(argv, "--workspace=").unwrap_or_default();
            let workspace = resolve_absolute_path(root, &raw_workspace);
            if branch.is_empty() || !is_agent_workspace_path(root, &workspace) {
                write_json_stdout(
                    &json!({
                        "ok": false,
                        "error": "invalid_git_tree_binding",
                        "branch": branch,
                        "workspace_dir": workspace.to_string_lossy().to_string()
                    }),
                    flags.pretty,
                );
                return 0;
            }
            if git_workspace_ready(root, &workspace) {
                write_json_stdout(
                    &json!({
                        "ok": true,
                        "created": false,
                        "branch": branch,
                        "workspace_dir": workspace.to_string_lossy().to_string()
                    }),
                    flags.pretty,
                );
                return 0;
            }

            if workspace.exists() && workspace.is_dir() {
                let _ = fs::remove_dir_all(&workspace);
            }
            if let Some(parent) = workspace.parent() {
                let _ = fs::create_dir_all(parent);
            }

            let branch_exists = git_branch_exists(root, &branch);
            let workspace_str = workspace.to_string_lossy().to_string();
            let mut args = vec!["worktree", "add", "--force"];
            if branch_exists {
                args.push(&workspace_str);
                args.push(&branch);
            } else {
                args.push("-b");
                args.push(&branch);
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
                write_json_stdout(
                    &json!({
                        "ok": false,
                        "error": detail,
                        "branch": branch,
                        "workspace_dir": workspace_str
                    }),
                    flags.pretty,
                );
                return 0;
            }

            write_json_stdout(
                &json!({
                    "ok": true,
                    "created": true,
                    "branch": branch,
                    "workspace_dir": workspace_str
                }),
                flags.pretty,
            );
            0
        }
        "remove-workspace" => {
            let raw_workspace = arg_value(argv, "--workspace=").unwrap_or_default();
            let workspace = resolve_absolute_path(root, &raw_workspace);
            let inside = is_agent_workspace_path(root, &workspace);
            if !inside || !workspace.exists() {
                write_json_stdout(
                    &json!({
                        "ok": true,
                        "removed": false,
                        "reason": "no_isolated_workspace",
                        "workspace_dir": workspace.to_string_lossy().to_string()
                    }),
                    flags.pretty,
                );
                return 0;
            }

            let workspace_str = workspace.to_string_lossy().to_string();
            let removed = Command::new("git")
                .args(["worktree", "remove", "--force", &workspace_str])
                .current_dir(root)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
            let removed = if removed {
                true
            } else {
                fs::remove_dir_all(&workspace).is_ok()
            };
            let _ = run_git(root, &["worktree", "prune", "--expire=now"]);
            write_json_stdout(
                &json!({
                    "ok": true,
                    "removed": removed,
                    "workspace_dir": workspace_str
                }),
                flags.pretty,
            );
            0
        }
        "delete-branch" => {
            let branch = normalize_branch_name(&arg_value(argv, "--branch=").unwrap_or_default());
            let main = normalize_branch_name(
                &arg_value(argv, "--main-branch=").unwrap_or_else(|| "main".to_string()),
            );
            let branch_in_use = arg_bool(argv, "--branch-in-use=", false);

            if branch.is_empty() {
                write_json_stdout(
                    &json!({
                        "ok": true,
                        "attempted": false,
                        "removed": false,
                        "reason": "no_isolated_branch",
                        "branch": ""
                    }),
                    flags.pretty,
                );
                return 0;
            }
            if !main.is_empty() && branch == main {
                write_json_stdout(
                    &json!({
                        "ok": true,
                        "attempted": false,
                        "removed": false,
                        "reason": "main_branch_protected",
                        "branch": branch
                    }),
                    flags.pretty,
                );
                return 0;
            }
            if branch_in_use {
                write_json_stdout(
                    &json!({
                        "ok": true,
                        "attempted": false,
                        "removed": false,
                        "reason": "branch_in_use",
                        "branch": branch
                    }),
                    flags.pretty,
                );
                return 0;
            }
            if !git_branch_exists(root, &branch) {
                write_json_stdout(
                    &json!({
                        "ok": true,
                        "attempted": false,
                        "removed": false,
                        "reason": "branch_missing",
                        "branch": branch
                    }),
                    flags.pretty,
                );
                return 0;
            }

            match run_git(root, &["branch", "-D", &branch]) {
                Ok(output) if output.status.success() => {
                    write_json_stdout(
                        &json!({
                            "ok": true,
                            "attempted": true,
                            "removed": true,
                            "reason": "deleted",
                            "branch": branch,
                            "detail": ""
                        }),
                        flags.pretty,
                    );
                    0
                }
                Ok(output) => {
                    let detail = clean_text(
                        &format!(
                            "{} {}",
                            String::from_utf8_lossy(&output.stdout),
                            String::from_utf8_lossy(&output.stderr)
                        ),
                        240,
                    );
                    write_json_stdout(
                        &json!({
                            "ok": false,
                            "attempted": true,
                            "removed": false,
                            "reason": "git_branch_delete_failed",
                            "branch": branch,
                            "detail": detail
                        }),
                        flags.pretty,
                    );
                    0
                }
                Err(err) => {
                    write_json_stdout(
                        &json!({
                            "ok": false,
                            "attempted": true,
                            "removed": false,
                            "reason": "git_branch_delete_failed",
                            "branch": branch,
                            "detail": clean_text(&err, 240)
                        }),
                        flags.pretty,
                    );
                    0
                }
            }
        }
        _ => {
            write_json_stdout(
                &json!({
                    "ok": false,
                    "error": format!("unsupported_git_action:{action}")
                }),
                flags.pretty,
            );
            2
        }
    }
}

fn parse_json_loose(raw: &str) -> Option<Value> {
    let text = raw.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return Some(value);
    }
    for line in text.lines().rev() {
        let candidate = line.trim();
        if candidate.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(candidate) {
            return Some(value);
        }
    }
    None
}

fn read_json_file(path: &Path) -> Option<Value> {
    let body = fs::read_to_string(path).ok()?;
    parse_json_loose(&body)
}

fn read_cached_snapshot_component(root: &Path, key: &str) -> Option<Value> {
    let snapshot = read_json_file(&root.join(SNAPSHOT_LATEST_REL))?;
    snapshot.get(key).cloned()
}

fn run_lane(root: &Path, domain: &str, args: &[String]) -> LaneResult {
    let exe = match env::current_exe() {
        Ok(path) => path,
        Err(_) => {
            return LaneResult {
                ok: false,
                status: 1,
                argv: std::iter::once(domain.to_string())
                    .chain(args.iter().cloned())
                    .collect(),
                payload: None,
            };
        }
    };
    let output = Command::new(exe)
        .arg(domain)
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let argv = std::iter::once(domain.to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    match output {
        Ok(out) => {
            let status = out.status.code().unwrap_or(1);
            let payload = parse_json_loose(&String::from_utf8_lossy(&out.stdout));
            LaneResult {
                ok: status == 0 && payload.is_some(),
                status,
                argv,
                payload,
            }
        }
        Err(_) => LaneResult {
            ok: false,
            status: 1,
            argv,
            payload: None,
        },
    }
}

fn ensure_dir(path: &Path) {
    let _ = fs::create_dir_all(path);
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        ensure_dir(parent);
    }
    if let Ok(body) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{body}\n"));
    }
}

fn append_jsonl(path: &Path, value: &Value) {
    let _ = crate::v8_kernel::append_jsonl_without_binary_queue(path, value);
}
