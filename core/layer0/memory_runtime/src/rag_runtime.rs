// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
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
        root.join("state").join("ops").join("local_rag")
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

fn load_index(path: &Path) -> Option<RagIndex> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<RagIndex>(&raw).ok()
}

fn write_index(path: &Path, index: &RagIndex) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(index) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn append_history(path: &Path, row: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(line) = serde_json::to_string(row) {
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| f.write_all(format!("{line}\n").as_bytes()));
    }
}

fn receipt(mut payload: Value) -> Value {
    let digest = sha256_hex(
        serde_json::to_string(&payload)
            .unwrap_or_default()
            .as_bytes(),
    );
    payload["receipt_hash"] = Value::String(digest);
    payload["receipt_deterministic"] = Value::Bool(true);
    payload
}

pub fn ingest_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let target_raw = clean_text(args.get("path").map_or("docs", String::as_str), 600);
    let target = {
        let p = PathBuf::from(target_raw.clone());
        if p.is_absolute() {
            p
        } else {
            root.join(p)
        }
    };
    let chunk_size = parse_usize(args.get("chunk-size"), 256, 4096, 900);
    let chunk_overlap = parse_usize(args.get("chunk-overlap"), 0, 1024, 120);
    let max_files = parse_usize(args.get("max-files"), 1, 10_000, 1000);
    let incremental = parse_bool(args.get("incremental"), true);
    let idx_path = index_path(&root, args);
    let hist_path = history_path(&root, args);

    let mut files = Vec::new();
    gather_supported_files(&target, &mut files);
    files.sort();
    files.truncate(max_files);

    let previous = load_index(&idx_path);
    let mut prev_by_source: HashMap<String, (String, Vec<RagChunk>)> = HashMap::new();
    if let Some(prev) = previous.clone() {
        let mut chunks_by_source: HashMap<String, Vec<RagChunk>> = HashMap::new();
        for chunk in prev.chunks {
            chunks_by_source
                .entry(chunk.source_path.clone())
                .or_default()
                .push(chunk);
        }
        for source in prev.sources {
            let chunks = chunks_by_source.remove(&source.path).unwrap_or_default();
            prev_by_source.insert(source.path.clone(), (source.sha256.clone(), chunks));
        }
    }

    let mut sources = Vec::new();
    let mut chunks = Vec::new();
    let mut reused_chunks = 0usize;
    let mut generated_chunks = 0usize;
    let mut active_sources = BTreeSet::new();
    let mut parse_errors = Vec::new();

    for file in files {
        let rel = normalize_rel_path(&root, &file);
        let mime = detect_mime(&file);
        let bytes = match fs::read(&file) {
            Ok(v) => v,
            Err(_) => {
                parse_errors.push(json!({"path": rel, "reason": "read_failed"}));
                continue;
            }
        };
        let source_sha = sha256_hex(&bytes);
        active_sources.insert(rel.clone());
        if incremental {
            if let Some((prev_sha, prev_chunks)) = prev_by_source.get(&rel) {
                if prev_sha == &source_sha && !prev_chunks.is_empty() {
                    reused_chunks += prev_chunks.len();
                    chunks.extend(prev_chunks.iter().cloned());
                    sources.push(RagSource {
                        path: rel.clone(),
                        mime: mime.clone(),
                        sha256: source_sha.clone(),
                        chunk_ids: prev_chunks
                            .iter()
                            .map(|c| c.chunk_id.clone())
                            .collect::<Vec<String>>(),
                    });
                    continue;
                }
            }
        }

        let text = match read_text_payload(&file, &mime) {
            Some(v) => v,
            None => {
                parse_errors.push(json!({"path": rel, "reason": "unsupported_or_empty"}));
                continue;
            }
        };
        let mut source_chunk_ids = Vec::new();
        for (start, end, chunk_text) in chunk_text(&text, chunk_size, chunk_overlap) {
            let seed = format!("{rel}|{start}|{end}|{}", sha256_hex(chunk_text.as_bytes()));
            let chunk_id = format!("chunk_{}", &sha256_hex(seed.as_bytes())[..20]);
            source_chunk_ids.push(chunk_id.clone());
            chunks.push(RagChunk {
                chunk_id,
                source_path: rel.clone(),
                mime: mime.clone(),
                offset_start: start,
                offset_end: end,
                text: chunk_text.clone(),
                sha256: sha256_hex(chunk_text.as_bytes()),
            });
            generated_chunks += 1;
        }
        sources.push(RagSource {
            path: rel,
            mime,
            sha256: source_sha,
            chunk_ids: source_chunk_ids,
        });
    }

    sources.sort_by(|a, b| a.path.cmp(&b.path));
    chunks.sort_by(|a, b| {
        let by_source = a.source_path.cmp(&b.source_path);
        if by_source != std::cmp::Ordering::Equal {
            return by_source;
        }
        a.offset_start.cmp(&b.offset_start)
    });

    let previous_sources = previous
        .map(|idx| {
            idx.sources
                .into_iter()
                .map(|s| s.path)
                .collect::<BTreeSet<String>>()
        })
        .unwrap_or_default();
    let tombstones = previous_sources
        .difference(&active_sources)
        .cloned()
        .collect::<Vec<String>>();

    let index = RagIndex {
        schema_version: "1.0".to_string(),
        generated_at: now_iso(),
        source_count: sources.len(),
        chunk_count: chunks.len(),
        sources,
        chunks,
        tombstones: tombstones.clone(),
    };
    write_index(&idx_path, &index);
    let index_hash = sha256_hex(
        serde_json::to_string(&index)
            .unwrap_or_else(|_| "{}".to_string())
            .as_bytes(),
    );

    let out = receipt(json!({
        "ok": true,
        "type": "local_rag_ingest",
        "backend": "protheus_memory_core",
        "schema_version": "1.0",
        "root": root.to_string_lossy().to_string(),
        "target": target_raw,
        "index_path": normalize_rel_path(&root, &idx_path),
        "source_count": index.source_count,
        "chunk_count": index.chunk_count,
        "generated_chunks": generated_chunks,
        "reused_chunks": reused_chunks,
        "tombstoned_sources": tombstones.len(),
        "parse_error_count": parse_errors.len(),
        "parse_errors": parse_errors,
        "index_sha256": index_hash
    }));
    append_history(&hist_path, &out);
    out
}

