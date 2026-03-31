
fn build_embedding_map_from_entries(
    entries: &[IndexEntry],
    dims: usize,
) -> HashMap<String, Vec<f32>> {
    let mut out = HashMap::new();
    for entry in entries {
        let vector = build_entry_embedding(entry, dims);
        if vector.is_empty() {
            continue;
        }
        out.insert(entry.node_id.clone(), vector);
    }
    out
}

fn daily_scan_signature(root: &Path) -> String {
    let memory_dir = root.join("memory");
    let Ok(entries) = fs::read_dir(&memory_dir) else {
        return String::new();
    };
    let mut rows: Vec<String> = vec![];
    for item in entries.flatten() {
        let name = item.file_name().to_string_lossy().to_string();
        if !is_date_memory_file(&name) {
            continue;
        }
        let file_path = memory_dir.join(&name);
        if let Ok(meta) = fs::metadata(&file_path) {
            let modified = meta
                .modified()
                .ok()
                .and_then(|v| v.duration_since(UNIX_EPOCH).ok())
                .map(|dur| dur.as_millis())
                .unwrap_or(0);
            rows.push(format!("{name}:{}:{modified}", meta.len()));
        }
    }
    if rows.is_empty() {
        return String::new();
    }
    rows.sort();
    sha256_hex(&rows.join("|"))
}

fn sanitize_event_token(raw: &str) -> String {
    let mut out = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn publish_memory_event(root: &Path, event: &str, payload: serde_json::Value) {
    let event_id = sanitize_event_token(event);
    if event_id.is_empty() {
        return;
    }
    let script = root
        .join("systems")
        .join("ops")
        .join("event_sourced_control_plane.js");
    if !script.exists() {
        return;
    }
    let payload_arg = format!("--payload_json={payload}");
    let _ = Command::new("node")
        .arg(script)
        .arg("append")
        .arg("--stream=memory")
        .arg(format!("--event={event_id}"))
        .arg(payload_arg)
        .current_dir(root)
        .output();
}

type RuntimeIndexSyncResult = (Vec<String>, Vec<String>, usize, bool, String);

fn sync_sqlite_runtime_index(
    root: &Path,
    db: &mut MemoryDb,
) -> Result<RuntimeIndexSyncResult, String> {
    let signature = daily_scan_signature(root);
    let previous = db
        .get_hot_state_json("daily_scan_signature")?
        .and_then(|value| value.as_str().map(|v| v.to_string()))
        .unwrap_or_default();
    let existing_rows = db.count_index_rows()?;
    if existing_rows > 0 && !signature.is_empty() && signature == previous {
        return Ok((
            vec!["daily_scan:unchanged".to_string()],
            vec!["daily_scan:unchanged".to_string()],
            existing_rows,
            false,
            signature,
        ));
    }

    let (entries, files_scanned) = scan_daily_entries(root);
    let index_sources = vec![format!("daily_scan:{files_scanned}_files")];
    let tag_sources = vec!["daily_scan:frontmatter_tags".to_string()];
    let db_entries = entries
        .iter()
        .map(to_db_index_entry)
        .collect::<Vec<DbIndexEntry>>();
    let inserted = db.replace_index_entries(&db_entries, "daily_scan_authority")?;
    let embedding_rows = entries
        .iter()
        .map(|entry| {
            (
                entry.node_id.clone(),
                build_entry_embedding(entry, 64),
                json!({
                    "node_id": entry.node_id,
                    "source": "daily_scan_authority",
                    "tags": entry.tags
                }),
            )
        })
        .collect::<Vec<(String, Vec<f32>, serde_json::Value)>>();
    let embedding_written = db.replace_embeddings(&embedding_rows, "daily_scan_authority")?;
    let _ = db.set_hot_state_json("daily_scan_signature", &json!(signature));
    let _ = db.set_hot_state_json("index_row_count", &json!(inserted));
    let _ = db.set_hot_state_json("embedding_row_count", &json!(embedding_written));
    let _ = db.set_hot_state_json("index_sync_source", &json!("daily_scan_authority"));
    let _ = db.set_hot_state_json("index_files_scanned", &json!(files_scanned));
    Ok((index_sources, tag_sources, inserted, true, signature))
}

fn load_runtime_index(root: &Path, args: &HashMap<String, String>) -> RuntimeIndexBundle {
    let mut out = RuntimeIndexBundle::default();
    let db_path = arg_any(args, &["db-path", "db_path"]);
    let disable_sqlite = parse_bool_arg(
        &arg_any(
            args,
            &["disable-sqlite", "disable_sqlite", "index-sqlite-disabled"],
        ),
        false,
    );
    if !disable_sqlite {
        if let Ok(mut db) = MemoryDb::open(root, &db_path) {
            out.sqlite_path = Some(db.rel_db_path(root));
            match sync_sqlite_runtime_index(root, &mut db) {
                Ok((idx_sources, tag_sources, row_count, wrote, signature)) => {
                    out.sqlite_sync_rows = row_count;
                    out.sqlite_sync_applied = wrote;
                    if wrote {
                        publish_memory_event(
                            root,
                            "rust_memory_index_sync",
                            json!({
                                "ok": true,
                                "row_count": row_count,
                                "signature": signature,
                                "sqlite_path": out.sqlite_path.clone().unwrap_or_default(),
                                "index_sources": idx_sources,
                                "tag_sources": tag_sources
                            }),
                        );
                    }
                }
                Err(sync_error) => {
                    publish_memory_event(
                        root,
                        "rust_memory_index_sync_error",
                        json!({
                            "ok": false,
                            "error": sync_error
                        }),
                    );
                }
            }
            if let Ok(db_rows) = db.load_index_entries() {
                if !db_rows.is_empty() {
                    out.entries = db_rows
                        .iter()
                        .map(from_db_index_entry)
                        .collect::<Vec<IndexEntry>>();
                    let sqlite_path = out
                        .sqlite_path
                        .clone()
                        .unwrap_or_else(|| "sqlite".to_string());
                    out.index_sources = vec![format!("sqlite:{sqlite_path}")];
                    out.tag_map = build_tag_map_from_entries(&out.entries);
                    out.tag_sources = vec![format!("sqlite:{sqlite_path}")];
                    out.embeddings = db
                        .load_embedding_map()
                        .unwrap_or_else(|_| build_embedding_map_from_entries(&out.entries, 64));
                    return out;
                }
            }
        }
    }

    let (entries, files_scanned) = scan_daily_entries(root);
    if !entries.is_empty() {
        out.index_sources = vec![format!("daily_scan_fallback:{files_scanned}_files")];
        out.tag_map = build_tag_map_from_entries(&entries);
        out.tag_sources = vec!["daily_scan_fallback:frontmatter_tags".to_string()];
        out.entries = entries;
        out.embeddings = build_embedding_map_from_entries(&out.entries, 64);
        return out;
    }

    let (index_sources, entries) = load_memory_index(root);
    let (tag_sources, tag_map) = load_tags_index(root);
    out.index_sources = index_sources;
    out.tag_sources = tag_sources;
    out.entries = entries;
    out.tag_map = tag_map;
    out.embeddings = build_embedding_map_from_entries(&out.entries, 64);
    out
}

fn file_mtime_ms(file_path: &Path) -> Option<u64> {
    let metadata = fs::metadata(file_path).ok()?;
    let modified = metadata.modified().ok()?;
    let dur = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(dur.as_millis() as u64)
}

fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_millis() as u64)
        .unwrap_or(0)
}

