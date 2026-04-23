#[path = "../db.rs"]
mod db;
#[path = "../rag_runtime.rs"]
mod rag_runtime;
#[path = "../wave1.rs"]
mod wave1;
use db::{DbIndexEntry, HotStateEnvelopeStats, MemoryDb};
use infring_layer1_memory_runtime::recall_policy::{
    enforce_hydration_guard, enforce_index_first, enforce_index_freshness, enforce_node_only,
    enforce_recall_budget, FailClosedMode, HydrationGuardInput, RecallBudgetInput,
    DEFAULT_BOOTSTRAP_HYDRATION_TOKEN_CAP, DEFAULT_BURN_THRESHOLD_TOKENS, DEFAULT_EXPAND_LINES,
    DEFAULT_INDEX_MAX_AGE_MS, DEFAULT_MAX_FILES, DEFAULT_RECALL_TOP, MAX_EXPAND_LINES,
    MAX_MAX_FILES, MAX_RECALL_TOP,
};
use infring_layer1_memory_runtime::token_telemetry::{
    evaluate_burn_slo, RetrievalMode, TokenTelemetryEvent,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, UNIX_EPOCH};

#[derive(Clone, Debug, Default)]
struct IndexEntry {
    node_id: String,
    uid: String,
    file_rel: String,
    summary: String,
    tags: Vec<String>,
}

#[derive(Serialize)]
struct ProbeResult {
    ok: bool,
    parity_error_count: usize,
    estimated_ms: u64,
}

#[derive(Serialize)]
struct QueryHit {
    node_id: String,
    uid: String,
    file: String,
    summary: String,
    tags: Vec<String>,
    score: i64,
    reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trust_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_refs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recall_explanation: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    section_excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    section_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    section_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expand_blocked: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expand_error: Option<String>,
}

#[derive(Serialize)]
struct QueryResult {
    ok: bool,
    backend: String,
    score_mode: String,
    vector_enabled: bool,
    recall_mode: String,
    entries_total: usize,
    candidates_total: usize,
    index_sources: Vec<String>,
    tag_sources: Vec<String>,
    hits: Vec<QueryHit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    policy: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    burn_slo: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    freshness: Option<Value>,
}

#[derive(Serialize)]
struct GetNodeResult {
    ok: bool,
    backend: String,
    node_id: String,
    uid: String,
    file: String,
    summary: String,
    tags: Vec<String>,
    section_hash: String,
    section: String,
}

#[derive(Serialize)]
struct BuildIndexResult {
    ok: bool,
    backend: String,
    node_count: usize,
    tag_count: usize,
    files_scanned: usize,
    wrote_files: bool,
    memory_index_path: String,
    tags_index_path: String,
    memory_index_sha256: String,
    tags_index_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sqlite_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sqlite_rows_written: Option<usize>,
}

#[derive(Serialize)]
struct VerifyEnvelopeResult {
    ok: bool,
    backend: String,
    db_path: String,
    total_rows: usize,
    enveloped_rows: usize,
    legacy_cipher_rows: usize,
    plain_rows: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct CacheNode {
    mtime_ms: u64,
    section_hash: String,
    section_text: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct WorkingSetCache {
    schema_version: String,
    nodes: HashMap<String, CacheNode>,
}

#[derive(Deserialize)]
struct DaemonRequest {
    cmd: String,
    #[serde(default)]
    args: HashMap<String, String>,
}

fn strip_ticks(s: &str) -> String {
    s.chars()
        .filter(|ch| {
            !matches!(
                *ch,
                '\u{200B}'
                    | '\u{200C}'
                    | '\u{200D}'
                    | '\u{2060}'
                    | '\u{FEFF}'
                    | '\u{202A}'
                    | '\u{202B}'
                    | '\u{202C}'
                    | '\u{202D}'
                    | '\u{202E}'
            ) && (!ch.is_control() || ch.is_ascii_whitespace())
        })
        .collect::<String>()
        .replace('`', "")
        .trim()
        .to_string()
}

fn clean_cell(s: &str) -> String {
    strip_ticks(s.trim())
}

fn normalize_node_id(v: &str) -> String {
    let raw = strip_ticks(v);
    if raw.is_empty() {
        return String::new();
    }
    if raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        raw
    } else {
        String::new()
    }
}

fn normalize_uid(v: &str) -> String {
    let raw = strip_ticks(v);
    if raw.is_empty() {
        return String::new();
    }
    if raw.chars().all(|c| c.is_ascii_alphanumeric()) {
        raw
    } else {
        String::new()
    }
}

fn normalize_tag(v: &str) -> String {
    let mut raw = strip_ticks(v).to_lowercase();
    while raw.starts_with('#') {
        raw = raw[1..].to_string();
    }
    raw.chars()
        .filter(|c| {
            c.is_ascii_lowercase()
                || c.is_ascii_digit()
                || matches!(c, '_' | '-' | ':' | '=')
        })
        .collect::<String>()
}

fn normalize_header_cell(v: &str) -> String {
    let s = clean_cell(v).to_lowercase();
    let mut norm = String::new();
    let mut prev_underscore = false;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            norm.push(ch);
            prev_underscore = false;
        } else if !prev_underscore {
            norm.push('_');
            prev_underscore = true;
        }
    }
    let norm = norm.trim_matches('_').to_string();
    if norm.contains("node_id") {
        return "node_id".to_string();
    }
    if norm == "uid" || norm.ends_with("_uid") {
        return "uid".to_string();
    }
    if norm.starts_with("file") {
        return "file".to_string();
    }
    if norm.starts_with("summary") || norm.starts_with("title") {
        return "summary".to_string();
    }
    if norm.starts_with("tags") {
        return "tags".to_string();
    }
    norm
}

fn is_date_memory_file(v: &str) -> bool {
    let bytes = v.as_bytes();
    if bytes.len() != 13 {
        return false;
    }
    for (idx, b) in bytes.iter().enumerate() {
        let is_digit = b.is_ascii_digit();
        match idx {
            4 | 7 => {
                if *b != b'-' {
                    return false;
                }
            }
            10 => {
                if *b != b'.' {
                    return false;
                }
            }
            11 => {
                if *b != b'm' {
                    return false;
                }
            }
            12 => {
                if *b != b'd' {
                    return false;
                }
            }
            _ => {
                if !is_digit {
                    return false;
                }
            }
        }
    }
    true
}