pub fn status_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let idx_path = index_path(&root, args);
    let hist_path = history_path(&root, args);
    let index = load_index(&idx_path);
    let out = match index {
        Some(idx) => json!({
            "ok": true,
            "type": "local_rag_status",
            "backend": "protheus_memory_core",
            "schema_version": idx.schema_version,
            "generated_at": idx.generated_at,
            "source_count": idx.source_count,
            "chunk_count": idx.chunk_count,
            "tombstone_count": idx.tombstones.len(),
            "index_path": normalize_rel_path(&root, &idx_path),
            "history_path": normalize_rel_path(&root, &hist_path)
        }),
        None => json!({
            "ok": false,
            "type": "local_rag_status",
            "error": "index_missing",
            "index_path": normalize_rel_path(&root, &idx_path)
        }),
    };
    receipt(out)
}

pub fn search_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let idx_path = index_path(&root, args);
    let hist_path = history_path(&root, args);
    let query = clean_text(args.get("q").map_or("", String::as_str), 1_000);
    if query.is_empty() {
        return receipt(json!({
            "ok": false,
            "type": "local_rag_search",
            "error": "query_required"
        }));
    }
    let top = parse_usize(args.get("top"), 1, 50, 5);
    let Some(index) = load_index(&idx_path) else {
        return receipt(json!({
            "ok": false,
            "type": "local_rag_search",
            "error": "index_missing",
            "index_path": normalize_rel_path(&root, &idx_path)
        }));
    };
    let query_tokens = tokenize(&query);
    let mut scored = Vec::new();
    for chunk in index.chunks {
        let hay = chunk.text.to_ascii_lowercase();
        let mut score = 0.0_f64;
        for token in &query_tokens {
            if hay.contains(token) {
                score += 1.0;
            }
        }
        if hay.contains(&query.to_ascii_lowercase()) {
            score += 2.0;
        }
        if score > 0.0 {
            scored.push((chunk, score));
        }
    }
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let max_score = scored.first().map(|row| row.1).unwrap_or(1.0).max(1.0);
    let hits = scored
        .into_iter()
        .take(top)
        .map(|(chunk, score)| {
            let confidence = (score / max_score).clamp(0.0, 1.0);
            json!({
                "source": chunk.source_path,
                "chunk_id": chunk.chunk_id,
                "offset_start": chunk.offset_start,
                "offset_end": chunk.offset_end,
                "confidence": ((confidence * 1000.0).round() / 1000.0),
                "preview": clean_text(&chunk.text, 280)
            })
        })
        .collect::<Vec<Value>>();

    let out = receipt(json!({
        "ok": true,
        "type": "local_rag_search",
        "backend": "protheus_memory_core",
        "query": query,
        "token_count": query_tokens.len(),
        "index_path": normalize_rel_path(&root, &idx_path),
        "hit_count": hits.len(),
        "hits": hits
    }));
    append_history(&hist_path, &out);
    out
}

