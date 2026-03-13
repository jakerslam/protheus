// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/memory_runtime (authoritative)

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
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

fn build_matrix_payload(args: &HashMap<String, String>) -> Value {
    let root = default_workspace_root(args);
    let (index_path, json_path, md_path) = matrix_paths(&root, args);
    let reason = clean_text(
        args.get("reason").map(String::as_str).unwrap_or("manual"),
        120,
    );
    let apply = args
        .get("apply")
        .map(|raw| {
            let lower = raw.trim().to_ascii_lowercase();
            !(lower == "0" || lower == "false" || lower == "off" || lower == "no")
        })
        .unwrap_or(true);

    let entries = parse_index_file(&index_path);
    if entries.is_empty() {
        return json!({
            "ok": false,
            "type": "tag_memory_matrix",
            "reason": "empty_memory_index",
            "index_path": index_path.to_string_lossy().to_string(),
            "matrix_path": json_path.to_string_lossy().to_string(),
            "markdown_path": md_path.to_string_lossy().to_string(),
            "generated_at": now_iso()
        });
    }

    let mut by_tag: BTreeMap<String, Vec<MatrixNode>> = BTreeMap::new();
    for entry in &entries {
        let (level_token, level_weight) = level_profile(&entry.node_id);
        let recency = recency_score(&entry.date);
        let priority = ((level_weight * 100.0) + (recency * 20.0) + 0.5).round() / 1.0;
        let node = MatrixNode {
            node_id: entry.node_id.clone(),
            tags: entry.tags.clone(),
            priority_score: priority,
            recency_score: recency,
            dream_score: 0.0,
            level_token: level_token.to_string(),
            date: entry.date.clone(),
            file: entry.file_rel.clone(),
            summary: entry.summary.clone(),
        };
        for tag in &entry.tags {
            by_tag.entry(tag.clone()).or_default().push(node.clone());
        }
    }

    let mut tag_rows = Vec::new();
    for (tag, mut nodes) in by_tag {
        nodes.sort_by(|a, b| {
            b.priority_score
                .total_cmp(&a.priority_score)
                .then_with(|| a.node_id.cmp(&b.node_id))
        });

        let ranking_scores = nodes
            .iter()
            .map(|row| row.priority_score)
            .collect::<Vec<f64>>();
        let ranking_ids = nodes
            .iter()
            .map(|row| row.node_id.clone())
            .collect::<Vec<String>>();
        let ranking = enforce_descending_ranking(&ranking_scores, &ranking_ids);
        if !ranking.ok {
            return json!({
                "ok": false,
                "type": "tag_memory_matrix",
                "reason": ranking.reason_code,
                "tag": tag,
                "index_path": index_path.to_string_lossy().to_string(),
                "matrix_path": json_path.to_string_lossy().to_string(),
                "markdown_path": md_path.to_string_lossy().to_string(),
                "generated_at": now_iso()
            });
        }

        let node_ids = nodes
            .iter()
            .map(|row| row.node_id.clone())
            .collect::<Vec<String>>();
        let tag_priority = nodes.first().map(|row| row.priority_score).unwrap_or(0.0);
        let nodes_json = nodes
            .iter()
            .map(|row| {
                json!({
                    "node_id": row.node_id,
                    "tags": row.tags,
                    "priority_score": row.priority_score,
                    "recency_score": row.recency_score,
                    "dream_score": row.dream_score,
                    "level_token": row.level_token,
                    "date": if row.date.is_empty() { Value::Null } else { Value::String(row.date.clone()) },
                    "file": if row.file.is_empty() { Value::Null } else { Value::String(row.file.clone()) },
                    "summary": if row.summary.is_empty() { Value::Null } else { Value::String(row.summary.clone()) }
                })
            })
            .collect::<Vec<Value>>();

        tag_rows.push(json!({
            "tag": tag,
            "tag_priority": tag_priority,
            "node_count": node_ids.len(),
            "node_ids": node_ids,
            "nodes": nodes_json
        }));
    }

    let payload = json!({
        "ok": true,
        "type": "tag_memory_matrix",
        "generated_at": now_iso(),
        "reason": reason,
        "stats": {
            "entries_total": entries.len(),
            "tags_total": tag_rows.len()
        },
        "index_path": index_path.to_string_lossy().to_string(),
        "matrix_path": json_path.to_string_lossy().to_string(),
        "markdown_path": md_path.to_string_lossy().to_string(),
        "tags": tag_rows
    });

    if apply {
        write_json(&json_path, &payload);
        if let Some(parent) = md_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&md_path, format!("{}\n", matrix_markdown(&payload)));
    }

    payload
}