fn parse_bool_arg(raw: &str, fallback: bool) -> bool {
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

fn parse_u32_clamped(raw: &str, min: u32, max: u32, fallback: u32) -> u32 {
    raw.trim()
        .parse::<u32>()
        .ok()
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn parse_u64_clamped(raw: &str, min: u64, max: u64, fallback: u64) -> u64 {
    raw.trim()
        .parse::<u64>()
        .ok()
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn newest_runtime_index_mtime_ms(root: &Path, bundle: &RuntimeIndexBundle) -> Option<u64> {
    let mut newest = 0u64;
    let mut seen = false;
    let candidates = vec![
        root.join("client/memory/MEMORY_INDEX.md"),
        root.join("client/memory/TAGS_INDEX.md"),
        root.join("memory"),
    ];
    for candidate in candidates {
        if let Some(ts) = file_mtime_ms(&candidate) {
            newest = newest.max(ts);
            seen = true;
        }
    }
    if let Some(sqlite_rel) = bundle.sqlite_path.as_ref() {
        let sqlite_abs = root.join(sqlite_rel);
        if let Some(ts) = file_mtime_ms(&sqlite_abs) {
            newest = newest.max(ts);
            seen = true;
        }
    }
    if seen {
        Some(newest)
    } else {
        None
    }
}

fn estimate_hydration_tokens(bundle: &RuntimeIndexBundle) -> u32 {
    if !bundle.sqlite_sync_applied {
        return 0;
    }
    let base = (bundle.sqlite_sync_rows as u32).saturating_mul(2);
    base.clamp(0, 2_000)
}

fn query_error(
    reason: &str,
    index_sources: Vec<String>,
    tag_sources: Vec<String>,
    policy: Value,
    freshness: Option<Value>,
    burn_slo: Option<Value>,
) -> QueryResult {
    QueryResult {
        ok: false,
        backend: "protheus_memory_core".to_string(),
        score_mode: "hybrid".to_string(),
        vector_enabled: true,
        entries_total: 0,
        candidates_total: 0,
        index_sources,
        tag_sources,
        hits: vec![],
        error: Some(reason.to_string()),
        reason_code: Some(reason.to_string()),
        policy: Some(policy),
        burn_slo,
        freshness,
    }
}

fn parse_cache_max_bytes(raw: &str) -> usize {
    parse_clamped_usize(raw, 65536, 16 * 1024 * 1024, 1024 * 1024)
}

fn load_working_set_cache(cache_path: &str) -> WorkingSetCache {
    if cache_path.is_empty() {
        return WorkingSetCache {
            schema_version: "1.0".to_string(),
            nodes: HashMap::new(),
        };
    }
    let p = PathBuf::from(cache_path);
    let Ok(text) = fs::read_to_string(&p) else {
        return WorkingSetCache {
            schema_version: "1.0".to_string(),
            nodes: HashMap::new(),
        };
    };
    let parsed = serde_json::from_str::<WorkingSetCache>(&text).ok();
    let mut out = parsed.unwrap_or(WorkingSetCache {
        schema_version: "1.0".to_string(),
        nodes: HashMap::new(),
    });
    if out.schema_version.is_empty() {
        out.schema_version = "1.0".to_string();
    }
    out
}

fn cache_size_bytes(cache: &WorkingSetCache) -> usize {
    serde_json::to_vec(cache)
        .map(|bytes| bytes.len())
        .unwrap_or(0)
}

fn prune_working_set_cache(cache: &mut WorkingSetCache, max_bytes: usize) {
    if cache_size_bytes(cache) <= max_bytes {
        return;
    }
    let mut keys = cache.nodes.keys().cloned().collect::<Vec<String>>();
    keys.sort();
    for key in keys {
        if cache_size_bytes(cache) <= max_bytes {
            break;
        }
        cache.nodes.remove(&key);
    }
}

fn save_working_set_cache(cache_path: &str, cache: &mut WorkingSetCache, max_bytes: usize) {
    if cache_path.is_empty() {
        return;
    }
    prune_working_set_cache(cache, max_bytes);
    let p = PathBuf::from(cache_path);
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(body) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(p, format!("{body}\n"));
    }
}

fn cache_key(node_id: &str, file_rel: &str) -> String {
    format!("{node_id}@{file_rel}")
}

fn load_section_cached(
    root: &Path,
    file_rel: &str,
    node_id: &str,
    mut cache: Option<&mut WorkingSetCache>,
) -> Result<(String, String), String> {
    let file_abs = root.join(file_rel);
    let mtime = file_mtime_ms(&file_abs).ok_or_else(|| "file_read_failed".to_string())?;
    let key = cache_key(node_id, file_rel);

    if let Some(cache_ref) = cache.as_mut() {
        if let Some(entry) = cache_ref.nodes.get(&key) {
            if entry.mtime_ms == mtime && !entry.section_text.is_empty() {
                return Ok((entry.section_text.clone(), entry.section_hash.clone()));
            }
        }
    }

    let content = fs::read_to_string(&file_abs).map_err(|_| "file_read_failed".to_string())?;
    let section = extract_node_section(&content, node_id);
    if section.is_empty() {
        return Err("node_not_found".to_string());
    }
    let section_hash = sha256_hex(&section);

    if let Some(cache_ref) = cache.as_mut() {
        cache_ref.nodes.insert(
            key,
            CacheNode {
                mtime_ms: mtime,
                section_hash: section_hash.clone(),
                section_text: section.clone(),
            },
        );
    }

    Ok((section, section_hash))
}

fn parse_kv_args(args: &[String]) -> HashMap<String, String> {
    let mut out: HashMap<String, String> = HashMap::new();
    let mut idx = 0usize;
    while idx < args.len() {
        let token = args[idx].to_string();
        if !token.starts_with("--") {
            idx += 1;
            continue;
        }
        let raw = token.trim_start_matches("--").to_string();
        if let Some(eq_idx) = raw.find('=') {
            let key = raw[..eq_idx].to_string();
            let value = raw[eq_idx + 1..].to_string();
            out.insert(key, value);
            idx += 1;
            continue;
        }
        let key = raw;
        if idx + 1 < args.len() && !args[idx + 1].starts_with("--") {
            out.insert(key, args[idx + 1].to_string());
            idx += 2;
            continue;
        }
        out.insert(key, "true".to_string());
        idx += 1;
    }
    out
}

fn arg_or_default(args: &HashMap<String, String>, key: &str, fallback: &str) -> String {
    args.get(key)
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

fn arg_any(args: &HashMap<String, String>, keys: &[&str]) -> String {
    for key in keys {
        if let Some(v) = args.get(*key) {
            return v.clone();
        }
    }
    String::new()
}
