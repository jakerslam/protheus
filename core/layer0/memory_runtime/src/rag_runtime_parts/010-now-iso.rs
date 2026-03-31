// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RagChunk {
    chunk_id: String,
    source_path: String,
    mime: String,
    offset_start: usize,
    offset_end: usize,
    text: String,
    sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RagSource {
    path: String,
    mime: String,
    sha256: String,
    chunk_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RagIndex {
    schema_version: String,
    generated_at: String,
    source_count: usize,
    chunk_count: usize,
    sources: Vec<RagSource>,
    chunks: Vec<RagChunk>,
    tombstones: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TaxonomyRow {
    chunk_id: String,
    source: String,
    when_value: String,
    what_value: String,
    how_value: String,
    which_value: String,
    confidence: f64,
    keywords: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TaxonomySnapshot {
    schema_version: String,
    generated_at: String,
    row_count: usize,
    rows: Vec<TaxonomyRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CausalNode {
    id: String,
    ts: String,
    event_type: String,
    summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CausalEdge {
    from: String,
    to: String,
    relation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CausalityGraph {
    schema_version: String,
    generated_at: String,
    node_count: usize,
    edge_count: usize,
    nodes: Vec<CausalNode>,
    edges: Vec<CausalEdge>,
}

fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn parse_usize(value: Option<&String>, min: usize, max: usize, default: usize) -> usize {
    value
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn parse_bool(value: Option<&String>, default: bool) -> bool {
    value
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn root_from_args(args: &HashMap<String, String>) -> PathBuf {
    let raw = clean_text(args.get("root").map_or(".", String::as_str), 600);
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn state_root(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    let raw = clean_text(args.get("state-root").map_or("", String::as_str), 600);
    if raw.is_empty() {
        root.join("local")
            .join("state")
            .join("ops")
            .join("local_rag")
    } else {
        let p = PathBuf::from(raw);
        if p.is_absolute() {
            p
        } else {
            root.join(p)
        }
    }
}

fn index_path(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    let raw = clean_text(args.get("index-path").map_or("", String::as_str), 600);
    if raw.is_empty() {
        state_root(root, args).join("index.json")
    } else {
        let p = PathBuf::from(raw);
        if p.is_absolute() {
            p
        } else {
            root.join(p)
        }
    }
}

fn history_path(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    state_root(root, args).join("history.jsonl")
}

fn taxonomy_path(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    state_root(root, args).join("taxonomy_4w.json")
}

fn metacognitive_config_path(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    state_root(root, args).join("metacognitive_config.json")
}

fn metacognitive_journal_path(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    state_root(root, args).join("metacognitive_journal.jsonl")
}

fn causality_path(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    state_root(root, args).join("causality_graph.json")
}

fn ama_benchmark_path(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    state_root(root, args).join("ama_benchmark_latest.json")
}

fn sharing_ledger_path(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    state_root(root, args).join("sharing_ledger.jsonl")
}

fn evolution_state_path(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    state_root(root, args).join("evolution_state.json")
}

fn fusion_state_path(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    state_root(root, args).join("fusion_snapshot.json")
}

fn nanochat_state_dir(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    state_root(root, args).join("nanochat")
}

fn nanochat_latest_path(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    nanochat_state_dir(root, args).join("latest.json")
}

fn byterover_root(root: &Path, args: &HashMap<String, String>) -> PathBuf {
    let raw = clean_text(
        args.get("byterover-root").map_or(".brv", String::as_str),
        400,
    );
    let p = PathBuf::from(raw);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hex::encode(hasher.finalize())
}

fn normalize_rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn looks_binary(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let mut binary = 0usize;
    for b in bytes.iter().take(4096) {
        if *b == 0 {
            binary += 1;
            continue;
        }
        if *b < 0x09 || (*b > 0x0d && *b < 0x20) {
            binary += 1;
        }
    }
    binary > 24
}

fn extract_pdf_text(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut run = String::new();
    for b in bytes {
        let ch = *b as char;
        if ch.is_ascii_alphanumeric() || ch.is_ascii_punctuation() || ch.is_ascii_whitespace() {
            run.push(ch);
            continue;
        }
        if run.len() >= 6 {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(&run);
        }
        run.clear();
    }
    if run.len() >= 6 {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&run);
    }
    clean_text(&out, 200_000)
}

fn detect_mime(path: &Path) -> String {
    let ext = path
        .extension()
        .map(|v| v.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "md" | "markdown" => "text/markdown".to_string(),
        "txt" | "log" | "rst" => "text/plain".to_string(),
        "json" => "application/json".to_string(),
        "yaml" | "yml" => "application/yaml".to_string(),
        "csv" => "text/csv".to_string(),
        "pdf" => "application/pdf".to_string(),
        "html" | "htm" => "text/html".to_string(),
        "rs" | "ts" | "js" | "py" | "go" | "java" | "c" | "cpp" => "text/source".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

fn read_text_payload(path: &Path, mime: &str) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    if mime == "application/pdf" {
        let out = extract_pdf_text(&bytes);
        if out.is_empty() {
            return None;
        }
        return Some(out);
    }
    if looks_binary(&bytes) {
        return None;
    }
    let text = String::from_utf8_lossy(&bytes).to_string();
    let out = clean_text(&text, 500_000);
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn gather_supported_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        out.push(path.to_path_buf());
        return;
    }
    let Ok(read_dir) = fs::read_dir(path) else {
        return;
    };
    for row in read_dir.flatten() {
        let p = row.path();
        if p.is_dir() {
            gather_supported_files(&p, out);
            continue;
        }
        let mime = detect_mime(&p);
        if mime == "application/octet-stream" {
            continue;
        }
        out.push(p);
    }
}

fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<(usize, usize, String)> {
    if text.is_empty() {
        return vec![];
    }
    let chars = text.chars().collect::<Vec<char>>();
    let mut out = Vec::new();
    let mut start = 0usize;
    let safe_overlap = overlap.min(chunk_size.saturating_sub(1));
    while start < chars.len() {
        let end = (start + chunk_size).min(chars.len());
        let chunk = chars[start..end].iter().collect::<String>();
        let clean = clean_text(&chunk, chunk_size + 64);
        if !clean.is_empty() {
            out.push((start, end, clean));
        }
        if end == chars.len() {
            break;
        }
        start = end.saturating_sub(safe_overlap);
    }
    out
}

fn tokenize(text: &str) -> Vec<String> {
    let mut out = BTreeSet::new();
    for token in text.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        let t = token.trim().to_ascii_lowercase();
        if t.len() >= 2 {
            out.insert(t);
        }
    }
    out.into_iter().collect::<Vec<String>>()
}

fn parse_yyyy_mm_dd(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() < 10 {
        return String::new();
    }
    for i in 0..=(bytes.len() - 10) {
        let mut ok = true;
        for off in 0..10 {
            let b = bytes[i + off];
            if off == 4 || off == 7 {
                if b != b'-' {
                    ok = false;
                    break;
                }
            } else if !b.is_ascii_digit() {
                ok = false;
                break;
            }
        }
        if ok {
            return value[i..i + 10].to_string();
        }
    }
    String::new()
}

fn classify_what(source: &str, mime: &str, text: &str) -> String {
    let lower_source = source.to_ascii_lowercase();
    let lower = text.to_ascii_lowercase();
    if mime == "text/source"
        || lower_source.ends_with(".rs")
        || lower_source.ends_with(".ts")
        || lower_source.ends_with(".js")
        || lower_source.ends_with(".py")
    {
        return "code".to_string();
    }
    if lower_source.contains("receipt") || lower.contains("receipt_hash") {
        return "receipt".to_string();
    }
    if lower_source.contains("policy") || lower.contains("policy") || lower.contains("rule") {
        return "policy".to_string();
    }
    if lower_source.contains("memory") || lower.contains("epistemic") {
        return "memory".to_string();
    }
    if lower_source.contains("log") || lower.contains("error") || lower.contains("warn") {
        return "log".to_string();
    }
    "document".to_string()
}

fn classify_how(source: &str, mime: &str) -> String {
    let lower = source.to_ascii_lowercase();
    if lower.contains(".brv/") || lower.contains("context-tree") {
        return "context_tree".to_string();
    }
    if lower.contains("memory/") {
        return "memory_ingest".to_string();
    }
    if mime == "text/source" {
        return "code_index".to_string();
    }
    "document_ingest".to_string()
}

fn effective_which(args: &HashMap<String, String>) -> String {
    clean_text(
        args.get("which")
            .or_else(|| args.get("persona"))
            .map_or("default", String::as_str),
        100,
    )
}

fn metacognitive_enabled(root: &Path, args: &HashMap<String, String>) -> bool {
    let path = metacognitive_config_path(root, args);
    let Some(raw) = fs::read_to_string(path).ok() else {
        return false;
    };
    let Ok(v) = serde_json::from_str::<Value>(&raw) else {
        return false;
    };
    v.get("enabled").and_then(Value::as_bool).unwrap_or(false)
}

