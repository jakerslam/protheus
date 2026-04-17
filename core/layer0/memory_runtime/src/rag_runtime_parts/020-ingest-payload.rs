fn append_metacognitive_note(root: &Path, args: &HashMap<String, String>, note: Value) {
    let path = metacognitive_journal_path(root, args);
    append_history(&path, &note);
}

fn load_history_rows(path: &Path) -> Vec<Value> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<Value>>()
}

fn read_json_file(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
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

fn normalize_rag_execution_status(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "ok" | "success" | "succeeded" | "ready" => "success",
        "timeout" | "timed_out" | "timed-out" => "timeout",
        "throttled" | "rate_limited" | "rate-limited" | "429" => "throttled",
        _ => "error",
    }
}

fn rag_execution_receipt(scope: &str, status: &str, error_kind: Option<&str>) -> Value {
    let normalized_status = normalize_rag_execution_status(status);
    let normalized_error = error_kind.map(|raw| clean_text(raw, 96).replace(' ', "_"));
    let seed = format!(
        "{}|{}|{}",
        clean_text(scope, 96),
        normalized_status,
        normalized_error.clone().unwrap_or_default()
    );
    json!({
        "call_id": format!("local-rag-{}", &sha256_hex(seed.as_bytes())[..16]),
        "status": normalized_status,
        "error_kind": normalized_error,
        "telemetry": {
            "duration_ms": 0,
            "tokens_used": 0
        }
    })
}

fn invalid_ingest_target(raw: &str) -> bool {
    if raw.trim().is_empty() || raw.chars().any(char::is_control) {
        return true;
    }
    let lowered = raw.trim().to_ascii_lowercase();
    lowered.starts_with("http:")
        || lowered.starts_with("https:")
        || lowered.starts_with("javascript:")
        || lowered.starts_with("data:")
        || lowered.starts_with("file:")
}

fn safe_ingest_source_path(raw: &str) -> bool {
    if raw.trim().is_empty() {
        return false;
    }
    let lowered = raw.trim().to_ascii_lowercase();
    !(lowered.starts_with("http:")
        || lowered.starts_with("https:")
        || lowered.starts_with("javascript:")
        || lowered.starts_with("data:")
        || lowered.starts_with("file:"))
}

fn receipt(mut payload: Value) -> Value {
    let scope = payload
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("local_rag");
    let status = if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        "success"
    } else {
        "error"
    };
    let error_kind = payload.get("error").and_then(Value::as_str);
    if payload.get("execution_receipt").is_none() {
        payload["execution_receipt"] = rag_execution_receipt(scope, status, error_kind);
    }
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
    if invalid_ingest_target(&target_raw) {
        return receipt(json!({
            "ok": false,
            "type": "local_rag_ingest",
            "error": "invalid_target_path"
        }));
    }
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
        if !safe_ingest_source_path(&chunk.source_path) {
            continue;
        }
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
