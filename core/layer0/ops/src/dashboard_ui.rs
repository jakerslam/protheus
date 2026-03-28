// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::cmp::Reverse;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;
use crate::dashboard_agent_state;
use crate::dashboard_compat_api;
use crate::dashboard_model_catalog;
use crate::dashboard_terminal_broker;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 4173;
const DEFAULT_TEAM: &str = "ops";
const DEFAULT_REFRESH_MS: u64 = 2000;
const MAX_REQUEST_BYTES: usize = 2_000_000;
const LOG_TAIL_MAX_READ_BYTES: usize = 256 * 1024;
const STATE_DIR_REL: &str = "client/runtime/local/state/ui/infring_dashboard";
const ACTION_DIR_REL: &str = "client/runtime/local/state/ui/infring_dashboard/actions";
const SNAPSHOT_LATEST_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/latest_snapshot.json";
const SNAPSHOT_HISTORY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/snapshot_history.jsonl";
const ACTION_LATEST_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/actions/latest.json";
const ACTION_HISTORY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/actions/history.jsonl";
#[cfg(test)]
const AGENT_PROFILES_REL: &str = "client/runtime/local/state/ui/infring_dashboard/agent_profiles.json";
#[cfg(test)]
const ARCHIVED_AGENTS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/archived_agents.json";
const RUNTIME_SYNC_MAX_BLOCKS: usize = 40;
const RUNTIME_SYNC_WARN_DEPTH: i64 = 50;
const RUNTIME_SYNC_BATCH_DEPTH: i64 = 75;
const RUNTIME_SYNC_DELTA_DEPTH: i64 = 50;
const RUNTIME_SYNC_SOFT_SCALE_DEPTH: i64 = 20;
const RUNTIME_SYNC_DRAIN_TRIGGER_DEPTH: i64 = 60;
const RUNTIME_SYNC_STALE_BLOCK_MS: i64 = 30_000;

#[derive(Debug, Clone)]
struct Flags {
    mode: String,
    host: String,
    port: u16,
    team: String,
    refresh_ms: u64,
    pretty: bool,
}

#[derive(Debug, Clone)]
struct LaneResult {
    ok: bool,
    status: i32,
    argv: Vec<String>,
    payload: Option<Value>,
}

