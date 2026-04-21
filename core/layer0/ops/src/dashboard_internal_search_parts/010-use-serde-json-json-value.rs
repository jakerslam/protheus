// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const AGENT_PROFILES_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_profiles.json";
const AGENT_CONTRACTS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_contracts.json";
const AGENT_SESSIONS_DIR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_sessions";
const MAX_INDEXED_LINES_PER_AGENT: usize = 320;
const MAX_LINE_CHARS: usize = 520;

fn clean_text(value: &str, max_len: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn normalize_agent_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 180).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        }
    }
    out
}

fn parse_json_loose(body: &str) -> Option<Value> {
    if body.trim().is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        return Some(value);
    }
    for line in body.lines().rev() {
        let row = line.trim();
        if row.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(row) {
            return Some(value);
        }
    }
    None
}

fn read_json_file(path: &Path) -> Option<Value> {
    let body = fs::read_to_string(path).ok()?;
    parse_json_loose(&body)
}

fn profiles_path(root: &Path) -> PathBuf {
    root.join(AGENT_PROFILES_REL)
}

fn contracts_path(root: &Path) -> PathBuf {
    root.join(AGENT_CONTRACTS_REL)
}

fn sessions_dir(root: &Path) -> PathBuf {
    root.join(AGENT_SESSIONS_DIR_REL)
}

fn is_stop_word(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "and"
            | "are"
            | "as"
            | "at"
            | "by"
            | "for"
            | "from"
            | "if"
            | "in"
            | "into"
            | "is"
            | "it"
            | "of"
            | "on"
            | "or"
            | "that"
            | "the"
            | "then"
            | "to"
            | "up"
            | "was"
            | "were"
            | "with"
    )
}

fn tokenize_for_search(value: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut current = String::new();
    let push_current = |row: &mut String, target: &mut Vec<String>| {
        if row.is_empty() {
            return;
        }
        let token = row.to_ascii_lowercase();
        row.clear();
        if token.len() < 2 || is_stop_word(&token) {
            return;
        }
        target.push(token);
    };
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            current.push(ch);
            continue;
        }
        push_current(&mut current, &mut out);
    }
    push_current(&mut current, &mut out);
    out
}

fn humanize_agent_name(agent_id: &str) -> String {
    let normalized = normalize_agent_id(agent_id);
    if normalized.is_empty() {
        return "Agent".to_string();
    }
    let mut out = String::new();
    let mut capitalize = true;
    for ch in normalized.chars() {
        if ch == '-' || ch == '_' {
            out.push(' ');
            capitalize = true;
            continue;
        }
        if capitalize {
            out.extend(ch.to_uppercase());
            capitalize = false;
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    clean_text(&out, 140)
}

fn text_from_message(row: &Value) -> String {
    let text = row
        .get("text")
        .and_then(Value::as_str)
        .or_else(|| row.get("content").and_then(Value::as_str))
        .unwrap_or("");
    clean_text(text, MAX_LINE_CHARS)
}

fn profile_map(root: &Path) -> HashMap<String, Value> {
    let mut out = HashMap::<String, Value>::new();
    let state = read_json_file(&profiles_path(root)).unwrap_or_else(|| json!({}));
    if let Some(agents) = state.get("agents").and_then(Value::as_object) {
        for (raw_id, profile) in agents {
            let id = normalize_agent_id(raw_id);
            if id.is_empty() {
                continue;
            }
            out.insert(id, profile.clone());
        }
    }
    out
}

fn contract_map(root: &Path) -> HashMap<String, Value> {
    let mut out = HashMap::<String, Value>::new();
    let state = read_json_file(&contracts_path(root)).unwrap_or_else(|| json!({}));
    if let Some(contracts) = state.get("contracts").and_then(Value::as_object) {
        for (raw_id, contract) in contracts {
            let id = normalize_agent_id(raw_id);
            if id.is_empty() {
                continue;
            }
            out.insert(id, contract.clone());
        }
    }
    out
}

#[derive(Clone, Debug)]
struct ConversationDocument {
    agent_id: String,
    name: String,
    archived: bool,
    state: String,
    avatar_url: String,
    emoji: String,
    updated_at: String,
    lines: Vec<String>,
}