fn matrix_status_payload(args: &HashMap<String, String>) -> Value {
    let root = default_workspace_root(args);
    let (_index_path, json_path, md_path) = matrix_paths(&root, args);
    let Some(payload) = read_json(&json_path) else {
        return json!({
            "ok": false,
            "type": "tag_memory_matrix_status",
            "reason": "missing_matrix",
            "matrix_path": json_path.to_string_lossy().to_string(),
            "markdown_path": md_path.to_string_lossy().to_string()
        });
    };

    let top_tags = payload
        .get("tags")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .take(8)
                .map(|row| {
                    json!({
                        "tag": row.get("tag").cloned().unwrap_or(Value::String(String::new())),
                        "tag_priority": row.get("tag_priority").cloned().unwrap_or(Value::from(0.0)),
                        "node_count": row.get("node_count").cloned().unwrap_or(Value::from(0))
                    })
                })
                .collect::<Vec<Value>>()
        })
        .unwrap_or_default();

    json!({
        "ok": true,
        "type": "tag_memory_matrix_status",
        "matrix_path": json_path.to_string_lossy().to_string(),
        "markdown_path": md_path.to_string_lossy().to_string(),
        "generated_at": payload.get("generated_at").cloned().unwrap_or(Value::Null),
        "tags_indexed": payload.get("tags").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "top_tags": top_tags
    })
}

fn parse_tags_arg(raw: &str) -> Vec<String> {
    let mut out = BTreeSet::new();
    for token in raw.split(',') {
        let tag = normalize_tag(token);
        if !tag.is_empty() {
            out.insert(tag);
        }
    }
    out.into_iter().collect::<Vec<String>>()
}

fn parse_bool_value(raw: &str, fallback: bool) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return fallback;
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn load_auto_recall_policy(path: &Path) -> Value {
    let defaults = json!({
        "enabled": true,
        "dry_run": false,
        "min_shared_tags": 1,
        "max_matches": 3,
        "max_matrix_age_ms": 1200000,
        "enqueue_to_attention": true,
        "summary_max_chars": 180,
        "recall_window_days": 90,
        "min_priority_score": 8
    });
    let Some(user) = read_json(path) else {
        return defaults;
    };

    let mut merged = defaults;
    if let Some(user_obj) = user.as_object() {
        let merged_obj = merged.as_object_mut().expect("object");
        for (k, v) in user_obj {
            merged_obj.insert(k.clone(), v.clone());
        }
    }
    merged
}

fn intersect_count(a: &[String], b: &[String]) -> usize {
    let bset = b.iter().cloned().collect::<BTreeSet<String>>();
    a.iter().filter(|tag| bset.contains(*tag)).count()
}

fn memory_auto_recall_paths(
    root: &Path,
    args: &HashMap<String, String>,
) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let matrix_path = args
        .get("matrix-path")
        .cloned()
        .or_else(|| env::var("MEMORY_MATRIX_JSON_PATH").ok())
        .unwrap_or_else(|| {
            "client/runtime/local/state/memory/matrix/tag_memory_matrix.json".to_string()
        });
    let policy_path = args
        .get("policy-path")
        .cloned()
        .or_else(|| env::var("MEMORY_AUTO_RECALL_POLICY_PATH").ok())
        .unwrap_or_else(|| "client/runtime/config/memory_auto_recall_policy.json".to_string());
    let events_path = args
        .get("events-path")
        .cloned()
        .or_else(|| env::var("MEMORY_AUTO_RECALL_EVENTS_PATH").ok())
        .unwrap_or_else(|| {
            "client/runtime/local/state/memory/auto_recall/events.jsonl".to_string()
        });
    let latest_path = args
        .get("latest-path")
        .cloned()
        .or_else(|| env::var("MEMORY_AUTO_RECALL_LATEST_PATH").ok())
        .unwrap_or_else(|| "client/runtime/local/state/memory/auto_recall/latest.json".to_string());

    (
        resolve_path(root, &matrix_path),
        resolve_path(root, &policy_path),
        resolve_path(root, &events_path),
        resolve_path(root, &latest_path),
    )
}

