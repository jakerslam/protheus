fn append_jsonl(file_path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("adaptive_layer_store_kernel_create_dir_failed:{err}"))?;
    }
    let mut handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .map_err(|err| format!("adaptive_layer_store_kernel_append_open_failed:{err}"))?;
    handle
        .write_all(
            format!(
                "{}\n",
                serde_json::to_string(row).unwrap_or_else(|_| "null".to_string())
            )
            .as_bytes(),
        )
        .map_err(|err| format!("adaptive_layer_store_kernel_append_failed:{err}"))?;
    Ok(())
}

fn append_mutation_log(root: &Path, payload: &Map<String, Value>, event: &Value) {
    let path = mutation_log_path(root, payload);
    let _ = append_jsonl(&path, event);
}

fn meta_actor(meta: &Map<String, Value>) -> String {
    let value = clean_text(meta.get("actor"), 80);
    if value.is_empty() {
        std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())
    } else {
        value
    }
}

fn meta_source(meta: &Map<String, Value>) -> String {
    clean_text(meta.get("source"), 120)
}

fn meta_reason(meta: &Map<String, Value>, fallback: &str) -> String {
    let value = clean_text(meta.get("reason"), 160);
    if value.is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

fn pointer_index_load(root: &Path, payload: &Map<String, Value>) -> Value {
    let path = adaptive_pointer_index_path(root, payload);
    let value =
        read_json_value(&path).unwrap_or_else(|| json!({ "version": "1.0", "pointers": {} }));
    if value.get("pointers").and_then(Value::as_object).is_some() {
        value
    } else {
        json!({ "version": "1.0", "pointers": {} })
    }
}

fn pointer_index_save(
    root: &Path,
    payload: &Map<String, Value>,
    index: &Value,
) -> Result<(), String> {
    let path = adaptive_pointer_index_path(root, payload);
    write_json_atomic(&path, index)
}

fn append_adaptive_pointer_rows(
    root: &Path,
    payload: &Map<String, Value>,
    rows: &[Value],
) -> Result<Value, String> {
    if rows.is_empty() {
        return Ok(json!({ "emitted": 0, "skipped": 0 }));
    }
    let mut index = pointer_index_load(root, payload);
    let pointers = index
        .get_mut("pointers")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "adaptive_layer_store_kernel_invalid_pointer_index".to_string())?;
    let path = adaptive_pointers_path(root, payload);
    let mut emitted = 0_u64;
    let mut skipped = 0_u64;
    for row in rows {
        let kind = clean_text(row.get("kind"), 120);
        let uid = clean_text(row.get("uid"), 120);
        let path_ref = clean_text(row.get("path_ref"), 240);
        let entity_id = clean_text(row.get("entity_id"), 120);
        let key = format!("{kind}|{uid}|{path_ref}|{entity_id}");
        let hash = hash16(
            &serde_json::to_string(&json!({
                "uid": row.get("uid").cloned().unwrap_or(Value::Null),
                "kind": row.get("kind").cloned().unwrap_or(Value::Null),
                "path_ref": row.get("path_ref").cloned().unwrap_or(Value::Null),
                "entity_id": row.get("entity_id").cloned().unwrap_or(Value::Null),
                "tags": row.get("tags").cloned().unwrap_or(Value::Null),
                "summary": row.get("summary").cloned().unwrap_or(Value::Null),
                "status": row.get("status").cloned().unwrap_or(Value::Null),
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        );
        let existing = pointers.get(&key).and_then(Value::as_str).unwrap_or("");
        if existing == hash {
            skipped = skipped.saturating_add(1);
            continue;
        }
        append_jsonl(&path, row)?;
        pointers.insert(key, Value::String(hash));
        emitted = emitted.saturating_add(1);
    }
    let updated = json!({
        "version": "1.0",
        "updated_ts": now_iso(),
        "pointers": Value::Object(pointers.clone()),
    });
    pointer_index_save(root, payload, &updated)?;
    Ok(json!({ "emitted": emitted, "skipped": skipped }))
}

fn project_adaptive_pointers(
    rel_path: &str,
    obj: &Value,
    op: &str,
    meta: &Map<String, Value>,
) -> Vec<Value> {
    let ts = now_iso();
    let path_ref = format!("adaptive/{}", rel_path.replace('\\', "/"));
    let actor = meta_actor(meta);
    let source = meta_source(meta);
    let reason = meta_reason(meta, op);
    let mut rows = Vec::new();

    if rel_path == "sensory/eyes/catalog.json" {
        if let Some(eyes) = obj.get("eyes").and_then(Value::as_array) {
            for eye in eyes {
                let Some(eye_obj) = eye.as_object() else {
                    continue;
                };
                let eye_id = clean_text(eye_obj.get("id"), 64);
                let eye_uid_candidate = clean_text(eye_obj.get("uid"), 64);
                let eye_uid = if is_alnum(&eye_uid_candidate) {
                    eye_uid_candidate
                } else {
                    stable_uid(&format!("adaptive_eye|{eye_id}|v1"), "e", 24)
                };
                let topic_tags = eye_obj
                    .get("topics")
                    .and_then(Value::as_array)
                    .map(|rows| {
                        rows.iter()
                            .map(|row| normalize_tag(&clean_text(Some(row), 32)))
                            .filter(|row| !row.is_empty())
                            .take(8)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let mut tags = vec![
                    "adaptive".to_string(),
                    "sensory".to_string(),
                    "eyes".to_string(),
                ];
                let status_tag = normalize_tag(&clean_text(eye_obj.get("status"), 24));
                if !status_tag.is_empty() {
                    tags.push(status_tag);
                }
                tags.extend(topic_tags);
                tags.sort();
                tags.dedup();
                let entity_id = if eye_id.is_empty() {
                    Value::Null
                } else {
                    Value::String(eye_id.clone())
                };
                let status = {
                    let status = clean_text(eye_obj.get("status"), 24);
                    if status.is_empty() {
                        "active".to_string()
                    } else {
                        status
                    }
                };
                let summary = {
                    let summary =
                        clean_text(eye_obj.get("name").or_else(|| eye_obj.get("id")), 160);
                    if summary.is_empty() {
                        "Adaptive eye".to_string()
                    } else {
                        summary
                    }
                };
                let created_ts = {
                    let created = clean_text(eye_obj.get("created_ts"), 40);
                    if created.is_empty() {
                        ts.clone()
                    } else {
                        created
                    }
                };
                rows.push(json!({
                    "ts": ts,
                    "op": op,
                    "source": "adaptive_layer_store",
                    "source_path": if source.is_empty() { Value::Null } else { Value::String(source.clone()) },
                    "reason": if reason.is_empty() { Value::Null } else { Value::String(reason.clone()) },
                    "actor": actor,
                    "kind": "adaptive_eye",
                    "layer": "sensory",
                    "uid": eye_uid,
                    "entity_id": entity_id,
                    "status": status,
                    "tags": tags,
                    "summary": summary,
                    "path_ref": path_ref,
                    "created_ts": created_ts,
                    "updated_ts": ts,
                }));
            }
            return rows;
        }
    }

    if let Some(obj_map) = obj.as_object() {
        let uid_candidate = clean_text(obj_map.get("uid"), 64);
        let uid = if is_alnum(&uid_candidate) {
            uid_candidate
        } else {
            stable_uid(&format!("adaptive_blob|{rel_path}|v1"), "a", 24)
        };
        let segments = rel_path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        let layer = segments
            .first()
            .map(|segment| normalize_tag(segment))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "adaptive".to_string());
        let kind = format!(
            "adaptive_{}",
            normalize_tag(&segments.join("_"))
                .trim_matches('-')
                .to_string()
        );
        rows.push(json!({
            "ts": ts,
            "op": op,
            "source": "adaptive_layer_store",
            "source_path": if source.is_empty() { Value::Null } else { Value::String(source) },
            "reason": if reason.is_empty() { Value::Null } else { Value::String(reason) },
            "actor": actor,
            "kind": if kind == "adaptive_" { "adaptive_blob".to_string() } else { kind },
            "layer": layer,
            "uid": uid,
            "entity_id": Value::Null,
            "status": "active",
            "tags": ["adaptive", layer],
            "summary": clean_text(Some(&Value::String(format!("Adaptive record: {rel_path}"))), 160),
            "path_ref": path_ref,
            "created_ts": ts,
            "updated_ts": ts,
        }));
    }
    rows
}

fn emit_adaptive_pointers(
    root: &Path,
    payload: &Map<String, Value>,
    rel_path: &str,
    obj: &Value,
    op: &str,
    meta: &Map<String, Value>,
) -> Value {
    let rows = project_adaptive_pointers(rel_path, obj, op, meta);
    append_adaptive_pointer_rows(root, payload, &rows)
        .unwrap_or_else(|_| json!({ "emitted": 0, "skipped": 0 }))
}

fn paths_command(root: &Path, payload: &Map<String, Value>) -> Value {
    let workspace = workspace_root(root, payload);
    let runtime = runtime_root(root, payload);
    json!({
        "ok": true,
        "workspace_root": workspace.to_string_lossy(),
        "runtime_root": runtime.to_string_lossy(),
        "repo_root": runtime.to_string_lossy(),
        "adaptive_root": adaptive_root(root, payload).to_string_lossy(),
        "adaptive_runtime_root": adaptive_runtime_root(root, payload).to_string_lossy(),
        "mutation_log_path": mutation_log_path(root, payload).to_string_lossy(),
        "adaptive_pointers_path": adaptive_pointers_path(root, payload).to_string_lossy(),
        "adaptive_pointer_index_path": adaptive_pointer_index_path(root, payload).to_string_lossy(),
    })
}

fn is_within_root_command(root: &Path, payload: &Map<String, Value>) -> Value {
    let target = clean_text(payload.get("target_path"), 520);
    let within = resolve_adaptive_path(root, payload, &target).is_ok();
    json!({
        "ok": true,
        "target_path": target,
        "within": within,
    })
}