pub fn chat_payload(args: &HashMap<String, String>) -> Value {
    let mut search_args = args.clone();
    search_args
        .entry("top".to_string())
        .or_insert_with(|| "4".to_string());
    let search = search_payload(&search_args);
    if !search.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return receipt(json!({
            "ok": false,
            "type": "local_rag_chat",
            "error": search.get("error").and_then(Value::as_str).unwrap_or("search_failed"),
            "search": search
        }));
    }
    let hits = search
        .get("hits")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let answer = if hits.is_empty() {
        "No matching document chunks were found for this question.".to_string()
    } else {
        let mut lines = vec!["Document-grounded answer:".to_string()];
        for (idx, row) in hits.iter().enumerate() {
            let source = row
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let preview = row.get("preview").and_then(Value::as_str).unwrap_or("");
            lines.push(format!("{}. {} — {}", idx + 1, source, preview));
        }
        lines.join("\n")
    };
    receipt(json!({
        "ok": true,
        "type": "local_rag_chat",
        "backend": "protheus_memory_core",
        "answer": answer,
        "citations": hits
    }))
}

pub fn merge_vault_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let idx_path = index_path(&root, args);
    let Some(index) = load_index(&idx_path) else {
        return receipt(json!({
            "ok": false,
            "type": "local_rag_merge_vault",
            "error": "index_missing",
            "index_path": normalize_rel_path(&root, &idx_path)
        }));
    };

    let memory_index_path = root.join("client").join("memory").join("MEMORY_INDEX.md");
    let existing = fs::read_to_string(&memory_index_path).unwrap_or_default();
    let mut existing_ids = BTreeSet::new();
    for line in existing.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let cols = trimmed
            .trim_matches('|')
            .split('|')
            .map(|v| clean_text(v, 120))
            .collect::<Vec<String>>();
        if cols.is_empty() {
            continue;
        }
        let id = clean_text(cols.first().map_or("", String::as_str), 100);
        if id.starts_with("rag.") {
            existing_ids.insert(id);
        }
    }

    let max_merge = parse_usize(args.get("max-merge"), 1, 5000, 200);
    let mut rows = Vec::new();
    let mut added = 0usize;
    for chunk in index.chunks.iter().take(max_merge) {
        let node_id = format!("rag.{}", &chunk.sha256[..12]);
        if existing_ids.contains(&node_id) {
            continue;
        }
        existing_ids.insert(node_id.clone());
        let uid = chunk.sha256.chars().take(24).collect::<String>();
        let summary = clean_text(&chunk.text, 160);
        rows.push(format!(
            "| {} | {} | {} | {} | rag imported |",
            node_id, uid, chunk.source_path, summary
        ));
        added += 1;
    }

    if added > 0 {
        if let Some(parent) = memory_index_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut out = String::new();
        if existing.trim().is_empty() {
            out.push_str("| node_id | uid | file | summary | tags |\n");
            out.push_str("| --- | --- | --- | --- | --- |\n");
        } else {
            out.push_str(&existing);
            if !existing.ends_with('\n') {
                out.push('\n');
            }
        }
        for row in rows {
            out.push_str(&row);
            out.push('\n');
        }
        let _ = fs::write(&memory_index_path, out);
    }

    let result = receipt(json!({
        "ok": true,
        "type": "local_rag_merge_vault",
        "backend": "protheus_memory_core",
        "index_path": normalize_rel_path(&root, &idx_path),
        "memory_index_path": normalize_rel_path(&root, &memory_index_path),
        "rows_added": added
    }));
    append_history(&history_path(&root, args), &result);
    result
}

