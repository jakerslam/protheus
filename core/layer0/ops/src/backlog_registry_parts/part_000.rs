// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_iso, parse_args};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct Paths {
    backlog_path: PathBuf,
    registry_path: PathBuf,
    active_view_path: PathBuf,
    archive_view_path: PathBuf,
    priority_view_path: PathBuf,
    reviewed_view_path: PathBuf,
    execution_path_view_path: PathBuf,
    state_path: PathBuf,
    latest_path: PathBuf,
    receipts_path: PathBuf,
}

#[derive(Debug, Clone)]
struct Policy {
    version: String,
    strict_default: bool,
    active_statuses: BTreeSet<String>,
    archive_statuses: BTreeSet<String>,
    paths: Paths,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryRow {
    id: String,
    class: String,
    wave: String,
    status: String,
    title: String,
    problem: String,
    acceptance: String,
    dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
struct ParsedRow {
    row: RegistryRow,
    canonical: bool,
    source_index: usize,
}

#[derive(Debug, Clone)]
struct CompiledBacklog {
    generated_at: String,
    rows: Vec<RegistryRow>,
    conflicts: Vec<Value>,
    active_view: String,
    archive_view: String,
    priority_view: String,
    reviewed_view: String,
    execution_view: String,
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops backlog-registry sync [--policy=<path>]");
    println!("  protheus-ops backlog-registry check [--strict=1|0] [--policy=<path>]");
    println!("  protheus-ops backlog-registry status [--policy=<path>]");
}

fn normalize_token(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars().take(max_len) {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '/' | '-') {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    let mut squashed = String::new();
    let mut prev_us = false;
    for ch in out.chars() {
        let is_us = ch == '_';
        if is_us && prev_us {
            continue;
        }
        squashed.push(ch);
        prev_us = is_us;
    }
    squashed.trim_matches('_').to_string()
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn normalize_id(raw: &str) -> Option<String> {
    let id = clean_text(raw, 120).trim_matches('`').to_ascii_uppercase();
    if id.is_empty() {
        return None;
    }
    let parts: Vec<&str> = id.split('-').collect();
    if parts.len() < 2 {
        return None;
    }
    if parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    if parts.iter().all(|p| {
        p.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    }) {
        Some(id)
    } else {
        None
    }
}

fn parse_bool(raw: Option<&String>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

fn path_from_policy(root: &Path, raw: Option<&str>, fallback: &str) -> PathBuf {
    let v = raw.unwrap_or(fallback).trim();
    if v.is_empty() {
        return root.join(fallback);
    }
    let candidate = PathBuf::from(v);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn canonicalize_or_self(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn enforce_canonical_backlog_path(root: &Path, policy: &Policy) -> Result<(), String> {
    let expected = root.join("docs/workspace/SRS.md");
    let expected_canon = canonicalize_or_self(&expected);
    let actual_canon = canonicalize_or_self(&policy.paths.backlog_path);
    if expected_canon != actual_canon {
        return Err(format!(
            "canonical_backlog_path_required:expected={}:actual={}",
            expected.display(),
            policy.paths.backlog_path.display()
        ));
    }
    Ok(())
}

fn load_policy(root: &Path, policy_override: Option<&String>) -> Policy {
    let default_path = root.join("client/runtime/config/backlog_registry_policy.json");
    let policy_path = policy_override.map(PathBuf::from).unwrap_or(default_path);

    let raw = fs::read_to_string(&policy_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| json!({}));

    let version = raw
        .get("version")
        .and_then(Value::as_str)
        .map(|s| clean_text(s, 32))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "1.0".to_string());

    let strict_default = raw
        .get("strict_default")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let active_statuses = raw
        .get("active_statuses")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|v| normalize_token(v, 40))
                .filter(|v| !v.is_empty())
                .collect::<BTreeSet<_>>()
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            ["queued", "in_progress", "blocked", "proposed"]
                .iter()
                .map(|v| (*v).to_string())
                .collect()
        });

    let archive_statuses = raw
        .get("archive_statuses")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|v| normalize_token(v, 40))
                .filter(|v| !v.is_empty())
                .collect::<BTreeSet<_>>()
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            ["done", "dropped", "archived", "obsolete"]
                .iter()
                .map(|v| (*v).to_string())
                .collect()
        });

    let paths_obj = raw.get("paths").and_then(Value::as_object);

