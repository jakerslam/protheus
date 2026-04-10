// Layer ownership: core/layer2/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0

use crate::hot_path_allocators::{
    mark_hot_path_batch, snapshot_json as hot_path_allocator_snapshot_json, with_arena_bytes,
    with_slab_buffer,
};
use crate::{deterministic_receipt_hash, now_epoch_ms};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const USAGE: &[&str] = &[
    "Usage:",
    "  protheus-ops nexus-internal-comms status [--limit=<n>] [--modules=a,b,c] [--task='<text>'] [--role=<id>]",
    "  protheus-ops nexus-internal-comms validate --message='[FROM>TO|MOD] CMD k=v' [--modules=a,b,c] [--task='<text>'] [--role=<id>]",
    "  protheus-ops nexus-internal-comms compress --from=<id> --to=<id> --cmd=<key> [--module=<name>] --text='<natural text>'",
    "  protheus-ops nexus-internal-comms decompress --message='<nexus_line>' [--modules=a,b,c] [--task='<text>'] [--role=<id>]",
    "  protheus-ops nexus-internal-comms send --message='<nexus_line>' [--raw-text='<natural text>'] [--modules=a,b,c] [--task='<text>'] [--role=<id>]",
    "  protheus-ops nexus-internal-comms log [--limit=<n>] [--decompressed=1|0]",
    "  protheus-ops nexus-internal-comms agent-prompt --agent=<id> [--modules=a,b,c] [--task='<text>'] [--role=<id>]",
    "  protheus-ops nexus-internal-comms resolve-modules [--modules=a,b,c] [--module=<name>] [--task='<text>'] [--role=<id>] [--text='<context>']",
    "  protheus-ops nexus-internal-comms export-lexicon [--modules=a,b,c] [--task='<text>'] [--role=<id>] [--with-catalog=1|0]",
];

const MAX_MODULES_PER_AGENT: usize = 3;
const DEFAULT_LIMIT: usize = 20;

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn usage() {
    for line in USAGE {
        println!("{line}");
    }
}

fn state_root(root: &Path) -> PathBuf {
    root.join("local")
        .join("state")
        .join("ops")
        .join("nexus_internal_comms")
}

fn messages_path(root: &Path) -> PathBuf {
    state_root(root).join("nexus_messages.jsonl")
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let pref = format!("--{key}=");
    let exact = format!("--{key}");
    let mut idx = 0usize;
    while idx < argv.len() {
        let token = argv[idx].trim();
        if let Some(value) = token.strip_prefix(&pref) {
            let cleaned = value.trim().to_string();
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
        if token == exact {
            if let Some(next) = argv.get(idx + 1) {
                let cleaned = next.trim().to_string();
                if !cleaned.is_empty() {
                    return Some(cleaned);
                }
            }
        }
        idx += 1;
    }
    None
}

fn parse_bool(raw: Option<String>, fallback: bool) -> bool {
    match raw.unwrap_or_default().trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn parse_limit(raw: Option<String>) -> usize {
    raw.and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, 500)
}

fn normalize_ascii_token(raw: &str, max_len: usize, uppercase: bool) -> String {
    raw.trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                if uppercase {
                    c.to_ascii_uppercase()
                } else {
                    c.to_ascii_lowercase()
                }
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn normalize_id(raw: &str) -> String {
    normalize_ascii_token(raw, 64, true)
}

fn normalize_token(raw: &str) -> String {
    normalize_ascii_token(raw, 96, true)
}

fn normalize_text_atom(raw: &str) -> String {
    normalize_ascii_token(raw, 128, false)
}

fn estimate_tokens(raw: &str) -> usize {
    raw.split_whitespace().count().max(1)
}

fn with_hash(mut payload: Value) -> Value {
    payload["receipt_hash"] = Value::String(deterministic_receipt_hash(&payload));
    payload
}

fn append_jsonl(path: &Path, payload: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_failed:{e}"))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open_append_failed:{e}"))?;
    let encoded = serde_json::to_string(payload).map_err(|e| format!("encode_failed:{e}"))?;
    writeln!(file, "{encoded}").map_err(|e| format!("append_failed:{e}"))?;
    Ok(())
}

fn write_json(path: &Path, payload: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_failed:{e}"))?;
    }
    let encoded =
        serde_json::to_string_pretty(payload).map_err(|e| format!("encode_pretty_failed:{e}"))?;
    fs::write(path, encoded).map_err(|e| format!("write_failed:{e}"))
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn read_recent_jsonl(path: &Path, limit: usize) -> Vec<Value> {
    let raw = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let mut rows = raw
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>();
    if rows.len() > limit {
        rows = rows.split_off(rows.len() - limit);
    }
    rows
}

fn error_payload(kind: &str, command: &str, error: &str) -> Value {
    with_hash(json!({
        "ok": false,
        "type": kind,
        "command": command,
        "error": error
    }))
}
