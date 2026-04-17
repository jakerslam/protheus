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
        "web-tooling-status" => {
            let provider_hint = clean_text(
                &arg_value(argv, "--provider=").unwrap_or_else(|| "auto".to_string()),
                80,
            )
            .to_ascii_lowercase();
            let profile_path =
                root.join("client/runtime/local/state/ui/infring_dashboard/web_tooling_profile.json");
            let history_path = root.join(ACTION_HISTORY_REL);
            let action_history_lines = fs::read_to_string(&history_path)
                .ok()
                .map(|raw| raw.lines().count())
                .unwrap_or(0);
            let web_tooling_status = read_json_file(&root.join(SNAPSHOT_LATEST_REL))
                .and_then(|value| {
                    value
                        .pointer("/web_tooling/status")
                        .and_then(Value::as_str)
                        .map(|raw| clean_text(raw, 40))
                })
                .filter(|value| !value.is_empty());
            write_json_stdout(
                &json!({
                    "ok": true,
                    "type": "dashboard_git_authority_web_tooling_status",
                    "provider_hint": provider_hint,
                    "profile_path": profile_path.to_string_lossy().to_string(),
                    "profile_exists": profile_path.exists(),
                    "action_history_path": history_path.to_string_lossy().to_string(),
                    "action_history_lines": action_history_lines,
                    "snapshot_web_tooling_status": web_tooling_status,
                    "auth_sources": web_tooling_auth_sources_git_lane()
                }),
                flags.pretty,
            );
            0
        }
        "web-tooling-errors" => {
            let limit = arg_usize(argv, "--limit=", 24, 1, 200);
            let history_path = root.join(ACTION_HISTORY_REL);
            let rows = fs::read_to_string(&history_path)
                .ok()
                .map(|raw| {
                    raw.lines()
                        .rev()
                        .filter(|line| line.contains("error") || line.contains("web_tool_"))
                        .take(limit)
                        .map(|line| clean_text(line, 400))
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            write_json_stdout(
                &json!({
                    "ok": true,
                    "type": "dashboard_git_authority_web_tooling_errors",
                    "history_path": history_path.to_string_lossy().to_string(),
                    "count": rows.len(),
                    "rows": rows
                }),
                flags.pretty,
            );
            0
        }
        "web-tooling-probe" => {
            let query = arg_value(argv, "--query=")
                .or_else(|| arg_value(argv, "--q="))
                .unwrap_or_default();
            if query.trim().is_empty() {
                write_json_stdout(
                    &json!({
                        "ok": false,
                        "error": "query_required"
                    }),
                    flags.pretty,
                );
                return 2;
            }
            let domain = arg_value(argv, "--domain=").unwrap_or_default();
            let provider_hint = clean_text(
                &arg_value(argv, "--provider=").unwrap_or_else(|| "auto".to_string()),
                80,
            )
            .to_ascii_lowercase();
            let sanitized = sanitize_web_query_git_lane(&query);
            let canonical = canonicalize_web_query_git_lane(&sanitized, &domain);
            write_json_stdout(
                &json!({
                    "ok": true,
                    "type": "dashboard_git_authority_web_tooling_probe",
                    "provider_hint": provider_hint,
                    "query": {
                        "input": query,
                        "sanitized": sanitized,
                        "canonical": canonical
                    }
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

fn is_invisible_unicode_git_lane(ch: char) -> bool {
    let code = ch as u32;
    matches!(
        code,
        0x200B..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x2064
            | 0x206A..=0x206F
            | 0xFEFF
            | 0xE0000..=0xE007F
    )
}

fn sanitize_web_query_git_lane(raw: &str) -> String {
    let mut cleaned = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if is_invisible_unicode_git_lane(ch) {
            continue;
        }
        if ch.is_control() && ch != '\n' && ch != '\t' {
            continue;
        }
        cleaned.push(ch);
    }
    clean_text(&cleaned, 1200)
}

fn normalize_domain_hint_git_lane(raw: &str) -> String {
    let lowered = sanitize_web_query_git_lane(raw).to_ascii_lowercase();
    if lowered.is_empty() {
        return String::new();
    }
    let without_scheme = lowered
        .strip_prefix("https://")
        .or_else(|| lowered.strip_prefix("http://"))
        .unwrap_or(&lowered)
        .to_string();
    clean_text(without_scheme.split('/').next().unwrap_or(""), 140)
        .trim_matches('.')
        .to_string()
}

fn canonicalize_web_query_git_lane(query: &str, domain_hint: &str) -> String {
    let sanitized = sanitize_web_query_git_lane(query);
    if sanitized.is_empty() {
        return sanitized;
    }
    if sanitized.to_ascii_lowercase().contains("site:") {
        return sanitized;
    }
    let domain = normalize_domain_hint_git_lane(domain_hint);
    if domain.is_empty() {
        return sanitized;
    }
    format!("site:{domain} {sanitized}")
}

fn web_tooling_auth_sources_git_lane() -> Vec<String> {
    let mut rows = Vec::<String>::new();
    for (label, env_var) in [
        ("openai", "OPENAI_API_KEY"),
        ("github", "GITHUB_TOKEN"),
        ("github_app", "GITHUB_APP_INSTALLATION_TOKEN"),
        ("brave", "BRAVE_API_KEY"),
        ("tavily", "TAVILY_API_KEY"),
        ("perplexity", "PERPLEXITY_API_KEY"),
        ("exa", "EXA_API_KEY"),
    ] {
        let present = env::var(env_var)
            .ok()
            .map(|raw| !sanitize_web_query_git_lane(&raw).is_empty())
            .unwrap_or(false);
        if present {
            rows.push(label.to_string());
        }
    }
    rows.sort();
    rows.dedup();
    rows
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