pub fn byterover_upgrade_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let brv = byterover_root(&root, args);
    let ctx = brv.join("context-tree");
    let timeline = ctx.join("timeline.md");
    let facts = ctx.join("facts.md");
    let meaning = ctx.join("meaning.md");
    let rules = ctx.join("rules.md");
    let manifest = ctx.join("manifest.json");

    let _ = fs::create_dir_all(&ctx);
    let mut created = Vec::new();
    for (path, title) in [
        (&timeline, "Timeline"),
        (&facts, "Facts"),
        (&meaning, "Meaning"),
        (&rules, "Rules"),
    ] {
        if !path.exists() {
            let body = format!("# {title}\n\nInitialized by `memory-upgrade-byterover`.\n");
            if fs::write(path, body).is_ok() {
                created.push(normalize_rel_path(&root, path));
            }
        }
    }

    let snapshot = json!({
        "schema_version": "1.0",
        "profile": "byterover",
        "generated_at": now_iso(),
        "paths": {
            "timeline": normalize_rel_path(&root, &timeline),
            "facts": normalize_rel_path(&root, &facts),
            "meaning": normalize_rel_path(&root, &meaning),
            "rules": normalize_rel_path(&root, &rules)
        }
    });
    let _ = fs::write(
        &manifest,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
        ),
    );

    let out = receipt(json!({
        "ok": true,
        "type": "memory_upgrade_byterover",
        "backend": "protheus_memory_core",
        "schema_version": "1.0",
        "profile": "byterover",
        "root": normalize_rel_path(&root, &brv),
        "context_tree_path": normalize_rel_path(&root, &ctx),
        "manifest_path": normalize_rel_path(&root, &manifest),
        "files_created": created,
        "created_count": created.len()
    }));
    append_history(&history_path(&root, args), &out);
    out
}

pub fn stable_status_payload() -> Value {
    receipt(json!({
        "ok": true,
        "type": "memory_stable_api_status",
        "backend": "protheus_memory_core",
        "stable_api_version": "v1",
        "supported_versions": ["stable", "v1", "1"],
        "commands": [
            "stable-status",
            "stable-search",
            "stable-get-node",
            "stable-build-index",
            "memory-upgrade-byterover",
            "stable-memory-upgrade-byterover",
            "stable-rag-ingest",
            "stable-rag-search",
            "stable-rag-chat"
        ]
    }))
}