fn auto_recall_filed_payload(args: &HashMap<String, String>) -> Value {
    let root = default_workspace_root(args);
    let (matrix_path, policy_path, events_path, latest_path) =
        memory_auto_recall_paths(&root, args);
    let policy = load_auto_recall_policy(&policy_path);

    let node_id = normalize_node_id(
        args.get("node-id")
            .or_else(|| args.get("node_id"))
            .map(String::as_str)
            .unwrap_or(""),
    );
    let tags = parse_tags_arg(args.get("tags").map(String::as_str).unwrap_or(""));
    let dry_run = args
        .get("dry-run")
        .map(|raw| {
            let lower = raw.trim().to_ascii_lowercase();
            lower == "1" || lower == "true" || lower == "yes" || lower == "on"
        })
        .unwrap_or_else(|| {
            policy
                .get("dry_run")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        });

    if node_id.is_empty() || tags.is_empty() {
        let out = json!({
            "ok": false,
            "type": "memory_auto_recall",
            "reason": "missing_node_or_tags",
            "node_id": if node_id.is_empty() { Value::Null } else { Value::String(node_id.clone()) },
            "tags": tags,
            "ts": now_iso()
        });
        append_jsonl(&events_path, &out);
        write_json(&latest_path, &out);
        return out;
    }

    if !policy
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        let out = json!({
            "ok": true,
            "type": "memory_auto_recall",
            "skipped": true,
            "reason": "disabled",
            "node_id": node_id,
            "tags": tags,
            "ts": now_iso()
        });
        append_jsonl(&events_path, &out);
        write_json(&latest_path, &out);
        return out;
    }

    let Some(matrix) = read_json(&matrix_path) else {
        let out = json!({
            "ok": false,
            "type": "memory_auto_recall",
            "reason": "matrix_unavailable",
            "node_id": node_id,
            "tags": tags,
            "ts": now_iso()
        });
        append_jsonl(&events_path, &out);
        write_json(&latest_path, &out);
        return out;
    };

    let max_matrix_age_ms = policy
        .get("max_matrix_age_ms")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_INDEX_MAX_AGE_MS);
    let allow_stale_matrix = parse_bool_value(
        args.get("allow-stale-matrix")
            .or_else(|| args.get("allow_stale_matrix"))
            .map(String::as_str)
            .unwrap_or(""),
        false,
    );
    let matrix_generated_ms = matrix
        .get("generated_at")
        .and_then(Value::as_str)
        .and_then(iso_to_epoch_ms)
        .or_else(|| file_mtime_ms(&matrix_path));
    let freshness = enforce_index_freshness(
        now_epoch_ms(),
        matrix_generated_ms,
        max_matrix_age_ms,
        allow_stale_matrix,
    );
    if !freshness.ok {
        let out = json!({
            "ok": false,
            "type": "memory_auto_recall",
            "reason": freshness.reason_code,
            "stale": freshness.stale,
            "age_ms": freshness.age_ms,
            "threshold_ms": freshness.threshold_ms,
            "node_id": node_id,
            "tags": tags,
            "matrix_path": matrix_path.to_string_lossy().to_string(),
            "ts": now_iso()
        });
        append_jsonl(&events_path, &out);
        write_json(&latest_path, &out);
        return out;
    }

    let min_shared = policy
        .get("min_shared_tags")
        .and_then(Value::as_u64)
        .unwrap_or(1) as usize;
    let max_matches = policy
        .get("max_matches")
        .and_then(Value::as_u64)
        .unwrap_or(3) as usize;
    let min_priority = policy
        .get("min_priority_score")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    let mut candidates: BTreeMap<String, Value> = BTreeMap::new();
    if let Some(tag_rows) = matrix.get("tags").and_then(Value::as_array) {
        for source_tag in &tags {
            let Some(tag_row) = tag_rows
                .iter()
                .find(|row| row.get("tag").and_then(Value::as_str).unwrap_or("") == source_tag)
            else {
                continue;
            };
            let nodes = tag_row
                .get("nodes")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();

            for node in nodes {
                let candidate_id =
                    normalize_node_id(node.get("node_id").and_then(Value::as_str).unwrap_or(""));
                if candidate_id.is_empty() || candidate_id == node_id {
                    continue;
                }
                let candidate_tags = node
                    .get("tags")
                    .and_then(Value::as_array)
                    .map(|rows| {
                        rows.iter()
                            .filter_map(Value::as_str)
                            .map(normalize_tag)
                            .filter(|tag| !tag.is_empty())
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default();
                let shared = intersect_count(&tags, &candidate_tags);
                if shared < min_shared {
                    continue;
                }
                let priority = node
                    .get("priority_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                if priority < min_priority {
                    continue;
                }
                let recency = node
                    .get("recency_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let dream = node
                    .get("dream_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let score =
                    (shared as f64 * 50.0) + (priority * 0.85) + (dream * 12.0) + (recency * 8.0);
                let shared_tags = tags
                    .iter()
                    .filter(|tag| candidate_tags.contains(tag))
                    .cloned()
                    .collect::<Vec<String>>();

                let next = json!({
                    "node_id": candidate_id,
                    "file": node.get("file").cloned().unwrap_or(Value::Null),
                    "date": node.get("date").cloned().unwrap_or(Value::Null),
                    "summary": node.get("summary").cloned().unwrap_or(Value::Null),
                    "level_token": node.get("level_token").cloned().unwrap_or(Value::Null),
                    "priority_score": priority,
                    "score": score,
                    "shared_tags": shared_tags
                });

                let replace = candidates
                    .get(&candidate_id)
                    .and_then(|cur| cur.get("score").and_then(Value::as_f64))
                    .map(|current| score > current)
                    .unwrap_or(true);
                if replace {
                    candidates.insert(candidate_id, next);
                }
            }
        }
    }

    let mut matches = candidates.into_values().collect::<Vec<Value>>();
    matches.sort_by(|a, b| {
        let sb = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let sa = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        sb.total_cmp(&sa).then_with(|| {
            let an = a.get("node_id").and_then(Value::as_str).unwrap_or("");
            let bn = b.get("node_id").and_then(Value::as_str).unwrap_or("");
            an.cmp(bn)
        })
    });
    matches.truncate(max_matches.max(1));

    let ranking_scores = matches
        .iter()
        .map(|row| row.get("score").and_then(Value::as_f64).unwrap_or(0.0))
        .collect::<Vec<f64>>();
    let ranking_ids = matches
        .iter()
        .map(|row| normalize_node_id(row.get("node_id").and_then(Value::as_str).unwrap_or("")))
        .collect::<Vec<String>>();
    let ranking = enforce_descending_ranking(&ranking_scores, &ranking_ids);
    if !ranking.ok {
        let out = json!({
            "ok": false,
            "type": "memory_auto_recall",
            "reason": ranking.reason_code,
            "node_id": node_id,
            "tags": tags,
            "ts": now_iso()
        });
        append_jsonl(&events_path, &out);
        write_json(&latest_path, &out);
        return out;
    }

    if matches.is_empty() {
        let out = json!({
            "ok": true,
            "type": "memory_auto_recall",
            "skipped": true,
            "reason": "no_matches",
            "node_id": node_id,
            "tags": tags,
            "ts": now_iso()
        });
        append_jsonl(&events_path, &out);
        write_json(&latest_path, &out);
        return out;
    }

    let attention = if dry_run {
        json!({
            "ok": true,
            "skipped": true,
            "reason": "dry_run_or_queue_disabled",
            "queued": false,
            "routed_via": "none"
        })
    } else {
        json!({
            "ok": true,
            "skipped": true,
            "reason": "queue_not_wired_in_wave1",
            "queued": false,
            "routed_via": "none"
        })
    };

    let out = json!({
        "ok": true,
        "type": "memory_auto_recall",
        "ts": now_iso(),
        "node_id": node_id,
        "tags": tags,
        "matches": matches,
        "match_count": matches.len(),
        "dry_run": dry_run,
        "matrix_path": matrix_path.to_string_lossy().to_string(),
        "freshness": {
            "ok": freshness.ok,
            "stale": freshness.stale,
            "reason_code": freshness.reason_code,
            "age_ms": freshness.age_ms,
            "threshold_ms": freshness.threshold_ms
        },
        "ranking_invariants": {
            "ok": ranking.ok,
            "reason_code": ranking.reason_code
        },
        "attention": attention
    });
    append_jsonl(&events_path, &out);
    write_json(&latest_path, &out);
    out
}

fn auto_recall_status_payload(args: &HashMap<String, String>) -> Value {
    let root = default_workspace_root(args);
    let (matrix_path, policy_path, events_path, latest_path) =
        memory_auto_recall_paths(&root, args);
    let latest = read_json(&latest_path).unwrap_or(Value::Null);
    json!({
        "ok": true,
        "type": "memory_auto_recall_status",
        "policy": load_auto_recall_policy(&policy_path),
        "latest": latest,
        "paths": {
            "events": events_path.to_string_lossy().to_string(),
            "latest": latest_path.to_string_lossy().to_string(),
            "matrix": matrix_path.to_string_lossy().to_string()
        }
    })
}

fn dream_paths(root: &Path, args: &HashMap<String, String>) -> (PathBuf, PathBuf, PathBuf) {
    let matrix_path = args
        .get("matrix-path")
        .cloned()
        .or_else(|| env::var("MEMORY_MATRIX_JSON_PATH").ok())
        .unwrap_or_else(|| {
            "client/runtime/local/state/memory/matrix/tag_memory_matrix.json".to_string()
        });
    let state_path = args
        .get("state-path")
        .cloned()
        .or_else(|| env::var("DREAM_SEQUENCER_STATE_PATH").ok())
        .unwrap_or_else(|| {
            "client/runtime/local/state/memory/dream_sequencer/latest.json".to_string()
        });
    let ledger_path = args
        .get("ledger-path")
        .cloned()
        .or_else(|| env::var("DREAM_SEQUENCER_LEDGER_PATH").ok())
        .unwrap_or_else(|| {
            "client/runtime/local/state/memory/dream_sequencer/runs.jsonl".to_string()
        });

    (
        resolve_path(root, &matrix_path),
        resolve_path(root, &state_path),
        resolve_path(root, &ledger_path),
    )
}

fn dream_run_payload(args: &HashMap<String, String>) -> Value {
    let root = default_workspace_root(args);
    let (matrix_path, state_path, ledger_path) = dream_paths(&root, args);
    let reason = clean_text(
        args.get("reason").map(String::as_str).unwrap_or("manual"),
        120,
    );
    let top_tags = args
        .get("top-tags")
        .or_else(|| args.get("top_tags"))
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(12)
        .clamp(1, 64);
    let apply = args
        .get("apply")
        .map(|raw| {
            let lower = raw.trim().to_ascii_lowercase();
            !(lower == "0" || lower == "false" || lower == "off" || lower == "no")
        })
        .unwrap_or(true);

    let matrix = read_json(&matrix_path).or_else(|| {
        let out = build_matrix_payload(&HashMap::from([
            ("root".to_string(), root.to_string_lossy().to_string()),
            ("apply".to_string(), "true".to_string()),
            ("reason".to_string(), "dream_sequencer_refresh".to_string()),
            (
                "matrix-json-path".to_string(),
                matrix_path.to_string_lossy().to_string(),
            ),
        ]));
        if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            Some(out)
        } else {
            None
        }
    });

    let Some(matrix_payload) = matrix else {
        let fail = json!({
            "ok": false,
            "type": "dream_sequencer",
            "reason": "matrix_unavailable",
            "ts": now_iso()
        });
        append_jsonl(&ledger_path, &fail);
        return fail;
    };

    let top = matrix_payload
        .get("tags")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .take(top_tags)
                .map(|row| {
                    json!({
                        "tag": row.get("tag").cloned().unwrap_or(Value::Null),
                        "tag_priority": row.get("tag_priority").cloned().unwrap_or(Value::from(0.0)),
                        "node_count": row.get("node_count").cloned().unwrap_or(Value::from(0)),
                        "top_nodes": row
                            .get("node_ids")
                            .and_then(Value::as_array)
                            .map(|ids| ids.iter().take(5).cloned().collect::<Vec<Value>>())
                            .unwrap_or_default()
                    })
                })
                .collect::<Vec<Value>>()
        })
        .unwrap_or_default();

    let out = json!({
        "ok": true,
        "type": "dream_sequencer",
        "ts": now_iso(),
        "reason": reason,
        "applied": apply,
        "matrix_path": matrix_path.to_string_lossy().to_string(),
        "stats": matrix_payload.get("stats").cloned().unwrap_or(Value::Null),
        "top_tags": top
    });

    if apply {
        write_json(&state_path, &out);
    }
    append_jsonl(&ledger_path, &out);
    out
}

