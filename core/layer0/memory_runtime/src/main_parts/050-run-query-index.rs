
fn run_query_index(args: &HashMap<String, String>) {
    let out = query_index_payload(args);
    println!(
        "{}",
        serde_json::to_string(&out).expect("serialize query result")
    );
}

fn get_node_payload(args: &HashMap<String, String>) -> (serde_json::Value, i32) {
    let root = PathBuf::from(arg_or_default(args, "root", "."));
    let node_id = normalize_node_id(&arg_any(args, &["node-id", "node_id"]));
    let uid = normalize_uid(&arg_or_default(args, "uid", ""));
    let file_filter = normalize_file_ref(&arg_or_default(args, "file", ""));
    let cache_path = arg_or_default(args, "cache-path", "");
    let cache_max_bytes = parse_cache_max_bytes(&arg_or_default(args, "cache-max-bytes", ""));
    let mut cache = if cache_path.is_empty() {
        None
    } else {
        Some(load_working_set_cache(&cache_path))
    };

    let node_guard = enforce_node_only(&node_id, &uid);
    if !node_guard.ok {
        return (
            json!({
                "ok": false,
                "error": node_guard.reason_code
            }),
            2,
        );
    }

    let runtime_index = load_runtime_index(&root, args);
    let index_guard =
        enforce_index_first(&runtime_index.index_sources, runtime_index.entries.len());
    if !index_guard.ok {
        return (
            json!({
                "ok": false,
                "error": index_guard.reason_code
            }),
            2,
        );
    }
    let freshness = enforce_index_freshness(
        now_epoch_ms(),
        newest_runtime_index_mtime_ms(&root, &runtime_index),
        parse_u64_clamped(
            &arg_any(args, &["max-index-age-ms", "max_index_age_ms"]),
            1_000,
            7 * 24 * 60 * 60 * 1000,
            DEFAULT_INDEX_MAX_AGE_MS,
        ),
        parse_bool_arg(&arg_any(args, &["allow-stale", "allow_stale"]), false),
    );
    if !freshness.ok {
        return (
            json!({
                "ok": false,
                "error": freshness.reason_code,
                "freshness": {
                    "stale": freshness.stale,
                    "age_ms": freshness.age_ms,
                    "threshold_ms": freshness.threshold_ms
                }
            }),
            2,
        );
    }

    let mut matches = runtime_index
        .entries
        .into_iter()
        .filter(|entry| {
            if !uid.is_empty() && entry.uid != uid {
                return false;
            }
            if !node_id.is_empty() && entry.node_id != node_id {
                return false;
            }
            if !file_filter.is_empty() && entry.file_rel != file_filter {
                return false;
            }
            true
        })
        .collect::<Vec<IndexEntry>>();

    sort_entries_for_get(&mut matches);
    let Some(entry) = matches.first() else {
        return (
            json!({
                "ok": false,
                "error": "node_not_found",
                "node_id": if node_id.is_empty() { serde_json::Value::Null } else { json!(node_id) },
                "uid": if uid.is_empty() { serde_json::Value::Null } else { json!(uid) },
                "file": if file_filter.is_empty() { serde_json::Value::Null } else { json!(file_filter) }
            }),
            1,
        );
    };

    let section_pair = load_section_cached(&root, &entry.file_rel, &entry.node_id, cache.as_mut());
    let (section, section_hash) = match section_pair {
        Ok(pair) => pair,
        Err(reason) => {
            let mapped = if reason == "file_read_failed" {
                json!({
                    "ok": false,
                    "error": "file_read_failed",
                    "file": entry.file_rel
                })
            } else {
                json!({
                    "ok": false,
                    "error": "node_not_found",
                    "node_id": entry.node_id,
                    "file": entry.file_rel
                })
            };
            return (mapped, 1);
        }
    };

    if let Some(ref mut cache_ref) = cache {
        save_working_set_cache(&cache_path, cache_ref, cache_max_bytes);
    }

    if section.is_empty() {
        return (
            json!({
                "ok": false,
                "error": "node_not_found",
                "node_id": entry.node_id,
                "file": entry.file_rel
            }),
            1,
        );
    }

    let out = GetNodeResult {
        ok: true,
        backend: "protheus_memory_core".to_string(),
        node_id: entry.node_id.clone(),
        uid: entry.uid.clone(),
        file: entry.file_rel.clone(),
        summary: entry.summary.clone(),
        tags: dedupe_sorted(entry.tags.clone()),
        section_hash,
        section,
    };
    (
        serde_json::to_value(&out).expect("serialize get-node value"),
        0,
    )
}

