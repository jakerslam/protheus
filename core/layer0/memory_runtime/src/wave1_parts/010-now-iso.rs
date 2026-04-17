// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/memory_runtime (authoritative)

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use protheus_layer1_memory_runtime::recall_policy::{
    enforce_descending_ranking, enforce_index_freshness, DEFAULT_INDEX_MAX_AGE_MS,
};

#[derive(Clone, Debug)]
struct IndexEntry {
    node_id: String,
    file_rel: String,
    summary: String,
    tags: Vec<String>,
    date: String,
}

#[derive(Clone, Debug)]
struct MatrixNode {
    node_id: String,
    tags: Vec<String>,
    priority_score: f64,
    recency_score: f64,
    dream_score: f64,
    level_token: String,
    date: String,
    file: String,
    summary: String,
}

fn now_iso() -> String {
    let now = time::OffsetDateTime::now_utc();
    now.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_millis() as u64)
        .unwrap_or(0)
}

fn iso_to_epoch_ms(raw: &str) -> Option<u64> {
    let parsed =
        time::OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339).ok()?;
    let nanos = parsed.unix_timestamp_nanos();
    if nanos < 0 {
        None
    } else {
        Some((nanos as u64) / 1_000_000)
    }
}

fn file_mtime_ms(path: &Path) -> Option<u64> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let dur = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(dur.as_millis() as u64)
}

fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !ch.is_control()
                && !matches!(
                    ch,
                    '\u{200B}'
                        | '\u{200C}'
                        | '\u{200D}'
                        | '\u{200E}'
                        | '\u{200F}'
                        | '\u{202A}'
                        | '\u{202B}'
                        | '\u{202C}'
                        | '\u{202D}'
                        | '\u{202E}'
                        | '\u{2060}'
                        | '\u{FEFF}'
                )
        })
        .collect::<String>()
}

fn clean_text(raw: &str, max_len: usize) -> String {
    strip_invisible_unicode(raw)
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn normalize_node_id(raw: &str) -> String {
    let candidate = clean_text(raw.replace('`', "").as_str(), 160);
    if candidate.is_empty() {
        return String::new();
    }
    if candidate
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-')
    {
        candidate
    } else {
        String::new()
    }
}

fn normalize_tag(raw: &str) -> String {
    let mut out = clean_text(raw, 80).to_ascii_lowercase();
    while out.starts_with('#') {
        out.remove(0);
    }
    out.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .collect::<String>()
}

fn normalize_header_cell(raw: &str) -> String {
    let s = clean_text(raw, 80)
        .to_ascii_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if s.contains("node_id") {
        return "node_id".to_string();
    }
    if s.starts_with("uid") || s.ends_with("_uid") {
        return "uid".to_string();
    }
    if s.starts_with("file") {
        return "file".to_string();
    }
    if s.starts_with("summary") || s.starts_with("title") {
        return "summary".to_string();
    }
    if s.starts_with("tags") {
        return "tags".to_string();
    }
    s
}

fn normalize_file_ref(raw: &str) -> String {
    let mut out = clean_text(raw.trim_matches('"').trim_matches('\''), 260).replace('\\', "/");
    while out.starts_with("./") {
        out = out[2..].to_string();
    }
    if out.is_empty() {
        return String::new();
    }
    if out.starts_with("client/memory/") {
        return out;
    }
    if out.starts_with("memory/") {
        return format!("client/{out}");
    }
    if out.starts_with("_archive/") {
        return format!("client/memory/{out}");
    }
    if out.len() == 13
        && out.ends_with(".md")
        && out.chars().nth(4) == Some('-')
        && out.chars().nth(7) == Some('-')
    {
        return format!("client/memory/{out}");
    }
    if out.ends_with(".md") {
        return out;
    }
    String::new()
}

fn parse_tag_cell(raw: &str) -> Vec<String> {
    let mut tags = BTreeSet::new();
    for token in raw.replace(',', " ").split_whitespace() {
        let tag = normalize_tag(token);
        if !tag.is_empty() {
            tags.insert(tag);
        }
    }
    tags.into_iter().collect::<Vec<String>>()
}

fn parse_date_from_file(file_rel: &str) -> String {
    let bytes = file_rel.as_bytes();
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
            return file_rel[i..i + 10].to_string();
        }
    }
    String::new()
}