fn dream_status_payload(args: &HashMap<String, String>) -> Value {
    let root = default_workspace_root(args);
    let (matrix_path, state_path, ledger_path) = dream_paths(&root, args);
    json!({
        "ok": true,
        "type": "dream_sequencer_status",
        "latest": read_json(&state_path).unwrap_or(Value::Null),
        "matrix": if matrix_path.exists() {
            json!({"ok": true, "exists": true, "path": matrix_path.to_string_lossy().to_string()})
        } else {
            json!({"ok": false, "exists": false, "path": matrix_path.to_string_lossy().to_string()})
        },
        "sequencer_state_path": state_path.to_string_lossy().to_string(),
        "sequencer_ledger_path": ledger_path.to_string_lossy().to_string()
    })
}

pub fn memory_matrix_payload(args: &HashMap<String, String>) -> Value {
    let action = args
        .get("action")
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "run".to_string());
    if action == "status" {
        matrix_status_payload(args)
    } else {
        build_matrix_payload(args)
    }
}

pub fn memory_auto_recall_payload(args: &HashMap<String, String>) -> Value {
    let action = args
        .get("action")
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if action == "status" {
        auto_recall_status_payload(args)
    } else {
        auto_recall_filed_payload(args)
    }
}

pub fn dream_sequencer_payload(args: &HashMap<String, String>) -> Value {
    let action = args
        .get("action")
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "run".to_string());
    if action == "status" {
        dream_status_payload(args)
    } else {
        dream_run_payload(args)
    }
}