fn run_get_node(args: &HashMap<String, String>) {
    let (payload, status_code) = get_node_payload(args);
    println!(
        "{}",
        serde_json::to_string(&payload).expect("serialize get-node payload")
    );
    if status_code != 0 {
        std::process::exit(status_code);
    }
}

fn build_index_payload(args: &HashMap<String, String>) -> BuildIndexResult {
    let root = PathBuf::from(arg_or_default(args, "root", "."));
    let write = parse_bool_flag(&arg_any(args, &["write", "save", "apply"]));
    let memory_index_path_raw = arg_any(args, &["memory-index-path", "memory_index_path"]);
    let tags_index_path_raw = arg_any(args, &["tags-index-path", "tags_index_path"]);

    let memory_index_abs = if memory_index_path_raw.is_empty() {
        root.join("client/memory/MEMORY_INDEX.md")
    } else {
        let p = PathBuf::from(memory_index_path_raw.clone());
        if p.is_absolute() {
            p
        } else {
            root.join(p)
        }
    };
    let tags_index_abs = if tags_index_path_raw.is_empty() {
        root.join("client/memory/TAGS_INDEX.md")
    } else {
        let p = PathBuf::from(tags_index_path_raw.clone());
        if p.is_absolute() {
            p
        } else {
            root.join(p)
        }
    };

    let (entries, files_scanned) = scan_daily_entries(&root);
    let memory_index_md = build_memory_index_doc(&entries);
    let (tags_index_md, tag_count) = build_tags_index_doc(&entries);
    let mut sqlite_rows_written: Option<usize> = None;
    let mut sqlite_path: Option<String> = None;

    let db_path = arg_any(args, &["db-path", "db_path"]);
    if let Ok(mut db) = MemoryDb::open(&root, &db_path) {
        let rel_path = db.rel_db_path(&root);
        let db_entries = entries
            .iter()
            .map(to_db_index_entry)
            .collect::<Vec<DbIndexEntry>>();
        match db.replace_index_entries(&db_entries, "daily_scan_build_index") {
            Ok(rows) => {
                sqlite_rows_written = Some(rows);
                sqlite_path = Some(rel_path.clone());
                let embedding_rows = entries
                    .iter()
                    .map(|entry| {
                        (
                            entry.node_id.clone(),
                            build_entry_embedding(entry, 64),
                            json!({
                                "node_id": entry.node_id,
                                "source": "daily_scan_build_index",
                                "tags": entry.tags
                            }),
                        )
                    })
                    .collect::<Vec<(String, Vec<f32>, serde_json::Value)>>();
                let embedding_written = db
                    .replace_embeddings(&embedding_rows, "daily_scan_build_index")
                    .unwrap_or(0);
                let _ = db.set_hot_state_json(
                    "build_index_memory_sha256",
                    &json!(sha256_hex(&memory_index_md)),
                );
                let _ = db.set_hot_state_json(
                    "build_index_tags_sha256",
                    &json!(sha256_hex(&tags_index_md)),
                );
                let _ = db.set_hot_state_json("build_index_node_count", &json!(entries.len()));
                let _ = db.set_hot_state_json("build_index_tag_count", &json!(tag_count));
                let _ =
                    db.set_hot_state_json("build_index_embedding_count", &json!(embedding_written));
                publish_memory_event(
                    &root,
                    "rust_memory_build_index",
                    json!({
                        "ok": true,
                        "node_count": entries.len(),
                        "tag_count": tag_count,
                        "embedding_count": embedding_written,
                        "files_scanned": files_scanned,
                        "sqlite_rows_written": rows,
                        "sqlite_path": rel_path
                    }),
                );
            }
            Err(err) => {
                publish_memory_event(
                    &root,
                    "rust_memory_build_index_error",
                    json!({
                        "ok": false,
                        "error": err
                    }),
                );
            }
        }
    }

    if write {
        if let Some(parent) = memory_index_abs.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Some(parent) = tags_index_abs.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&memory_index_abs, format!("{}\n", memory_index_md));
        let _ = fs::write(&tags_index_abs, format!("{}\n", tags_index_md));
    }

    BuildIndexResult {
        ok: true,
        backend: "protheus_memory_core".to_string(),
        node_count: entries.len(),
        tag_count,
        files_scanned,
        wrote_files: write,
        memory_index_path: rel_path(&root, &memory_index_abs),
        tags_index_path: rel_path(&root, &tags_index_abs),
        memory_index_sha256: sha256_hex(&memory_index_md),
        tags_index_sha256: sha256_hex(&tags_index_md),
        sqlite_path,
        sqlite_rows_written,
    }
}