fn parse_index_file(path: &Path) -> Vec<IndexEntry> {
    let text = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let mut headers: Option<Vec<String>> = None;
    let mut rows: Vec<IndexEntry> = vec![];

    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let cells = trimmed
            .trim_matches('|')
            .split('|')
            .map(|cell| clean_text(cell, 512))
            .collect::<Vec<String>>();
        if cells.is_empty() {
            continue;
        }
        if cells
            .iter()
            .all(|cell| cell.chars().all(|ch| ch == '-' || ch == ':' || ch == ' '))
        {
            continue;
        }

        let normalized = cells
            .iter()
            .map(|cell| normalize_header_cell(cell))
            .collect::<Vec<String>>();
        if normalized.contains(&"node_id".to_string()) && normalized.contains(&"file".to_string()) {
            headers = Some(normalized);
            continue;
        }
        let Some(header) = headers.as_ref() else {
            continue;
        };

        let mut row: HashMap<String, String> = HashMap::new();
        for (idx, key) in header.iter().enumerate() {
            row.insert(key.clone(), cells.get(idx).cloned().unwrap_or_default());
        }

        let node_id = normalize_node_id(row.get("node_id").map_or("", String::as_str));
        let file_rel = normalize_file_ref(row.get("file").map_or("", String::as_str));
        if node_id.is_empty() || file_rel.is_empty() {
            continue;
        }

        rows.push(IndexEntry {
            node_id,
            summary: clean_text(row.get("summary").map_or("", String::as_str), 280),
            tags: parse_tag_cell(row.get("tags").map_or("", String::as_str)),
            date: parse_date_from_file(&file_rel),
            file_rel,
        });
    }

    rows
}

fn default_workspace_root(args: &HashMap<String, String>) -> PathBuf {
    let raw = args
        .get("root")
        .cloned()
        .or_else(|| env::var("MEMORY_RECALL_ROOT").ok())
        .unwrap_or_else(|| ".".to_string());
    let root = PathBuf::from(raw);
    if root.is_absolute() {
        root
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(root)
    }
}

fn resolve_path(root: &Path, raw: &str) -> PathBuf {
    let value = clean_text(raw, 512);
    if value.is_empty() {
        return root.to_path_buf();
    }
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        root.to_path_buf()
    } else {
        root.join(path)
    }
}

fn read_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn append_jsonl(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(line) = serde_json::to_string(value) {
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| f.write_all(format!("{line}\n").as_bytes()));
    }
}

fn level_profile(node_id: &str) -> (&'static str, f64) {
    let lower = node_id.to_ascii_lowercase();
    if lower.starts_with("node") {
        return ("node1", 1.0);
    }
    if lower.starts_with("tag") {
        return ("tag2", 0.67);
    }
    if lower.starts_with("jot") {
        return ("jot3", 0.34);
    }
    ("node1", 1.0)
}

fn recency_score(date: &str) -> f64 {
    if date.trim().is_empty() {
        0.4
    } else {
        1.0
    }
}

fn matrix_markdown(payload: &Value) -> String {
    let mut lines = vec![
        "# TAG_MEMORY_MATRIX.md".to_string(),
        format!(
            "Generated: {}",
            payload
                .get("generated_at")
                .and_then(Value::as_str)
                .unwrap_or("")
        ),
        String::new(),
        "| tag | tag_priority | node_count | top_nodes |".to_string(),
        "|-----|--------------|------------|-----------|".to_string(),
    ];

    if let Some(tags) = payload.get("tags").and_then(Value::as_array) {
        for row in tags {
            let tag = clean_text(row.get("tag").and_then(Value::as_str).unwrap_or(""), 80);
            let tag_priority = row
                .get("tag_priority")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let node_count = row.get("node_count").and_then(Value::as_u64).unwrap_or(0);
            let top_nodes = row
                .get("node_ids")
                .and_then(Value::as_array)
                .map(|ids| {
                    ids.iter()
                        .take(5)
                        .filter_map(Value::as_str)
                        .collect::<Vec<&str>>()
                        .join(", ")
                })
                .unwrap_or_default();
            lines.push(format!(
                "| {} | {:.4} | {} | {} |",
                tag, tag_priority, node_count, top_nodes
            ));
        }
    }
    lines.push(String::new());
    lines.join("\n")
}

fn matrix_paths(root: &Path, args: &HashMap<String, String>) -> (PathBuf, PathBuf, PathBuf) {
    let index_path = args
        .get("index-path")
        .cloned()
        .or_else(|| args.get("memory-index-path").cloned())
        .or_else(|| env::var("MEMORY_MATRIX_INDEX_PATH").ok())
        .or_else(|| {
            env::var("MEMORY_MATRIX_MEMORY_DIR")
                .ok()
                .map(|dir| format!("{}/MEMORY_INDEX.md", dir.trim_end_matches('/')))
        })
        .unwrap_or_else(|| "client/memory/MEMORY_INDEX.md".to_string());

    let json_path = args
        .get("matrix-json-path")
        .cloned()
        .or_else(|| env::var("MEMORY_MATRIX_JSON_PATH").ok())
        .unwrap_or_else(|| {
            "client/runtime/local/state/memory/matrix/tag_memory_matrix.json".to_string()
        });

    let md_path = args
        .get("matrix-md-path")
        .cloned()
        .or_else(|| env::var("MEMORY_MATRIX_MD_PATH").ok())
        .unwrap_or_else(|| "client/memory/TAG_MEMORY_MATRIX.md".to_string());

    (
        resolve_path(root, &index_path),
        resolve_path(root, &json_path),
        resolve_path(root, &md_path),
    )
}