pub fn print_payload_and_exit_code(payload: Value) -> i32 {
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
    println!(
        "{}",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{\"ok\":false}".to_string())
    );
    if ok {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<String, String>>()
    }

    #[test]
    fn auto_recall_blocks_stale_matrix() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let matrix_path = tmp.path().join("state/matrix.json");
        let policy_path = tmp.path().join("state/policy.json");
        fs::create_dir_all(matrix_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &matrix_path,
            serde_json::to_string_pretty(&json!({
                "generated_at": "2000-01-01T00:00:00Z",
                "tags": [{
                    "tag": "memory",
                    "nodes": [{
                        "node_id": "node.alpha",
                        "tags": ["memory"],
                        "priority_score": 10.0,
                        "recency_score": 1.0,
                        "dream_score": 0.0
                    }]
                }]
            }))
            .expect("encode"),
        )
        .expect("write matrix");
        fs::write(
            &policy_path,
            serde_json::to_string_pretty(&json!({
                "max_matrix_age_ms": 10
            }))
            .expect("encode"),
        )
        .expect("write policy");

        let root = tmp.path().to_string_lossy().to_string();
        let matrix = matrix_path.to_string_lossy().to_string();
        let policy = policy_path.to_string_lossy().to_string();
        let out = memory_auto_recall_payload(&map(&[
            ("root", root.as_str()),
            ("action", "filed"),
            ("node-id", "node.seed"),
            ("tags", "memory"),
            ("matrix-path", matrix.as_str()),
            ("policy-path", policy.as_str()),
        ]));
        assert_eq!(out["ok"], false);
        assert_eq!(out["reason"], "index_stale_blocked");
    }

    #[test]
    fn auto_recall_produces_sorted_matches_with_invariant_receipt() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let matrix_path = tmp.path().join("state/matrix.json");
        fs::create_dir_all(matrix_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &matrix_path,
            serde_json::to_string_pretty(&json!({
                "generated_at": now_iso(),
                "tags": [{
                    "tag": "memory",
                    "nodes": [
                        {
                            "node_id": "node.low",
                            "tags": ["memory"],
                            "priority_score": 9.0,
                            "recency_score": 1.0,
                            "dream_score": 0.0
                        },
                        {
                            "node_id": "node.high",
                            "tags": ["memory"],
                            "priority_score": 20.0,
                            "recency_score": 1.0,
                            "dream_score": 0.0
                        }
                    ]
                }]
            }))
            .expect("encode"),
        )
        .expect("write matrix");

        let root = tmp.path().to_string_lossy().to_string();
        let matrix = matrix_path.to_string_lossy().to_string();
        let out = memory_auto_recall_payload(&map(&[
            ("root", root.as_str()),
            ("action", "filed"),
            ("node-id", "node.seed"),
            ("tags", "memory"),
            ("matrix-path", matrix.as_str()),
            ("allow-stale-matrix", "1"),
        ]));
        assert_eq!(out["ok"], true);
        assert_eq!(out["ranking_invariants"]["ok"], true);
        assert_eq!(out["matches"][0]["node_id"], "node.high");
    }
}
