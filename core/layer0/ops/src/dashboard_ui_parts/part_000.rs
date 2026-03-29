// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::dashboard_agent_state;
use crate::dashboard_compat_api;
use crate::dashboard_model_catalog;
use crate::dashboard_terminal_broker;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

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
const AGENT_PROFILES_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_profiles.json";
#[cfg(test)]
const ARCHIVED_AGENTS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/archived_agents.json";
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