fn run_build_index(args: &HashMap<String, String>) {
    let out = build_index_payload(args);
    println!(
        "{}",
        serde_json::to_string(&out).expect("serialize build-index result")
    );
}

fn verify_envelope_payload(args: &HashMap<String, String>) -> VerifyEnvelopeResult {
    let root = PathBuf::from(arg_or_default(args, "root", "."));
    let db_path_raw = arg_or_default(args, "db-path", "");
    let db = MemoryDb::open(&root, &db_path_raw).expect("open sqlite runtime");
    let stats = db
        .hot_state_envelope_stats()
        .unwrap_or_else(|_| HotStateEnvelopeStats::default());
    VerifyEnvelopeResult {
        ok: stats.total_rows == stats.enveloped_rows,
        backend: "rust_memory_box".to_string(),
        db_path: db.rel_db_path(&root),
        total_rows: stats.total_rows,
        enveloped_rows: stats.enveloped_rows,
        legacy_cipher_rows: stats.legacy_cipher_rows,
        plain_rows: stats.plain_rows,
    }
}

fn run_verify_envelope(args: &HashMap<String, String>) {
    let out = verify_envelope_payload(args);
    println!(
        "{}",
        serde_json::to_string(&out).expect("serialize verify-envelope result")
    );
}

fn run_value_payload(payload: Value) {
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
    println!(
        "{}",
        serde_json::to_string(&payload).expect("serialize value payload")
    );
    if !ok {
        std::process::exit(1);
    }
}

fn set_hot_state_payload(args: &HashMap<String, String>) -> serde_json::Value {
    let root = PathBuf::from(arg_or_default(args, "root", "."));
    let db_path_raw = arg_or_default(args, "db-path", "");
    let key = arg_or_default(args, "key", "");
    if key.trim().is_empty() {
        return json!({
            "ok": false,
            "error": "key_required"
        });
    }
    let value_raw = arg_any(args, &["value_json", "value"]);
    if value_raw.trim().is_empty() {
        return json!({
            "ok": false,
            "error": "value_json_required"
        });
    }
    let value = match serde_json::from_str::<serde_json::Value>(&value_raw) {
        Ok(v) => v,
        Err(_) => {
            return json!({
                "ok": false,
                "error": "value_json_invalid"
            })
        }
    };
    let db = match MemoryDb::open(&root, &db_path_raw) {
        Ok(v) => v,
        Err(err) => {
            return json!({
                "ok": false,
                "error": "db_open_failed",
                "reason": err
            })
        }
    };
    match db.set_hot_state_json(&key, &value) {
        Ok(_) => {
            publish_memory_event(
                &root,
                "rust_memory_hot_state_set",
                json!({
                    "ok": true,
                    "key": key
                }),
            );
            json!({
                "ok": true,
                "backend": "protheus_memory_core",
                "key": key,
                "db_path": db.rel_db_path(&root)
            })
        }
        Err(err) => json!({
            "ok": false,
            "error": "db_hot_state_set_failed",
            "reason": err
        }),
    }
}

fn run_set_hot_state(args: &HashMap<String, String>) {
    let out = set_hot_state_payload(args);
    println!(
        "{}",
        serde_json::to_string(&out).expect("serialize set-hot-state result")
    );
    if !out.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        std::process::exit(1);
    }
}

fn get_hot_state_payload(args: &HashMap<String, String>) -> serde_json::Value {
    let root = PathBuf::from(arg_or_default(args, "root", "."));
    let db_path_raw = arg_or_default(args, "db-path", "");
    let key = arg_or_default(args, "key", "");
    if key.trim().is_empty() {
        return json!({
            "ok": false,
            "error": "key_required"
        });
    }
    let db = match MemoryDb::open(&root, &db_path_raw) {
        Ok(v) => v,
        Err(err) => {
            return json!({
                "ok": false,
                "error": "db_open_failed",
                "reason": err
            })
        }
    };
    match db.get_hot_state_json(&key) {
        Ok(value) => json!({
            "ok": true,
            "backend": "protheus_memory_core",
            "key": key,
            "db_path": db.rel_db_path(&root),
            "value": value
        }),
        Err(err) => json!({
            "ok": false,
            "error": "db_hot_state_get_failed",
            "reason": err
        }),
    }
}

fn run_get_hot_state(args: &HashMap<String, String>) {
    let out = get_hot_state_payload(args);
    println!(
        "{}",
        serde_json::to_string(&out).expect("serialize get-hot-state result")
    );
    if !out.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        std::process::exit(1);
    }
}
