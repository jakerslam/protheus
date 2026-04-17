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

    if tag_rows.is_empty() {
        return json!({
            "ok": false,
            "type": "tag_memory_matrix",
            "reason": "empty_tag_projection",
            "index_path": index_path.to_string_lossy().to_string(),
            "matrix_path": json_path.to_string_lossy().to_string(),
            "markdown_path": md_path.to_string_lossy().to_string(),
            "generated_at": now_iso()
        });
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
            if let Err(err) = fs::create_dir_all(parent) {
                return json!({
                    "ok": false,
                    "type": "tag_memory_matrix",
                    "reason": format!("markdown_parent_create_failed:{err}"),
                    "index_path": index_path.to_string_lossy().to_string(),
                    "matrix_path": json_path.to_string_lossy().to_string(),
                    "markdown_path": md_path.to_string_lossy().to_string(),
                    "generated_at": now_iso()
                });
            }
        }
        if let Err(err) = fs::write(&md_path, format!("{}\n", matrix_markdown(&payload))) {
            return json!({
                "ok": false,
                "type": "tag_memory_matrix",
                "reason": format!("markdown_write_failed:{err}"),
                "index_path": index_path.to_string_lossy().to_string(),
                "matrix_path": json_path.to_string_lossy().to_string(),
                "markdown_path": md_path.to_string_lossy().to_string(),
                "generated_at": now_iso()
            });
        }
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