    let paths = Paths {
        backlog_path: path_from_policy(
            root,
            paths_obj
                .and_then(|o| o.get("backlog_path"))
                .and_then(Value::as_str),
            "docs/workspace/SRS.md",
        ),
        registry_path: path_from_policy(
            root,
            paths_obj
                .and_then(|o| o.get("registry_path"))
                .and_then(Value::as_str),
            "client/runtime/config/backlog_registry.json",
        ),
        active_view_path: path_from_policy(
            root,
            paths_obj
                .and_then(|o| o.get("active_view_path"))
                .and_then(Value::as_str),
            "docs/client/backlog_views/active.md",
        ),
        archive_view_path: path_from_policy(
            root,
            paths_obj
                .and_then(|o| o.get("archive_view_path"))
                .and_then(Value::as_str),
            "docs/client/backlog_views/archive.md",
        ),
        priority_view_path: root.join("docs/client/backlog_views/priority_queue.md"),
        reviewed_view_path: root.join("docs/client/backlog_views/reviewed.md"),
        execution_path_view_path: root.join("docs/client/backlog_views/execution_path.md"),
        state_path: path_from_policy(
            root,
            paths_obj
                .and_then(|o| o.get("state_path"))
                .and_then(Value::as_str),
            "local/state/ops/backlog_registry/state.json",
        ),
        latest_path: path_from_policy(
            root,
            paths_obj
                .and_then(|o| o.get("latest_path"))
                .and_then(Value::as_str),
            "local/state/ops/backlog_registry/latest.json",
        ),
        receipts_path: path_from_policy(
            root,
            paths_obj
                .and_then(|o| o.get("receipts_path"))
                .and_then(Value::as_str),
            "local/state/ops/backlog_registry/receipts.jsonl",
        ),
    };

    Policy {
        version,
        strict_default,
        active_statuses,
        archive_statuses,
        paths,
    }
}

fn split_markdown_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return Vec::new();
    }
    let mut row = trimmed.trim_start_matches('|').to_string();
    if row.ends_with('|') {
        row.pop();
    }

    let mut cells = Vec::new();
    let mut current = String::new();
    let mut in_backtick = false;
    let mut escaped = false;

    for ch in row.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            current.push(ch);
            continue;
        }
        if ch == '`' {
            in_backtick = !in_backtick;
            current.push(ch);
            continue;
        }
        if ch == '|' && !in_backtick {
            cells.push(clean_text(&current.replace("\\|", "|"), 8000));
            current.clear();
            continue;
        }
        current.push(ch);
    }
    cells.push(clean_text(&current.replace("\\|", "|"), 8000));
    cells
}

fn is_separator_row(cells: &[String]) -> bool {
    if cells.is_empty() {
        return true;
    }
    let first = cells[0].replace(['-', ':', ' '], "");
    first.is_empty()
}

fn status_weight(status: &str) -> i32 {
    match status {
        "reviewed" => 700,
        "done" => 650,
        "in_progress" => 500,
        "blocked" => 350,
        "queued" => 250,
        "proposed" => 200,
        "archived" | "obsolete" | "dropped" => 180,
        _ => 100,
    }
}

fn parse_dependencies(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let upper = raw.to_ascii_uppercase();
    for token in upper.split(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-'))
    {
        if let Some(id) = normalize_id(token) {
            if !out.contains(&id) {
                out.push(id);
            }
        }
    }
    out
}

fn parse_backlog_rows(markdown: &str) -> Vec<ParsedRow> {
    let mut parsed = Vec::new();
    for (idx, raw_line) in markdown.lines().enumerate() {
        let line = raw_line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells = split_markdown_row(line);
        if cells.len() < 5 || is_separator_row(&cells) {
            continue;
        }

        let Some(id) = normalize_id(&cells[0]) else {
            continue;
        };

        let compact_status = cells
            .get(1)
            .map(|s| normalize_token(s, 40))
            .unwrap_or_default();
        let canonical_status = cells
            .get(3)
            .map(|s| normalize_token(s, 40))
            .unwrap_or_default();

        let (canonical, class, wave, status, title, problem, acceptance, deps_raw) =
            if cells.len() >= 8 && !canonical_status.is_empty() {
                (
                    true,
                    normalize_token(&cells[1], 80),
                    clean_text(&cells[2], 40),
                    canonical_status,
                    clean_text(&cells[4], 500),
                    clean_text(&cells[5], 8000),
                    clean_text(&cells[6], 12000),
                    cells.get(7).cloned().unwrap_or_default(),
                )
            } else if !compact_status.is_empty() {
                (
                    false,
                    "backlog".to_string(),
                    id.split('-').next().unwrap_or("V?").to_string(),
                    compact_status,
                    clean_text(cells.get(2).map(String::as_str).unwrap_or(""), 500),
                    clean_text(cells.get(3).map(String::as_str).unwrap_or(""), 8000),
                    clean_text(cells.get(4).map(String::as_str).unwrap_or(""), 12000),
                    cells.get(5).cloned().unwrap_or_default(),
                )
            } else {
                continue;
            };

        let row = RegistryRow {
            id,
            class: if class.is_empty() {
                "backlog".to_string()
            } else {
                class
            },
            wave: if wave.is_empty() {
                "V?".to_string()
            } else {
                wave
            },
            status: if status.is_empty() {
                "queued".to_string()
            } else {
                status
            },
            title,
            problem,
            acceptance,
            dependencies: parse_dependencies(&deps_raw),
        };

        parsed.push(ParsedRow {
            row,
            canonical,
            source_index: idx,
        });
    }
    parsed
}