#[derive(Debug, Clone)]
struct FileRow {
    rel_path: String,
    full_path: PathBuf,
    mtime_ms: i64,
    mtime: String,
    size_bytes: u64,
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn now_iso() -> String {
    crate::now_iso()
}

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

fn parse_positive_u16(raw: &str, fallback: u16) -> u16 {
    raw.parse::<u16>().ok().unwrap_or(fallback)
}

fn parse_positive_u64(raw: &str, fallback: u64, min: u64, max: u64) -> u64 {
    raw.parse::<u64>()
        .ok()
        .map(|n| n.clamp(min, max))
        .unwrap_or(fallback)
}

fn parse_flags(argv: &[String]) -> Flags {
    let mut out = Flags {
        mode: "serve".to_string(),
        host: DEFAULT_HOST.to_string(),
        port: DEFAULT_PORT,
        team: DEFAULT_TEAM.to_string(),
        refresh_ms: DEFAULT_REFRESH_MS,
        pretty: true,
    };
    let mut mode_set = false;
    for token in argv {
        let value = token.trim();
        if value.is_empty() {
            continue;
        }
        if !mode_set && !value.starts_with("--") {
            out.mode = value.to_ascii_lowercase();
            mode_set = true;
            continue;
        }
        if let Some(rest) = value.strip_prefix("--host=") {
            let parsed = clean_text(rest, 100);
            if !parsed.is_empty() {
                out.host = parsed;
            }
            continue;
        }
        if let Some(rest) = value.strip_prefix("--port=") {
            out.port = parse_positive_u16(rest, DEFAULT_PORT);
            continue;
        }
        if let Some(rest) = value.strip_prefix("--team=") {
            let parsed = clean_text(rest, 80);
            if !parsed.is_empty() {
                out.team = parsed;
            }
            continue;
        }
        if let Some(rest) = value.strip_prefix("--refresh-ms=") {
            out.refresh_ms = parse_positive_u64(rest, DEFAULT_REFRESH_MS, 800, 60_000);
            continue;
        }
        if value == "--pretty=0" || value == "--pretty=false" {
            out.pretty = false;
            continue;
        }
    }
    out
}

fn write_json_stdout(value: &Value, pretty: bool) {
    if pretty {
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!(
            "{}",
            serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
        );
    }
}

fn arg_value(argv: &[String], prefix: &str) -> Option<String> {
    argv.iter().find_map(|token| {
        token
            .strip_prefix(prefix)
            .map(|value| clean_text(value, 4096))
            .filter(|value| !value.is_empty())
    })
}

fn arg_bool(argv: &[String], prefix: &str, fallback: bool) -> bool {
    let Some(raw) = arg_value(argv, prefix) else {
        return fallback;
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn arg_usize(argv: &[String], prefix: &str, fallback: usize, min: usize, max: usize) -> usize {
    arg_value(argv, prefix)
        .and_then(|raw| raw.parse::<usize>().ok())
        .map(|n| n.clamp(min, max))
        .unwrap_or(fallback)
}

fn branch_is_safe_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' || ch == '/'
}

fn normalize_branch_name(value: &str) -> String {
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

fn resolve_absolute_path(root: &Path, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn agent_git_trees_dir(root: &Path) -> PathBuf {
    root.join(STATE_DIR_REL).join("agent_git_trees")
}

fn is_agent_workspace_path(root: &Path, workspace: &Path) -> bool {
    workspace.starts_with(agent_git_trees_dir(root))
}

fn run_git(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .map_err(|err| format!("git_spawn_failed:{err}"))
}

fn git_current_branch(root: &Path, fallback: &str) -> String {
    let out = run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"]);
    if let Ok(output) = out {
        if output.status.success() {
            let value = clean_text(&String::from_utf8_lossy(&output.stdout), 80);
            if !value.is_empty() {
                return value;
            }
        }
    }
    let fallback_clean = clean_text(fallback, 80);
    if fallback_clean.is_empty() {
        "main".to_string()
    } else {
        fallback_clean
    }
}

fn git_main_branch(root: &Path, fallback: &str) -> String {
    let out = run_git(
        root,
        &["show-ref", "--verify", "--quiet", "refs/heads/main"],
    );
    if let Ok(output) = out {
        if output.status.success() {
            return "main".to_string();
        }
    }
    git_current_branch(root, fallback)
}

fn git_branch_exists(root: &Path, branch: &str) -> bool {
    if branch.is_empty() {
        return false;
    }
    run_git(
        root,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
    .map(|out| out.status.success())
    .unwrap_or(false)
}

fn git_workspace_ready(root: &Path, workspace: &Path) -> bool {
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

fn list_git_branches(root: &Path, limit: usize, fallback_main: &str) -> (String, Vec<String>) {
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
            write_json_stdout(
                &json!({
                    "ok": true,
                    "files": rows
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
    if let Some(parent) = path.parent() {
        ensure_dir(parent);
    }
    if let Ok(line) = serde_json::to_string(value) {
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| writeln!(f, "{line}"));
    }
}

fn to_iso(ts: SystemTime) -> String {
    DateTime::<Utc>::from(ts).to_rfc3339()
}

fn file_rows(
    root: &Path,
    dir: &Path,
    max_depth: usize,
    limit: usize,
    include: &dyn Fn(&Path) -> bool,
) -> Vec<FileRow> {
    let mut rows = Vec::<FileRow>::new();
    for entry in WalkDir::new(dir)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !include(path) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let modified = meta.modified().unwrap_or(UNIX_EPOCH);
        let mtime_ms = modified
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let rel = path
            .strip_prefix(root)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        rows.push(FileRow {
            rel_path: rel,
            full_path: path.to_path_buf(),
            mtime_ms,
            mtime: to_iso(modified),
            size_bytes: meta.len(),
        });
    }
    rows.sort_by_key(|row| Reverse(row.mtime_ms));
    rows.truncate(limit);
    rows
}

fn read_tail_lines(path: &Path, max_lines: usize) -> Vec<String> {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return Vec::new(),
    };

    let len = file.metadata().ok().map(|meta| meta.len()).unwrap_or(0);
    if len == 0 {
        return Vec::new();
    }

    let take = len.min(LOG_TAIL_MAX_READ_BYTES as u64);
    if len > take {
        let _ = file.seek(SeekFrom::End(-(take as i64)));
    }

    let mut buf = Vec::<u8>::with_capacity(take as usize);
    if file.read_to_end(&mut buf).is_err() {
        return Vec::new();
    }

    let mut raw = String::from_utf8_lossy(&buf).to_string();
    if len > take {
        if let Some((_, rest)) = raw.split_once('\n') {
            raw = rest.to_string();
        }
    }

    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .rev()
        .take(max_lines)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn collect_log_events(root: &Path) -> Vec<Value> {
    let roots = [
        root.join("core/local/state/ops"),
        root.join("client/runtime/local/state"),
    ];
    let mut rows = Vec::<Value>::new();
    for base in roots {
        let files = file_rows(root, &base, 4, 8, &|path| {
            let rel = path.to_string_lossy();
            rel.ends_with(".jsonl")
        });
        for file in files {
            for line in read_tail_lines(&file.full_path, 8) {
                let payload = parse_json_loose(&line).unwrap_or(Value::Null);
                let ts = payload
                    .get("ts")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| file.mtime.clone());
                let message = payload
                    .get("type")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| clean_text(&line, 220));
                rows.push(json!({
                    "ts": ts,
                    "source": file.rel_path,
                    "message": message
                }));
            }
        }
    }
    rows.sort_by(|a, b| {
        b.get("ts")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(a.get("ts").and_then(Value::as_str).unwrap_or(""))
    });
    rows.truncate(40);
    rows
}

fn collect_receipts(root: &Path) -> Vec<Value> {
    let roots = [
        root.join("core/local/state/ops"),
        root.join("client/runtime/local/state"),
    ];
    let mut files = Vec::<FileRow>::new();
    for base in roots {
        files.extend(file_rows(root, &base, 4, 30, &|path| {
            let rel = path.to_string_lossy();
            rel.ends_with("latest.json")
                || rel.ends_with("history.jsonl")
                || rel.ends_with(".receipt.json")
        }));
    }
    files.sort_by_key(|row| Reverse(row.mtime_ms));
    files.truncate(32);
    files
        .into_iter()
        .map(|row| {
            json!({
                "kind": if row.rel_path.ends_with(".jsonl") { "timeline" } else { "receipt" },
                "path": row.rel_path,
                "mtime": row.mtime,
                "size_bytes": row.size_bytes
            })
        })
        .collect()
}

fn collect_memory_artifacts(root: &Path) -> Vec<Value> {
    let roots = [
        root.join("client/runtime/local/state"),
        root.join("core/local/state/ops"),
    ];
    let mut rows = Vec::<Value>::new();
    for base in roots {
        for row in file_rows(root, &base, 3, 20, &|path| {
            let rel = path.to_string_lossy();
            rel.ends_with("latest.json") || rel.ends_with(".jsonl") || rel.ends_with("queue.json")
        }) {
            rows.push(json!({
                "scope": if row.rel_path.contains("memory") { "memory" } else { "state" },
                "kind": if row.rel_path.ends_with(".jsonl") { "timeline" } else { "snapshot" },
                "path": row.rel_path,
                "mtime": row.mtime
            }));
        }
    }
    rows.sort_by(|a, b| {
        b.get("mtime")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(a.get("mtime").and_then(Value::as_str).unwrap_or(""))
    });
    rows.truncate(30);
    rows
}

fn metric_rows(health: &Value) -> Vec<Value> {
    let Some(metrics) = health.get("dashboard_metrics").and_then(Value::as_object) else {
        return Vec::new();
    };
    metrics
        .iter()
        .map(|(name, row)| {
            let status = row
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let target = row
                .get("target_max")
                .map(|v| format!("<= {v}"))
                .or_else(|| row.get("target_min").map(|v| format!(">= {v}")))
                .unwrap_or_else(|| "n/a".to_string());
            json!({
                "name": name,
                "status": status,
                "value": row.get("value").cloned().unwrap_or(Value::Null),
                "target": target
            })
        })
        .collect()
}

fn i64_from_value(value: Option<&Value>, fallback: i64) -> i64 {
    let parsed = value
        .and_then(|row| {
            row.as_i64()
                .or_else(|| row.as_u64().and_then(|n| i64::try_from(n).ok()))
                .or_else(|| row.as_f64().map(|n| n.round() as i64))
                .or_else(|| row.as_str().and_then(|s| s.trim().parse::<i64>().ok()))
        })
        .unwrap_or(fallback);
    parsed.max(0)
}

fn recommended_conduit_signals(
    queue_depth: i64,
    queue_utilization: f64,
    active_conduit_channels: i64,
    active_agents: i64,
) -> i64 {
    let depth = queue_depth.max(0);
    let util = queue_utilization.clamp(0.0, 1.0);
    let mut baseline = 4;
    if depth >= 95 || util >= 0.90 {
        baseline = 16;
    } else if depth >= 85 || util >= 0.82 {
        baseline = 14;
    } else if depth >= 65 || util >= 0.68 {
        baseline = 12;
    } else if depth >= RUNTIME_SYNC_WARN_DEPTH || util >= 0.58 {
        baseline = 8;
    } else if depth >= RUNTIME_SYNC_SOFT_SCALE_DEPTH || util >= 0.40 {
        baseline = 6;
    }

    let channels = active_conduit_channels.max(0);
    let conduit_floor = if channels > 0 {
        let bonus = if depth >= RUNTIME_SYNC_DRAIN_TRIGGER_DEPTH || util >= 0.65 {
            2
        } else if depth >= RUNTIME_SYNC_SOFT_SCALE_DEPTH || util >= 0.40 {
            1
        } else {
            0
        };
        (channels + bonus).clamp(4, 16)
    } else {
        4
    };

    let agents = active_agents.max(0);
    let agent_scale = if depth >= RUNTIME_SYNC_DRAIN_TRIGGER_DEPTH || util >= 0.65 {
        40
    } else if depth >= RUNTIME_SYNC_SOFT_SCALE_DEPTH || util >= 0.40 {
        120
    } else {
        400
    };
    let agent_floor = if agents > 0 {
        (4 + ((agents + agent_scale - 1) / agent_scale)).clamp(4, 24)
    } else {
        4
    };

    baseline.max(conduit_floor).max(agent_floor)
}

fn build_runtime_sync(root: &Path, flags: &Flags) -> Value {
    let team = if flags.team.trim().is_empty() {
        DEFAULT_TEAM.to_string()
    } else {
        clean_text(&flags.team, 80)
    };

    let cockpit = run_lane(
        root,
        "hermes-plane",
        &[
            "cockpit".to_string(),
            format!("--max-blocks={RUNTIME_SYNC_MAX_BLOCKS}"),
            "--strict=1".to_string(),
        ],
    );
    let attention_status = run_lane(root, "attention-queue", &["status".to_string()]);
    let attention_next = run_lane(
        root,
        "attention-queue",
        &[
            "next".to_string(),
            "--consumer=dashboard_mirror".to_string(),
            "--limit=32".to_string(),
            "--wait-ms=0".to_string(),
            "--run-context=dashboard_mirror".to_string(),
        ],
    );

    let cockpit_payload = cockpit.payload.unwrap_or_else(|| json!({}));
    let attention_status_payload = attention_status.payload.unwrap_or_else(|| json!({}));
    let attention_next_payload = attention_next.payload.unwrap_or_else(|| json!({}));

    let blocks = cockpit_payload
        .pointer("/cockpit/render/stream_blocks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .take(RUNTIME_SYNC_MAX_BLOCKS)
        .collect::<Vec<_>>();

    let cockpit_metrics = cockpit_payload
        .pointer("/cockpit/metrics")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let stale_threshold_ms = i64_from_value(
        cockpit_metrics.get("stale_block_threshold_ms"),
        RUNTIME_SYNC_STALE_BLOCK_MS,
    );
    let stale_measured = blocks
        .iter()
        .filter(|row| {
            let stale_flag = row
                .get("is_stale")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let duration = i64_from_value(row.get("duration_ms"), 0);
            stale_flag || duration >= stale_threshold_ms
        })
        .count() as i64;
    let total_block_count_value = cockpit_metrics
        .get("total_block_count")
        .cloned()
        .or_else(|| {
            cockpit_payload
                .pointer("/cockpit/render/total_blocks")
                .cloned()
        });
    let total_block_count = i64_from_value(total_block_count_value.as_ref(), blocks.len() as i64);
    let stale_block_count =
        i64_from_value(cockpit_metrics.get("stale_block_count"), stale_measured);
    let active_block_count = (total_block_count - stale_block_count).max(0);

    let conduit_detected_from_blocks = blocks
        .iter()
        .filter(|row| {
            let lane = row
                .get("lane")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let event_type = row
                .get("event_type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            lane.contains("conduit")
                || event_type.contains("conduit")
                || row
                    .get("conduit_enforced")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .count() as i64;
    let conduit_signals = i64_from_value(
        cockpit_metrics
            .get("conduit_signals_active")
            .or_else(|| cockpit_metrics.get("conduit_signals")),
        conduit_detected_from_blocks,
    );
    let conduit_channels_total = i64_from_value(
        cockpit_metrics.get("conduit_signals_total"),
        conduit_detected_from_blocks.max(conduit_signals),
    );
    let conduit_channels_observed = i64_from_value(
        cockpit_metrics.get("conduit_channels_observed"),
        conduit_signals,
    );

    let queue_depth = i64_from_value(
        attention_status_payload
            .get("queue_depth")
            .or_else(|| attention_next_payload.get("queue_depth")),
        0,
    );
    let attention_contract = attention_status_payload
        .get("attention_contract")
        .and_then(Value::as_object)
        .or_else(|| {
            attention_next_payload
                .get("attention_contract")
                .and_then(Value::as_object)
        })
        .cloned()
        .unwrap_or_default();
    let max_queue_depth = i64_from_value(attention_contract.get("max_queue_depth"), 2048).max(1);
    let queue_utilization = (queue_depth as f64 / max_queue_depth as f64).clamp(0.0, 1.0);
    let active_agents = i64_from_value(cockpit_metrics.get("active_agent_count"), 0);
    let target_conduit_signals = recommended_conduit_signals(
        queue_depth,
        queue_utilization,
        conduit_channels_observed,
        active_agents,
    );
    let conduit_scale_required = conduit_channels_observed < target_conduit_signals;
    let sync_mode = if queue_depth >= RUNTIME_SYNC_BATCH_DEPTH {
        "batch_sync"
    } else if queue_depth >= RUNTIME_SYNC_DELTA_DEPTH {
        "delta_sync"
    } else {
        "live_sync"
    };
    let pressure_level = if queue_depth >= max_queue_depth || queue_utilization >= 0.90 {
        "critical"
    } else if queue_depth >= RUNTIME_SYNC_BATCH_DEPTH || queue_utilization >= 0.75 {
        "high"
    } else if queue_depth >= RUNTIME_SYNC_WARN_DEPTH || queue_utilization >= 0.60 {
        "elevated"
    } else {
        "normal"
    };

    let events = attention_next_payload
        .get("events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut critical_visible_count = 0i64;
    let mut standard_count = 0i64;
    let mut background_count = 0i64;
    for row in &events {
        let lane = row
            .get("priority_lane")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let severity = row
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if lane == "critical" || severity == "critical" || severity == "error" {
            critical_visible_count += 1;
        } else if lane == "background" || severity == "background" {
            background_count += 1;
        } else {
            standard_count += 1;
        }
    }
    let lane_counts = attention_status_payload
        .get("lane_counts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let critical_total_count = i64_from_value(lane_counts.get("critical"), critical_visible_count)
        .max(critical_visible_count);
    let standard_total_count = i64_from_value(lane_counts.get("standard"), standard_count);
    let background_total_count = i64_from_value(lane_counts.get("background"), background_count);

    let mut out = json!({
        "ok": cockpit.ok && attention_status.ok && attention_next.ok,
        "type": "infring_dashboard_runtime_sync",
        "ts": now_iso(),
        "metadata": {
            "team": team,
            "authority": "rust_core_runtime_sync",
            "lanes": {
                "cockpit": cockpit.argv.join(" "),
                "attention_status": attention_status.argv.join(" "),
                "attention_next": attention_next.argv.join(" ")
            }
        },
        "team": team,
        "cockpit_ok": cockpit.ok,
        "attention_status_ok": attention_status.ok,
        "attention_next_ok": attention_next.ok,
        "lanes": {
            "cockpit": cockpit.argv.join(" "),
            "attention_status": attention_status.argv.join(" "),
            "attention_next": attention_next.argv.join(" ")
        },
        "cockpit": {
            "blocks": blocks,
            "block_count": active_block_count,
            "active_block_count": active_block_count,
            "total_block_count": total_block_count,
            "metrics": {
                "conduit_signals": conduit_signals,
                "conduit_signals_active": conduit_signals,
                "conduit_channels_observed": conduit_channels_observed,
                "conduit_signals_total": conduit_channels_total,
                "stale_block_count": stale_block_count,
                "stale_block_threshold_ms": stale_threshold_ms,
                "active_block_count": active_block_count,
                "total_block_count": total_block_count
            },
            "payload_type": cockpit_payload.get("type").cloned().unwrap_or(Value::Null),
            "receipt_hash": cockpit_payload.get("receipt_hash").cloned().unwrap_or(Value::Null)
        },
        "attention_queue": {
            "queue_depth": queue_depth,
            "events": events,
            "critical_visible_count": critical_visible_count,
            "critical_total_count": critical_total_count,
            "priority_counts": {
                "critical": critical_total_count,
                "standard": standard_total_count,
                "background": background_total_count,
                "telemetry": 0,
                "total": critical_total_count + standard_total_count + background_total_count
            },
            "lane_counts": {
                "critical": critical_total_count,
                "standard": standard_total_count,
                "background": background_total_count
            },
            "backpressure": {
                "level": pressure_level,
                "sync_mode": sync_mode,
                "max_queue_depth": max_queue_depth,
                "queue_utilization": queue_utilization,
                "conduit_signals": conduit_signals,
                "conduit_channels_total": conduit_channels_total,
                "conduit_channels_observed": conduit_channels_observed,
                "target_conduit_signals": target_conduit_signals,
                "scale_required": conduit_scale_required
            },
            "latest": attention_status_payload.get("latest").cloned().unwrap_or(Value::Null),
            "status_type": attention_status_payload.get("type").cloned().unwrap_or(Value::Null),
            "next_type": attention_next_payload.get("type").cloned().unwrap_or(Value::Null),
            "receipt_hashes": {
                "status": attention_status_payload.get("receipt_hash").cloned().unwrap_or(Value::Null),
                "next": attention_next_payload.get("receipt_hash").cloned().unwrap_or(Value::Null)
            }
        },
        "summary": {
            "queue_depth": queue_depth,
            "cockpit_blocks": active_block_count,
            "cockpit_total_blocks": total_block_count,
            "cockpit_stale_blocks": stale_block_count,
            "conduit_signals": conduit_signals,
            "conduit_channels_observed": conduit_channels_observed,
            "conduit_channels_total": conduit_channels_total,
            "target_conduit_signals": target_conduit_signals,
            "conduit_scale_required": conduit_scale_required,
            "attention_batch_count": critical_visible_count + standard_count + background_count,
            "sync_mode": sync_mode,
            "backpressure_level": pressure_level
        }
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn build_snapshot(root: &Path, flags: &Flags) -> Value {
    let team = if flags.team.trim().is_empty() {
        DEFAULT_TEAM.to_string()
    } else {
        clean_text(&flags.team, 80)
    };
    let contract_enforcement = dashboard_agent_state::enforce_expired_contracts(root);
    let app_payload = read_json_file(&root.join("core/local/state/ops/app_plane/latest.json"))
        .or_else(|| read_cached_snapshot_component(root, "app"))
        .unwrap_or_else(|| json!({}));

    let mut collab_payload = read_json_file(
        &root.join(format!("core/local/state/ops/collab_plane/dashboard/{team}.json")),
    )
    .map(|dashboard| {
        json!({
            "ok": true,
            "type": "collab_plane_dashboard",
            "dashboard": dashboard
        })
    })
    .or_else(|| read_cached_snapshot_component(root, "collab"))
    .unwrap_or_else(|| json!({}));
    dashboard_agent_state::merge_profiles_into_collab(root, &mut collab_payload, &team);

    let skills_payload = read_json_file(&root.join("core/local/state/ops/skills_plane/latest.json"))
        .or_else(|| read_cached_snapshot_component(root, "skills"))
        .unwrap_or_else(|| json!({}));

    let health_payload = read_cached_snapshot_component(root, "health").unwrap_or_else(|| {
        json!({
            "ok": true,
            "type": "health_status_dashboard_cache_fallback",
            "checks": {},
            "alerts": {},
            "dashboard_metrics": {}
        })
    });

    let mut out = json!({
        "ok": true,
        "type": "infring_dashboard_snapshot",
        "ts": now_iso(),
        "metadata": {
            "root": root.to_string_lossy().to_string(),
            "team": team,
            "refresh_ms": flags.refresh_ms,
            "authority": "rust_core_cached_runtime_state",
            "sources": {
                "app": "core/local/state/ops/app_plane/latest.json",
                "collab": format!("core/local/state/ops/collab_plane/dashboard/{team}.json"),
                "skills": "core/local/state/ops/skills_plane/latest.json",
                "health": "client/runtime/local/state/ui/infring_dashboard/latest_snapshot.json#health"
            }
        },
        "health": health_payload,
        "app": app_payload,
        "collab": collab_payload,
        "skills": skills_payload,
        "agents": {
            "session_summaries": dashboard_agent_state::session_summaries(root, 200),
            "contract_enforcement": contract_enforcement
        },
        "memory": {
            "entries": collect_memory_artifacts(root)
        },
        "receipts": {
            "recent": collect_receipts(root),
            "action_history_path": ACTION_HISTORY_REL
        },
        "logs": {
            "recent": collect_log_events(root)
        },
        "apm": {
            "metrics": [],
            "checks": {},
            "alerts": {}
        }
    });
    out["apm"]["metrics"] = Value::Array(metric_rows(&out["health"]));
    out["apm"]["checks"] = out["health"]
        .get("checks")
        .cloned()
        .unwrap_or_else(|| json!({}));
    out["apm"]["alerts"] = out["health"]
        .get("alerts")
        .cloned()
        .unwrap_or_else(|| json!({}));
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn write_snapshot_receipt(root: &Path, snapshot: &Value) {
    let latest = root.join(SNAPSHOT_LATEST_REL);
    let history = root.join(SNAPSHOT_HISTORY_REL);
    write_json(&latest, snapshot);
    append_jsonl(&history, snapshot);
}

fn ui_event_payload(event: &str, payload: Value) -> LaneResult {
    let mut out = json!({
        "ok": true,
        "type": "infring_dashboard_ui_event",
        "event": event,
        "ts": now_iso()
    });
    if let Some(map) = payload.as_object() {
        for (k, v) in map {
            out[k] = v.clone();
        }
    }
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    LaneResult {
        ok: true,
        status: 0,
        argv: vec![event.to_string()],
        payload: Some(out),
    }
}

fn run_action(root: &Path, action: &str, payload: &Value) -> LaneResult {
    let normalized = clean_text(action, 80);
    match normalized.as_str() {
        "dashboard.ui.toggleControls" => {
            let open = payload
                .get("open")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            ui_event_payload("toggle_controls", json!({ "open": open }))
        }
        "dashboard.ui.toggleSection" => {
            let section = payload
                .get("section")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "unknown".to_string());
            let open = payload
                .get("open")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            ui_event_payload(
                "toggle_section",
                json!({
                    "section": section,
                    "open": open
                }),
            )
        }
        "dashboard.ui.switchControlsTab" => {
            let tab = payload
                .get("tab")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 40))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "swarm".to_string());
            ui_event_payload("switch_controls_tab", json!({ "tab": tab }))
        }
        "app.switchProvider" => {
            let provider = payload
                .get("provider")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "openai".to_string());
            let model = payload
                .get("model")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 100))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "gpt-5".to_string());
            run_lane(
                root,
                "app-plane",
                &[
                    "switch-provider".to_string(),
                    "--app=chat-ui".to_string(),
                    format!("--provider={provider}"),
                    format!("--model={model}"),
                ],
            )
        }
        "app.chat" => {
            let input = payload
                .get("input")
                .and_then(Value::as_str)
                .or_else(|| payload.get("message").and_then(Value::as_str))
                .map(|v| clean_text(v, 2000))
                .unwrap_or_default();
            if input.is_empty() {
                return LaneResult {
                    ok: false,
                    status: 2,
                    argv: vec!["app-plane".to_string(), "run".to_string()],
                    payload: Some(json!({
                        "ok": false,
                        "type": "infring_dashboard_action_error",
                        "error": "chat_input_required"
                    })),
                };
            }
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "chat-ui-default-agent".to_string());
            let lane = run_lane(
                root,
                "app-plane",
                &[
                    "run".to_string(),
                    "--app=chat-ui".to_string(),
                    format!("--input={input}"),
                ],
            );
            if lane.ok {
                let assistant_text = lane
                    .payload
                    .as_ref()
                    .and_then(|row| row.get("response").and_then(Value::as_str))
                    .or_else(|| {
                        lane.payload
                            .as_ref()
                            .and_then(|row| row.get("output").and_then(Value::as_str))
                    })
                    .or_else(|| {
                        lane.payload.as_ref().and_then(|row| {
                            row.get("turns")
                                .and_then(Value::as_array)
                                .and_then(|turns| turns.last())
                                .and_then(|turn| turn.get("assistant").and_then(Value::as_str))
                        })
                    })
                    .unwrap_or("");
                let _ = dashboard_agent_state::append_turn(root, &agent_id, &input, assistant_text);
            }
            lane
        }
        "collab.launchRole" => {
            let team = payload
                .get("team")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| DEFAULT_TEAM.to_string());
            let role = payload
                .get("role")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "analyst".to_string());
            let shadow = payload
                .get("shadow")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| format!("{team}-{role}-shadow"));
            run_lane(
                root,
                "collab-plane",
                &[
                    "launch-role".to_string(),
                    format!("--team={team}"),
                    format!("--role={role}"),
                    format!("--shadow={shadow}"),
                ],
            )
        }
        "skills.run" => {
            let skill = payload
                .get("skill")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            if skill.is_empty() {
                return LaneResult {
                    ok: false,
                    status: 2,
                    argv: vec!["skills-plane".to_string(), "run".to_string()],
                    payload: Some(json!({
                        "ok": false,
                        "type": "infring_dashboard_action_error",
                        "error": "skill_required"
                    })),
                };
            }
            let input = payload
                .get("input")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 600))
                .unwrap_or_default();
            let mut args = vec!["run".to_string(), format!("--skill={skill}")];
            if !input.is_empty() {
                args.push(format!("--input={input}"));
            }
            run_lane(root, "skills-plane", &args)
        }
        "dashboard.assimilate" => {
            let target = payload
                .get("target")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "codex".to_string());
            run_lane(
                root,
                "app-plane",
                &[
                    "run".to_string(),
                    "--app=chat-ui".to_string(),
                    format!("--input=assimilate target {target} with receipt-first safety"),
                ],
            )
        }
        "dashboard.benchmark" => run_lane(root, "health-status", &["dashboard".to_string()]),
        "dashboard.models.catalog" => {
            let runtime_flags = Flags {
                mode: "snapshot".to_string(),
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                team: DEFAULT_TEAM.to_string(),
                refresh_ms: DEFAULT_REFRESH_MS,
                pretty: false,
            };
            let snapshot = build_snapshot(root, &runtime_flags);
            let result = dashboard_model_catalog::catalog_payload(root, &snapshot);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: 0,
                argv: vec!["dashboard.models.catalog".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.model.routeDecision" => {
            let runtime_flags = Flags {
                mode: "snapshot".to_string(),
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                team: DEFAULT_TEAM.to_string(),
                refresh_ms: DEFAULT_REFRESH_MS,
                pretty: false,
            };
            let snapshot = build_snapshot(root, &runtime_flags);
            let result = dashboard_model_catalog::route_decision_payload(root, &snapshot, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: 0,
                argv: vec!["dashboard.model.routeDecision".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.terminal.session.create" => {
            let result = dashboard_terminal_broker::create_session(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.terminal.session.create".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.terminal.exec" => {
            let result = dashboard_terminal_broker::exec_command(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: result
                    .get("exit_code")
                    .and_then(Value::as_i64)
                    .unwrap_or(if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                        0
                    } else {
                        2
                    }) as i32,
                argv: vec!["dashboard.terminal.exec".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.terminal.session.close" => {
            let session_id = payload
                .get("session_id")
                .or_else(|| payload.get("sessionId"))
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_terminal_broker::close_session(root, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.terminal.session.close".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.update.check" => {
            let result = crate::dashboard_release_update::check_update(root);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.update.check".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.update.apply" => {
            let result = crate::dashboard_release_update::apply_update(root);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.update.apply".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.runtime.executeSwarmRecommendation"
        | "dashboard.runtime.applyTelemetryRemediations" => {
            let team = payload
                .get("team")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| DEFAULT_TEAM.to_string());
            let action_key = if normalized == "dashboard.runtime.applyTelemetryRemediations" {
                "apply_telemetry_remediations"
            } else {
                "execute_swarm_recommendation"
            };
            let runtime_flags = Flags {
                mode: "runtime-sync".to_string(),
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                team: team.clone(),
                refresh_ms: DEFAULT_REFRESH_MS,
                pretty: false,
            };
            let runtime = build_runtime_sync(root, &runtime_flags);
            let summary = runtime
                .get("summary")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let queue_depth = i64_from_value(summary.get("queue_depth"), 0);
            let target_conduit_signals = i64_from_value(summary.get("target_conduit_signals"), 4);
            let conduit_scale_required = summary
                .get("conduit_scale_required")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let mut launch_receipt = Value::Null;
            if queue_depth >= RUNTIME_SYNC_DRAIN_TRIGGER_DEPTH {
                let shadow = format!("{team}-drain-{}", Utc::now().timestamp_millis());
                let launch = run_lane(
                    root,
                    "collab-plane",
                    &[
                        "launch-role".to_string(),
                        format!("--team={team}"),
                        "--role=analyst".to_string(),
                        format!("--shadow={shadow}"),
                    ],
                );
                launch_receipt = launch.payload.unwrap_or_else(|| {
                    json!({
                        "ok": launch.ok,
                        "status": launch.status,
                        "argv": launch.argv
                    })
                });
            }
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![
                    normalized.clone(),
                    format!("--team={team}"),
                ],
                payload: Some(json!({
                    "ok": true,
                    "type": "infring_dashboard_runtime_action",
                    "action": action_key,
                    "ts": now_iso(),
                    "team": team,
                    "queue_depth": queue_depth,
                    "target_conduit_signals": target_conduit_signals,
                    "conduit_scale_required": conduit_scale_required,
                    "launch_receipt": launch_receipt
                })),
            }
        }
        "dashboard.agent.upsertProfile" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::upsert_profile(root, &agent_id, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.upsertProfile".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.archive" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let reason = payload
                .get("reason")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::archive_agent(root, &agent_id, &reason);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.archive".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.unarchive" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::unarchive_agent(root, &agent_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.unarchive".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.upsertContract" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::upsert_contract(root, &agent_id, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.upsertContract".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.enforceContracts" => {
            let result = dashboard_agent_state::enforce_expired_contracts(root);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: 0,
                argv: vec!["dashboard.agent.enforceContracts".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.get" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::load_session(root, &agent_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.get".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.create" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let label = payload
                .get("label")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            let result = dashboard_agent_state::create_session(root, &agent_id, &label);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.create".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.switch" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let session_id = payload
                .get("session_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("sessionId").and_then(Value::as_str))
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::switch_session(root, &agent_id, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.switch".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.delete" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let session_id = payload
                .get("session_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("sessionId").and_then(Value::as_str))
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::delete_session(root, &agent_id, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.delete".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.appendTurn" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let user_text = payload
                .get("user")
                .and_then(Value::as_str)
                .or_else(|| payload.get("input").and_then(Value::as_str))
                .map(|v| clean_text(v, 2000))
                .unwrap_or_default();
            let assistant_text = payload
                .get("assistant")
                .and_then(Value::as_str)
                .or_else(|| payload.get("response").and_then(Value::as_str))
                .map(|v| clean_text(v, 4000))
                .unwrap_or_default();
            let result =
                dashboard_agent_state::append_turn(root, &agent_id, &user_text, &assistant_text);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.appendTurn".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.set" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let value = payload.get("value").cloned().unwrap_or(Value::Null);
            let result = dashboard_agent_state::memory_kv_set(root, &agent_id, &key, &value);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.set".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.get" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::memory_kv_get(root, &agent_id, &key);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.get".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.delete" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::memory_kv_delete(root, &agent_id, &key);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.delete".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.suggestions" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let user_hint = payload
                .get("user_hint")
                .and_then(Value::as_str)
                .or_else(|| payload.get("hint").and_then(Value::as_str))
                .map(|v| clean_text(v, 220))
                .unwrap_or_default();
            let result = dashboard_agent_state::suggestions(root, &agent_id, &user_hint);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.suggestions".to_string()],
                payload: Some(result),
            }
        }
        _ => LaneResult {
            ok: false,
            status: 2,
            argv: Vec::new(),
            payload: Some(json!({
                "ok": false,
                "type": "infring_dashboard_action_error",
                "error": format!("unsupported_action:{normalized}")
            })),
        },
    }
}

fn write_action_receipt(root: &Path, action: &str, payload: &Value, lane: &LaneResult) -> Value {
    let mut row = json!({
        "ok": lane.ok,
        "type": "infring_dashboard_action_receipt",
        "ts": now_iso(),
        "action": clean_text(action, 120),
        "payload": payload.clone(),
        "lane_status": lane.status,
        "lane_argv": lane.argv,
        "lane_receipt_hash": lane
            .payload
            .as_ref()
            .and_then(|v| v.get("receipt_hash"))
            .cloned()
            .unwrap_or(Value::Null)
    });
    row["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&row));
    write_json(&root.join(ACTION_LATEST_REL), &row);
    append_jsonl(&root.join(ACTION_HISTORY_REL), &row);
    row
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn parse_request(mut stream: &TcpStream) -> Result<HttpRequest, String> {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(2000)));
    let mut data = Vec::<u8>::new();
    let mut chunk = [0u8; 4096];
    let header_end;
    loop {
        let n = stream
            .read(&mut chunk)
            .map_err(|err| format!("request_read_failed:{err}"))?;
        if n == 0 {
            return Err("request_closed".to_string());
        }
        data.extend_from_slice(&chunk[..n]);
        if data.len() > MAX_REQUEST_BYTES {
            return Err("request_too_large".to_string());
        }
        if let Some(pos) = find_bytes(&data, b"\r\n\r\n") {
            header_end = pos;
            break;
        }
    }
    let header_raw = String::from_utf8_lossy(&data[..header_end]).to_string();
    let mut lines = header_raw.lines();
    let Some(first_line) = lines.next() else {
        return Err("request_line_missing".to_string());
    };
    let mut parts = first_line.split_whitespace();
    let method = parts
        .next()
        .map(|v| v.to_ascii_uppercase())
        .ok_or_else(|| "request_method_missing".to_string())?;
    let path = parts
        .next()
        .map(|v| v.to_string())
        .ok_or_else(|| "request_path_missing".to_string())?;

    let mut content_length = 0usize;
    let mut headers = Vec::<(String, String)>::new();
    for line in lines {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let key = k.trim().to_string();
        let value = v.trim().to_string();
        if !key.is_empty() {
            headers.push((key.clone(), value.clone()));
        }
        if key.eq_ignore_ascii_case("content-length") {
            content_length = value.parse::<usize>().unwrap_or(0);
        }
    }
    if content_length > MAX_REQUEST_BYTES {
        return Err("content_length_too_large".to_string());
    }

    let mut body = data[(header_end + 4)..].to_vec();
    while body.len() < content_length {
        let n = stream
            .read(&mut chunk)
            .map_err(|err| format!("request_body_read_failed:{err}"))?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
        if body.len() > MAX_REQUEST_BYTES {
            return Err("request_body_too_large".to_string());
        }
    }
    body.truncate(content_length);

    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn status_reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

fn write_response(
    mut stream: &TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
        status,
        status_reason(status),
        content_type,
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .map_err(|err| format!("response_head_write_failed:{err}"))?;
    stream
        .write_all(body)
        .map_err(|err| format!("response_body_write_failed:{err}"))?;
    stream
        .flush()
        .map_err(|err| format!("response_flush_failed:{err}"))
}

fn html_shell(refresh_ms: u64) -> String {
    const TEMPLATE: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Infring Dashboard (Legacy Core Shell)</title>
  <style>
    :root {
      --bg: #070b14;
      --panel: rgba(12, 20, 38, 0.86);
      --panel-soft: rgba(18, 29, 52, 0.72);
      --text: #e7f0ff;
      --muted: #a7bfde;
      --accent: #36f0c7;
      --accent-2: #5aa3ff;
      --line: rgba(112, 156, 255, 0.28);
    }
    :root[data-theme='light'] {
      --bg: #eef4ff;
      --panel: rgba(255, 255, 255, 0.94);
      --panel-soft: rgba(245, 250, 255, 0.96);
      --text: #1a2b46;
      --muted: #556f90;
      --accent: #008e71;
      --accent-2: #2458d3;
      --line: rgba(71, 106, 184, 0.34);
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial;
      color: var(--text);
      background:
        radial-gradient(circle at 10% 10%, rgba(54, 240, 199, 0.08), transparent 30%),
        radial-gradient(circle at 90% 0%, rgba(90, 163, 255, 0.09), transparent 40%),
        linear-gradient(180deg, #050912, var(--bg));
      min-height: 100vh;
    }
    .wrap {
      max-width: 1060px;
      margin: 20px auto;
      padding: 14px;
    }
    .top {
      display: flex;
      gap: 12px;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 10px;
    }
    .top-left {
      display: flex;
      align-items: center;
      gap: 10px;
    }
    .title {
      font-size: 18px;
      font-weight: 700;
      margin: 0;
    }
    .muted { color: var(--muted); }
    .btn {
      border: 1px solid var(--line);
      background: rgba(90, 163, 255, 0.12);
      color: var(--text);
      border-radius: 9px;
      padding: 8px 10px;
      font-size: 12px;
      cursor: pointer;
    }
    .btn.primary {
      border-color: rgba(54, 240, 199, 0.54);
      background: rgba(54, 240, 199, 0.14);
    }
    .pill {
      border: 1px solid var(--line);
      border-radius: 999px;
      padding: 4px 10px;
      font-size: 11px;
      color: var(--muted);
      background: var(--panel-soft);
    }
    .chat {
      border: 1px solid var(--line);
      border-radius: 14px;
      background: var(--panel);
      padding: 12px;
      box-shadow: 0 14px 40px rgba(0, 0, 0, 0.28);
    }
    .chat-head {
      display: flex;
      justify-content: space-between;
      gap: 12px;
      align-items: center;
      margin-bottom: 10px;
      font-size: 12px;
    }
    .log {
      height: 340px;
      overflow: auto;
      border: 1px solid var(--line);
      border-radius: 10px;
      padding: 10px;
      background: var(--panel-soft);
    }
    .turn {
      border: 1px solid var(--line);
      border-radius: 10px;
      padding: 8px;
      margin-bottom: 8px;
      background: rgba(12, 20, 38, 0.55);
    }
    .chips {
      display: flex;
      gap: 8px;
      margin: 10px 0 8px;
      flex-wrap: wrap;
    }
    .chip {
      border-radius: 999px;
      border: 1px solid var(--line);
      padding: 6px 10px;
      font-size: 11px;
      background: rgba(90, 163, 255, 0.12);
      color: var(--text);
      cursor: pointer;
    }
    .composer {
      display: flex;
      gap: 8px;
    }
    .composer input {
      flex: 1;
      border: 1px solid var(--line);
      border-radius: 10px;
      background: var(--panel-soft);
      color: var(--text);
      padding: 10px;
      font-size: 13px;
    }
    .hint {
      margin-top: 8px;
      font-size: 11px;
      color: var(--muted);
    }
    .drawer {
      position: fixed;
      top: 0;
      right: 0;
      width: min(420px, 96vw);
      height: 100vh;
      background: var(--panel);
      border-left: 1px solid var(--line);
      box-shadow: -10px 0 28px rgba(0, 0, 0, 0.32);
      padding: 14px;
      overflow: auto;
      transform: translateX(102%);
      transition: transform .18s ease-out;
      z-index: 50;
    }
    .drawer.open { transform: translateX(0); }
    details {
      border: 1px solid var(--line);
      border-radius: 10px;
      padding: 8px;
      margin-bottom: 8px;
      background: var(--panel-soft);
    }
    .tabs {
      display: flex;
      gap: 6px;
      flex-wrap: wrap;
      margin-bottom: 10px;
    }
    .tab {
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 6px 8px;
      font-size: 11px;
      color: var(--muted);
      background: var(--panel-soft);
      cursor: pointer;
    }
    .tab.active {
      color: var(--text);
      border-color: rgba(54, 240, 199, 0.54);
      background: rgba(54, 240, 199, 0.15);
    }
    .panel { display: none; }
    .panel.active { display: block; }
    summary {
      font-size: 12px;
      font-weight: 700;
      cursor: pointer;
    }
    ul { margin: 8px 0 0 16px; padding: 0; }
    li { margin-bottom: 4px; font-size: 12px; color: var(--muted); }
  </style>
</head>
<body>
  <div class="wrap">
    <div class="top">
      <div class="top-left">
        <button id="themeToggle" class="btn" type="button">Light</button>
        <div>
          <h1 class="title">Infring Dashboard (Legacy Core Shell)</h1>
          <div class="muted" style="font-size:12px">Chat-first UI with advanced controls in side pane</div>
        </div>
      </div>
      <div style="display:flex;align-items:center;gap:8px">
        <span class="pill">Mode: Chat</span>
        <button id="controlsToggle" class="btn primary" type="button">Open Controls</button>
      </div>
    </div>

    <section class="chat">
      <div class="chat-head">
        <div id="sessionHint" class="muted">Session: chat-ui-default</div>
        <div id="receiptHint" class="muted" style="font-family: ui-monospace, Menlo, monospace;">Receipt: n/a</div>
      </div>
      <div id="chatLog" class="log"></div>
      <div class="chips">
        <button class="chip" data-action="new-agent" type="button">New Agent</button>
        <button class="chip" data-action="new-swarm" type="button">New Swarm</button>
        <button class="chip" data-action="assimilate" type="button">Assimilate Codex</button>
        <button class="chip" data-action="benchmark" type="button">Run Benchmark</button>
        <button class="chip" data-action="open-controls" type="button">Open Controls</button>
        <button class="chip" data-action="manage-swarm" type="button">Swarm Tab</button>
      </div>
      <div class="composer">
        <input id="chatInput" placeholder="Ask anything or type 'new agent' to begin..." />
        <button id="sendBtn" class="btn primary" type="button">Send</button>
      </div>
      <div id="typingHint" class="hint">Tip: Press Enter to send. Esc closes controls.</div>
    </section>
  </div>

  <aside id="drawer" class="drawer">
    <div style="display:flex;justify-content:space-between;align-items:center;gap:8px;margin-bottom:10px">
      <strong>Advanced Controls</strong>
      <button id="drawerClose" class="btn" type="button">Close</button>
    </div>
    <div class="tabs">
      <button type="button" class="tab active" data-controls-tab="swarm">Swarm</button>
      <button type="button" class="tab" data-controls-tab="audit">Audit</button>
      <button type="button" class="tab" data-controls-tab="ops">Ops</button>
      <button type="button" class="tab" data-controls-tab="settings">Settings</button>
    </div>
    <section class="panel active" data-controls-panel="swarm">
      <details open>
        <summary>Swarm / Agent Management</summary>
        <div style="display:flex;gap:8px;margin:8px 0 10px">
          <button class="btn" id="launchAnalyst" type="button">Launch Analyst</button>
          <button class="btn" id="launchOrchestrator" type="button">Launch Orchestrator</button>
        </div>
        <ul id="agentsList"></ul>
      </details>
    </section>
    <section class="panel" data-controls-panel="audit">
      <details open>
        <summary>Receipts & Audit</summary>
        <ul id="receiptsList"></ul>
      </details>
      <details>
        <summary>Logs</summary>
        <ul id="logsList"></ul>
      </details>
    </section>
    <section class="panel" data-controls-panel="ops">
      <details open>
        <summary>APM & Alerts</summary>
        <ul id="metricsList"></ul>
      </details>
    </section>
    <section class="panel" data-controls-panel="settings">
      <details open>
        <summary>Workspace</summary>
        <ul>
          <li>Theme toggle is in top-left.</li>
          <li>Chat mode is default on first load.</li>
          <li>Use controls tab to access advanced surfaces.</li>
        </ul>
      </details>
    </section>
  </aside>

  <script>
    const REFRESH_MS = __REFRESH_MS__;
    const keyTheme = 'infring_dashboard_theme_v2';
    const keyOpen = 'infring_dashboard_controls_open_v2';
    const keyTab = 'infring_dashboard_controls_tab_v1';

    const ui = {
      drawer: document.getElementById('drawer'),
      controlsToggle: document.getElementById('controlsToggle'),
      drawerClose: document.getElementById('drawerClose'),
      themeToggle: document.getElementById('themeToggle'),
      chatLog: document.getElementById('chatLog'),
      chatInput: document.getElementById('chatInput'),
      sendBtn: document.getElementById('sendBtn'),
      typingHint: document.getElementById('typingHint'),
      sessionHint: document.getElementById('sessionHint'),
      receiptHint: document.getElementById('receiptHint'),
      launchAnalyst: document.getElementById('launchAnalyst'),
      launchOrchestrator: document.getElementById('launchOrchestrator'),
      agentsList: document.getElementById('agentsList'),
      receiptsList: document.getElementById('receiptsList'),
      logsList: document.getElementById('logsList'),
      metricsList: document.getElementById('metricsList'),
      tabs: Array.from(document.querySelectorAll('[data-controls-tab]')),
      panels: Array.from(document.querySelectorAll('[data-controls-panel]'))
    };

    function esc(v) {
      return String(v == null ? '' : v)
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;')
        .replaceAll("'", '&#39;');
    }

    function short(v, n = 96) {
      const t = String(v == null ? '' : v).trim();
      if (!t) return 'n/a';
      return t.length <= n ? t : `${t.slice(0, n)}...`;
    }

    function getTheme() {
      try {
        return localStorage.getItem(keyTheme) === 'light' ? 'light' : 'dark';
      } catch { return 'dark'; }
    }

    function setTheme(theme) {
      document.documentElement.setAttribute('data-theme', theme);
      ui.themeToggle.textContent = theme === 'dark' ? 'Light' : 'Dark';
      try { localStorage.setItem(keyTheme, theme); } catch {}
    }

    function getDrawerOpen() {
      try { return localStorage.getItem(keyOpen) === '1'; } catch { return false; }
    }

    function getControlsTab() {
      try {
        const v = localStorage.getItem(keyTab);
        return v || 'swarm';
      } catch { return 'swarm'; }
    }

    async function postAction(action, payload) {
      try {
        const res = await fetch('/api/dashboard/action', {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ action, payload })
        });
        return await res.json();
      } catch {
        return null;
      }
    }

    async function fetchSnapshot() {
      try {
        const res = await fetch('/api/dashboard/snapshot', { cache: 'no-store' });
        if (!res.ok) return null;
        return await res.json();
      } catch {
        return null;
      }
    }

    function list(el, rows, map) {
      const items = Array.isArray(rows) ? rows : [];
      el.innerHTML = items.length
        ? items.map((row) => `<li>${map(row)}</li>`).join('')
        : '<li>No data yet.</li>';
    }

    function renderTurns(turns) {
      const rows = Array.isArray(turns) ? turns.slice(-20) : [];
      if (!rows.length) {
        ui.chatLog.innerHTML = '<div class="muted" style="font-size:12px">No turns yet.</div>';
        return;
      }
      ui.chatLog.innerHTML = rows.map((turn) => `
        <article class="turn">
          <div class="muted" style="font-size:11px">${esc(short(turn.ts || 'n/a', 28))} · ${esc(turn.status || 'complete')}</div>
          <div style="font-size:12px;color:#8fd0ff;margin-top:4px"><strong>You:</strong> ${esc(turn.user || '')}</div>
          <div style="font-size:12px;color:#9ff2cf;margin-top:4px"><strong>Agent:</strong> ${esc(turn.assistant || '')}</div>
        </article>
      `).join('');
      ui.chatLog.scrollTop = ui.chatLog.scrollHeight;
    }

    function render(snapshot) {
      if (!snapshot || typeof snapshot !== 'object') return;
      const turns = snapshot?.app?.turns || [];
      renderTurns(turns);
      ui.sessionHint.textContent = `Session: ${short(snapshot?.app?.session_id || 'chat-ui-default', 64)}`;
      ui.receiptHint.textContent = `Receipt: ${short(snapshot?.receipt_hash || 'n/a', 32)}`;
      list(ui.agentsList, snapshot?.collab?.dashboard?.agents || [], (row) =>
        `${esc(row.shadow || 'shadow')} · ${esc(row.role || 'role')} · ${esc(row.status || 'unknown')}`
      );
      list(ui.receiptsList, snapshot?.receipts?.recent || [], (row) => esc(short(row.path || 'artifact', 94)));
      list(ui.logsList, snapshot?.logs?.recent || [], (row) =>
        `${esc(short(row.ts || 'n/a', 24))} — ${esc(short(row.message || '', 96))}`
      );
      list(ui.metricsList, snapshot?.apm?.metrics || [], (row) =>
        `<strong>${esc(row.name || 'metric')}</strong>: ${esc(row.status || 'unknown')} (${esc(short(row.value, 22))})`
      );
    }

    function setDrawer(open) {
      ui.drawer.classList.toggle('open', open);
      ui.controlsToggle.textContent = open ? 'Close Controls' : 'Open Controls';
      try { localStorage.setItem(keyOpen, open ? '1' : '0'); } catch {}
      postAction('dashboard.ui.toggleControls', { open });
    }

    function setControlsTab(tab) {
      const selected = tab || 'swarm';
      ui.tabs.forEach((el) => {
        el.classList.toggle('active', el.getAttribute('data-controls-tab') === selected);
      });
      ui.panels.forEach((el) => {
        el.classList.toggle('active', el.getAttribute('data-controls-panel') === selected);
      });
      try { localStorage.setItem(keyTab, selected); } catch {}
      postAction('dashboard.ui.switchControlsTab', { tab: selected });
    }

    ui.controlsToggle.addEventListener('click', () => setDrawer(!ui.drawer.classList.contains('open')));
    ui.drawerClose.addEventListener('click', () => setDrawer(false));
    ui.themeToggle.addEventListener('click', () => {
      const next = getTheme() === 'dark' ? 'light' : 'dark';
      setTheme(next);
    });
    ui.tabs.forEach((el) => {
      el.addEventListener('click', () => setControlsTab(el.getAttribute('data-controls-tab')));
    });
    ui.launchAnalyst.addEventListener('click', async () => {
      await postAction('collab.launchRole', { team: 'ops', role: 'analyst', shadow: 'ops-analyst' });
      const snap = await fetchSnapshot();
      render(snap);
    });
    ui.launchOrchestrator.addEventListener('click', async () => {
      await postAction('collab.launchRole', { team: 'ops', role: 'orchestrator', shadow: 'ops-orchestrator' });
      const snap = await fetchSnapshot();
      render(snap);
    });

    document.querySelectorAll('[data-action]').forEach((el) => {
      el.addEventListener('click', async () => {
        const name = el.getAttribute('data-action');
        if (name === 'new-agent') await postAction('collab.launchRole', { team: 'ops', role: 'analyst', shadow: 'ops-analyst' });
        if (name === 'new-swarm') await postAction('collab.launchRole', { team: 'ops', role: 'orchestrator', shadow: 'ops-orchestrator' });
        if (name === 'assimilate') await postAction('dashboard.assimilate', { target: 'codex' });
        if (name === 'benchmark') await postAction('dashboard.benchmark', {});
        if (name === 'open-controls') setDrawer(true);
        if (name === 'manage-swarm') {
          setDrawer(true);
          setControlsTab('swarm');
        }
        const snap = await fetchSnapshot();
        render(snap);
      });
    });

    async function sendChat() {
      const text = String(ui.chatInput.value || '').trim();
      if (!text) return;
      ui.sendBtn.disabled = true;
       ui.typingHint.textContent = 'Sending...';
      await postAction('app.chat', { input: text });
      ui.chatInput.value = '';
      const snap = await fetchSnapshot();
      render(snap);
      ui.sendBtn.disabled = false;
      ui.typingHint.textContent = "Tip: Press Enter to send. Esc closes controls.";
    }

    ui.sendBtn.addEventListener('click', sendChat);
    ui.chatInput.addEventListener('keydown', (ev) => {
      if (ev.key === 'Enter' && !ev.shiftKey) {
        ev.preventDefault();
        sendChat();
      }
    });
    document.addEventListener('keydown', (ev) => {
      if (ev.key === 'Escape') setDrawer(false);
      if ((ev.metaKey || ev.ctrlKey) && ev.key.toLowerCase() === 'k') {
        ev.preventDefault();
        ui.chatInput.focus();
      }
    });

    (async function boot() {
      setTheme(getTheme());
      setDrawer(getDrawerOpen());
      setControlsTab(getControlsTab());
      const first = await fetchSnapshot();
      render(first);
      setInterval(async () => {
        const snap = await fetchSnapshot();
        render(snap);
      }, REFRESH_MS);
    })();
  </script>
</body>
</html>
"#;
    TEMPLATE.replace("__REFRESH_MS__", &refresh_ms.to_string())
}

fn handle_request(
    root: &Path,
    flags: &Flags,
    latest_snapshot: &Arc<Mutex<Value>>,
    stream: &TcpStream,
) -> Result<(), String> {
    let req = parse_request(stream)?;
    if req.method == "GET" && (req.path == "/" || req.path == "/dashboard") {
        let html = html_shell(flags.refresh_ms);
        return write_response(stream, 200, "text/html; charset=utf-8", html.as_bytes());
    }

    if req.method == "GET" && req.path == "/api/dashboard/snapshot" {
        let snapshot = build_snapshot(root, flags);
        write_snapshot_receipt(root, &snapshot);
        if let Ok(mut guard) = latest_snapshot.lock() {
            *guard = snapshot.clone();
        }
        let body = serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string());
        return write_response(
            stream,
            200,
            "application/json; charset=utf-8",
            body.as_bytes(),
        );
    }

    if req.method == "POST" && req.path == "/api/dashboard/action" {
        let payload =
            parse_json_loose(&String::from_utf8_lossy(&req.body)).unwrap_or_else(|| json!({}));
        let action = payload
            .get("action")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 80))
            .unwrap_or_default();
        let action_payload = payload.get("payload").cloned().unwrap_or_else(|| json!({}));
        let lane = run_action(root, &action, &action_payload);
        let action_receipt = write_action_receipt(root, &action, &action_payload, &lane);
        let snapshot = build_snapshot(root, flags);
        write_snapshot_receipt(root, &snapshot);
        if let Ok(mut guard) = latest_snapshot.lock() {
            *guard = snapshot.clone();
        }
        let out = json!({
            "ok": lane.ok,
            "type": "infring_dashboard_action_response",
            "action": action,
            "action_receipt": action_receipt,
            "lane": lane.payload.unwrap_or(Value::Null),
            "snapshot": snapshot
        });
        let body = serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string());
        let status = if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            200
        } else {
            400
        };
        return write_response(
            stream,
            status,
            "application/json; charset=utf-8",
            body.as_bytes(),
        );
    }

    if req.method == "GET" && req.path == "/healthz" {
        let hash = latest_snapshot
            .lock()
            .ok()
            .and_then(|s| s.get("receipt_hash").cloned())
            .unwrap_or(Value::Null);
        let out = json!({
            "ok": true,
            "type": "infring_dashboard_healthz",
            "ts": now_iso(),
            "receipt_hash": hash
        });
        let body = serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string());
        return write_response(
            stream,
            200,
            "application/json; charset=utf-8",
            body.as_bytes(),
        );
    }

    if req.path.starts_with("/api/") {
        let snapshot = latest_snapshot
            .lock()
            .ok()
            .map(|v| v.clone())
            .unwrap_or_else(|| build_snapshot(root, flags));
        let header_refs = req
            .headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>();
        if let Some(response) =
            dashboard_compat_api::handle_with_headers(
                root,
                &req.method,
                &req.path,
                &req.body,
                &header_refs,
                &snapshot,
            )
        {
            let body =
                serde_json::to_string_pretty(&response.payload).unwrap_or_else(|_| "{}".to_string());
            return write_response(
                stream,
                response.status,
                "application/json; charset=utf-8",
                body.as_bytes(),
            );
        }
    }

    let out = json!({
        "ok": false,
        "type": "infring_dashboard_not_found",
        "path": req.path
    });
    let body = serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string());
    write_response(
        stream,
        404,
        "application/json; charset=utf-8",
        body.as_bytes(),
    )
}

fn run_serve(root: &Path, flags: &Flags) -> i32 {
    ensure_dir(&root.join(STATE_DIR_REL));
    ensure_dir(&root.join(ACTION_DIR_REL));

    let initial = build_snapshot(root, flags);
    write_snapshot_receipt(root, &initial);
    let latest_snapshot = Arc::new(Mutex::new(initial.clone()));
    let addr = format!("{}:{}", flags.host, flags.port);
    let listener = match TcpListener::bind(&addr) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "infring_dashboard_server_error",
                    "error": clean_text(&format!("bind_failed:{err}"), 220),
                    "host": flags.host,
                    "port": flags.port
                })
            );
            return 1;
        }
    };

    let url = format!("http://{}:{}/dashboard", flags.host, flags.port);
    let status = json!({
        "ok": true,
        "type": "infring_dashboard_server",
        "ts": now_iso(),
        "url": url,
        "host": flags.host,
        "port": flags.port,
        "refresh_ms": flags.refresh_ms,
        "team": flags.team,
        "authority": "rust_core",
        "receipt_hash": initial.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "snapshot_path": SNAPSHOT_LATEST_REL,
        "action_path": ACTION_LATEST_REL
    });
    write_json(
        &root.join(STATE_DIR_REL).join("server_status.json"),
        &status,
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&status).unwrap_or_else(|_| "{}".to_string())
    );
    println!("Dashboard listening at {url}");

    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            continue;
        };
        if let Err(err) = handle_request(root, flags, &latest_snapshot, &stream) {
            let out = json!({
                "ok": false,
                "type": "infring_dashboard_request_error",
                "ts": now_iso(),
                "error": clean_text(&err, 240)
            });
            let body =
                serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{\"ok\":false}".to_string());
            let _ = write_response(
                &stream,
                500,
                "application/json; charset=utf-8",
                body.as_bytes(),
            );
        }
    }
    0
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let flags = parse_flags(argv);
    match flags.mode.as_str() {
        "git-authority" | "git-authority-v1" => run_git_authority(root, &flags, argv),
        "runtime-sync" | "runtime" => {
            let sync = build_runtime_sync(root, &flags);
            write_json_stdout(&sync, flags.pretty);
            0
        }
        "snapshot" | "status" => {
            let snapshot = build_snapshot(root, &flags);
            write_snapshot_receipt(root, &snapshot);
            write_json_stdout(&snapshot, flags.pretty);
            0
        }
        "serve" | "web" => run_serve(root, &flags),
        _ => {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "infring_dashboard_cli_error",
                    "error": format!("unsupported_mode:{} (expected serve|snapshot|status|runtime-sync|git-authority)", flags.mode)
                })
            );
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parse_flags_defaults() {
        let flags = parse_flags(&[]);
        assert_eq!(flags.mode, "serve");
        assert_eq!(flags.host, "127.0.0.1");
        assert_eq!(flags.port, 4173);
        assert_eq!(flags.team, "ops");
    }

    #[test]
    fn parse_flags_overrides() {
        let flags = parse_flags(&[
            "snapshot".to_string(),
            "--host=0.0.0.0".to_string(),
            "--port=8080".to_string(),
            "--team=alpha".to_string(),
            "--refresh-ms=5000".to_string(),
            "--pretty=0".to_string(),
        ]);
        assert_eq!(flags.mode, "snapshot");
        assert_eq!(flags.host, "0.0.0.0");
        assert_eq!(flags.port, 8080);
        assert_eq!(flags.team, "alpha");
        assert_eq!(flags.refresh_ms, 5000);
        assert!(!flags.pretty);
    }

    #[test]
    fn parse_json_loose_supports_multiline_logs() {
        let raw = "noise\n{\"ok\":false}\n{\"ok\":true,\"type\":\"x\"}\n";
        let parsed = parse_json_loose(raw).expect("json");
        assert_eq!(parsed.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn recommended_conduit_signals_scales_with_pressure() {
        assert_eq!(recommended_conduit_signals(5, 0.10, 1, 0), 4);
        assert!(recommended_conduit_signals(80, 0.70, 4, 120) >= 12);
        assert_eq!(recommended_conduit_signals(120, 0.95, 2, 0), 16);
    }

    #[test]
    fn merge_profile_agents_adds_profile_rows_and_excludes_archived() {
        let root = tempfile::tempdir().expect("tempdir");
        let profiles_path = root.path().join(AGENT_PROFILES_REL);
        let archived_path = root.path().join(ARCHIVED_AGENTS_REL);
        if let Some(parent) = profiles_path.parent() {
            fs::create_dir_all(parent).expect("mkdir profiles");
        }
        fs::write(
            &profiles_path,
            serde_json::to_string_pretty(&json!({
                "type": "infring_dashboard_agent_profiles",
                "agents": {
                    "runtime-a": { "role": "analyst", "updated_at": "2026-03-28T00:00:00Z" },
                    "profile-b": { "role": "orchestrator", "updated_at": "2026-03-28T01:00:00Z" },
                    "archived-c": { "role": "analyst", "updated_at": "2026-03-28T02:00:00Z" }
                }
            }))
            .expect("json profiles"),
        )
        .expect("write profiles");
        fs::write(
            &archived_path,
            serde_json::to_string_pretty(&json!({
                "type": "infring_dashboard_archived_agents",
                "agents": {
                    "archived-c": { "reason": "timeout" }
                }
            }))
            .expect("json archived"),
        )
        .expect("write archived");

        let mut collab = json!({
            "ok": true,
            "type": "collab_plane_dashboard",
            "dashboard": {
                "team": "ops",
                "agents": [
                    { "shadow": "runtime-a", "role": "analyst", "status": "running" }
                ],
                "tasks": [],
                "handoff_history": []
            }
        });
        dashboard_agent_state::merge_profiles_into_collab(root.path(), &mut collab, "ops");
        let rows = collab
            .get("dashboard")
            .and_then(|v| v.get("agents"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let ids = rows
            .iter()
            .filter_map(|row| row.get("shadow").and_then(Value::as_str))
            .map(ToString::to_string)
            .collect::<HashSet<_>>();
        assert!(ids.contains("runtime-a"));
        assert!(ids.contains("profile-b"));
        assert!(!ids.contains("archived-c"));
    }

    #[test]
    fn runtime_apply_telemetry_remediations_action_is_rust_handled() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.runtime.applyTelemetryRemediations",
            &json!({ "team": "ops" }),
        );
        assert!(lane.ok);
        assert_eq!(lane.status, 0);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("infring_dashboard_runtime_action")
        );
        assert_eq!(
            payload.get("action").and_then(Value::as_str),
            Some("apply_telemetry_remediations")
        );
    }

    #[test]
    fn dashboard_agent_actions_round_trip_through_rust_authority() {
        let root = tempfile::tempdir().expect("tempdir");
        let model_catalog = run_action(root.path(), "dashboard.models.catalog", &json!({}));
        assert!(model_catalog.ok);
        let route_decision = run_action(
            root.path(),
            "dashboard.model.routeDecision",
            &json!({"task_type":"general","offline_required":false}),
        );
        assert!(route_decision.ok);
        let terminal_create = run_action(
            root.path(),
            "dashboard.terminal.session.create",
            &json!({"id":"term-test"}),
        );
        assert!(terminal_create.ok);
        let terminal_exec = run_action(
            root.path(),
            "dashboard.terminal.exec",
            &json!({"session_id":"term-test","command":"printf 'ok'"}),
        );
        assert!(terminal_exec.ok);
        assert_eq!(
            terminal_exec
                .payload
                .clone()
                .unwrap_or_else(|| json!({}))
                .get("stdout")
                .and_then(Value::as_str),
            Some("ok")
        );
        let terminal_close = run_action(
            root.path(),
            "dashboard.terminal.session.close",
            &json!({"session_id":"term-test"}),
        );
        assert!(terminal_close.ok);
        let upsert_profile = run_action(
            root.path(),
            "dashboard.agent.upsertProfile",
            &json!({
                "agent_id": "agent-a",
                "role": "analyst",
                "name": "Agent A"
            }),
        );
        assert!(upsert_profile.ok);

        let append_turn = run_action(
            root.path(),
            "dashboard.agent.session.appendTurn",
            &json!({
                "agent_id": "agent-a",
                "user": "Can you reduce queue depth before spikes?",
                "assistant": "Yes, running mitigation now."
            }),
        );
        assert!(append_turn.ok);

        let create_session = run_action(
            root.path(),
            "dashboard.agent.session.create",
            &json!({
                "agent_id": "agent-a",
                "label": "Deep Work"
            }),
        );
        assert!(create_session.ok);
        let active_session = create_session
            .payload
            .clone()
            .unwrap_or_else(|| json!({}))
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(!active_session.is_empty());

        let switch_session = run_action(
            root.path(),
            "dashboard.agent.session.switch",
            &json!({
                "agent_id": "agent-a",
                "session_id": active_session
            }),
        );
        assert!(switch_session.ok);

        let set_memory = run_action(
            root.path(),
            "dashboard.agent.memoryKv.set",
            &json!({
                "agent_id": "agent-a",
                "key": "focus.topic",
                "value": "reliability"
            }),
        );
        assert!(set_memory.ok);

        let get_memory = run_action(
            root.path(),
            "dashboard.agent.memoryKv.get",
            &json!({
                "agent_id": "agent-a",
                "key": "focus.topic"
            }),
        );
        assert!(get_memory.ok);
        assert_eq!(
            get_memory
                .payload
                .clone()
                .unwrap_or_else(|| json!({}))
                .get("value")
                .and_then(Value::as_str),
            Some("reliability")
        );

        let delete_memory = run_action(
            root.path(),
            "dashboard.agent.memoryKv.delete",
            &json!({
                "agent_id": "agent-a",
                "key": "focus.topic"
            }),
        );
        assert!(delete_memory.ok);

        let suggestions = run_action(
            root.path(),
            "dashboard.agent.suggestions",
            &json!({
                "agent_id": "agent-a",
                "hint": "\"Can you reduce queue depth before spikes?\""
            }),
        );
        assert!(suggestions.ok);
        let suggestion_rows = suggestions
            .payload
            .clone()
            .unwrap_or_else(|| json!({}))
            .get("suggestions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(suggestion_rows.len() <= 3);
        for row in suggestion_rows {
            let text = row.as_str().unwrap_or("");
            assert!(!text.contains('"'));
            assert!(!text.contains('\''));
        }

        let upsert_contract = run_action(
            root.path(),
            "dashboard.agent.upsertContract",
            &json!({
                "agent_id": "agent-a",
                "created_at": "2000-01-01T00:00:00Z",
                "expiry_seconds": 1,
                "status": "active"
            }),
        );
        assert!(upsert_contract.ok);
        let enforce_contracts = run_action(
            root.path(),
            "dashboard.agent.enforceContracts",
            &json!({}),
        );
        assert!(enforce_contracts.ok);
        let terminated_rows = enforce_contracts
            .payload
            .clone()
            .unwrap_or_else(|| json!({}))
            .get("terminated")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!terminated_rows.is_empty());
    }
}