pub fn ensure_supported_version(args: &HashMap<String, String>) -> Result<String, Value> {
    let version = clean_text(args.get("api-version").map_or("stable", String::as_str), 20)
        .to_ascii_lowercase();
    let normalized = if version == "1" {
        "v1".to_string()
    } else {
        version
    };
    if normalized == "stable" || normalized == "v1" {
        Ok(normalized)
    } else {
        Err(receipt(json!({
            "ok": false,
            "type": "memory_stable_api_error",
            "error": "unsupported_api_version",
            "requested_version": normalized,
            "supported_versions": ["stable", "v1", "1"]
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        byterover_upgrade_payload, chat_payload, ensure_supported_version, ingest_payload,
        merge_vault_payload, search_payload, stable_status_payload, status_payload,
    };
    use std::collections::HashMap;
    use std::fs;

    fn base_args(root: &str) -> HashMap<String, String> {
        let mut args = HashMap::new();
        args.insert("root".to_string(), root.to_string());
        args.insert("path".to_string(), "docs".to_string());
        args
    }

    #[test]
    fn ingest_search_chat_merge_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let docs = dir.path().join("docs");
        fs::create_dir_all(&docs).expect("mkdir docs");
        fs::create_dir_all(dir.path().join("client/memory")).expect("mkdir memory");
        fs::write(
            docs.join("alpha.md"),
            "# Alpha\nThis document describes local rag indexing and memory retrieval.\n",
        )
        .expect("write alpha");
        fs::write(
            docs.join("beta.txt"),
            "The second document mentions retrieval confidence and citations.",
        )
        .expect("write beta");

        let args = base_args(&dir.path().to_string_lossy());
        let ingest = ingest_payload(&args);
        assert!(ingest.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(
            ingest
                .get("chunk_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= 2
        );

        let mut search_args = args.clone();
        search_args.insert("q".to_string(), "retrieval citations".to_string());
        let search = search_payload(&search_args);
        assert!(search.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(
            search
                .get("hit_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= 1
        );

        let chat = chat_payload(&search_args);
        assert!(chat.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(chat
            .get("answer")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("Document-grounded answer"));

        let merge = merge_vault_payload(&args);
        assert!(merge.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(
            merge
                .get("rows_added")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= 1
        );

        let status = status_payload(&args);
        assert!(status.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
    }

    #[test]
    fn incremental_reuses_unchanged_chunks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let docs = dir.path().join("docs");
        fs::create_dir_all(&docs).expect("mkdir docs");
        fs::write(
            docs.join("stable.md"),
            "Stable file for incremental ingest reuse behavior.",
        )
        .expect("write stable");
        let mut args = base_args(&dir.path().to_string_lossy());
        args.insert("incremental".to_string(), "true".to_string());
        let first = ingest_payload(&args);
        assert!(first.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        let second = ingest_payload(&args);
        assert!(second.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(
            second
                .get("reused_chunks")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= 1
        );
    }

    #[test]
    fn stable_api_version_gate_accepts_and_rejects_expected_values() {
        let mut ok_args = HashMap::new();
        ok_args.insert("api-version".to_string(), "1".to_string());
        assert_eq!(
            ensure_supported_version(&ok_args).expect("v1"),
            "v1".to_string()
        );

        let mut bad_args = HashMap::new();
        bad_args.insert("api-version".to_string(), "v9".to_string());
        let err = ensure_supported_version(&bad_args).expect_err("must reject");
        assert_eq!(
            err.get("error").and_then(|v| v.as_str()),
            Some("unsupported_api_version")
        );
    }

    #[test]
    fn stable_status_reports_expected_commands() {
        let out = stable_status_payload();
        assert!(out.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        let commands = out
            .get("commands")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(commands
            .iter()
            .any(|v| v.as_str() == Some("stable-rag-search")));
    }

    #[test]
    fn byterover_upgrade_materializes_context_tree() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut args = HashMap::new();
        args.insert("root".to_string(), dir.path().to_string_lossy().to_string());
        let out = byterover_upgrade_payload(&args);
        assert!(out.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(dir.path().join(".brv/context-tree/timeline.md").exists());
        assert!(dir.path().join(".brv/context-tree/manifest.json").exists());
    }
}
