
fn command_process_nodes(payload: &Map<String, Value>) -> Value {
    let mut index = normalize_index(payload.get("index"));
    let topics = payload
        .get("topics")
        .cloned()
        .unwrap_or_else(|| Value::Array(normalize_topics(payload)));
    let weekly_node_limit = clamp_u64(payload, "weekly_node_limit", 10, 1, 50);
    let weekly_promotion_overrides = clamp_u64(payload, "weekly_promotion_overrides", 2, 0, 20);
    let max_items = clamp_u64(payload, "max_items", 3, 1, 64) as usize;
    let now_ts = clean_text(payload.get("now_ts").and_then(Value::as_str), 80);
    let now_ts = if now_ts.is_empty() { now_iso() } else { now_ts };
    let candidates = payload
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut memory_rows = Vec::<Value>::new();
    let mut items = Vec::<Value>::new();
    let mut node_writes = 0_u64;
    let mut recall_queued = 0_u64;
    let mut recall_matches = 0_u64;
    let mut quota_skipped = 0_u64;

    for candidate in candidates {
        if items.len() >= max_items {
            break;
        }
        let candidate_obj = match candidate.as_object() {
            Some(v) => v,
            None => continue,
        };
        let node = match candidate_obj.get("node") {
            Some(v) if v.is_object() => v.clone(),
            _ => continue,
        };

        let mut apply_payload = Map::new();
        apply_payload.insert("index".to_string(), index.clone());
        apply_payload.insert("node".to_string(), node);
        apply_payload.insert("topics".to_string(), topics.clone());
        apply_payload.insert(
            "weekly_node_limit".to_string(),
            Value::Number(weekly_node_limit.into()),
        );
        apply_payload.insert(
            "weekly_promotion_overrides".to_string(),
            Value::Number(weekly_promotion_overrides.into()),
        );
        apply_payload.insert("now_ts".to_string(), Value::String(now_ts.clone()));
        if let Some(recall) = candidate_obj.get("recall") {
            apply_payload.insert("recall".to_string(), recall.clone());
        }

        let applied = command_apply_node(&apply_payload);
        index = normalize_index(applied.get("index"));

        if applied
            .get("allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            if let Some(row) = applied.get("memory_row") {
                memory_rows.push(row.clone());
                node_writes = node_writes.saturating_add(1);
            }
            if let Some(item) = applied.get("collect_item") {
                items.push(item.clone());
            }

            if let Some(recall_obj) = candidate_obj.get("recall").and_then(Value::as_object) {
                let matches_len = recall_obj
                    .get("matches")
                    .and_then(Value::as_array)
                    .map(|rows| rows.len() as u64)
                    .unwrap_or(0);
                recall_matches = recall_matches.saturating_add(matches_len);
                if recall_obj
                    .get("attention")
                    .and_then(Value::as_object)
                    .and_then(|obj| obj.get("queued"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    recall_queued = recall_queued.saturating_add(1);
                }
            }
        } else if applied
            .get("quota_skipped")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            quota_skipped = quota_skipped.saturating_add(1);
        }
    }

    json!({
        "ok": true,
        "index": index,
        "memory_rows": memory_rows,
        "items": items,
        "node_writes": node_writes,
        "recall_queued": recall_queued,
        "recall_matches": recall_matches,
        "quota_skipped": quota_skipped,
    })
}

fn command_append_memory_row(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let jsonl_path = resolve_memory_jsonl_path(root, payload, "jsonl_path");
    let row = payload
        .get("row")
        .cloned()
        .unwrap_or(Value::Object(Map::new()));
    append_jsonl(&jsonl_path, &row)?;
    Ok(json!({
        "ok": true,
        "jsonl_path": jsonl_path.display().to_string()
    }))
}

fn command_append_memory_rows(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let jsonl_path = resolve_memory_jsonl_path(root, payload, "jsonl_path");
    let rows = payload
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut appended = 0_u64;
    for row in rows {
        append_jsonl(&jsonl_path, &row)?;
        appended = appended.saturating_add(1);
    }
    Ok(json!({
        "ok": true,
        "jsonl_path": jsonl_path.display().to_string(),
        "appended": appended
    }))
}

fn command_save_index(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let index_path = resolve_index_path(root, payload);
    let index = normalize_index(payload.get("index"));
    write_json_atomic(&index_path, &index)?;
    Ok(json!({
        "ok": true,
        "index_path": index_path.display().to_string(),
        "index": index
    }))
}

fn dispatch(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "begin-collection" => Ok(command_begin_collection(root, payload)),
        "preflight" => Ok(command_preflight(root, payload)),
        "load-source-rows" => Ok(command_load_source_rows(root, payload)),
        "normalize-topics" => Ok(command_normalize_topics(payload)),
        "load-index" => Ok(command_load_index(root, payload)),
        "apply-node" => Ok(command_apply_node(payload)),
        "process-nodes" => Ok(command_process_nodes(payload)),
        "append-memory-row" => command_append_memory_row(root, payload),
        "append-memory-rows" => command_append_memory_rows(root, payload),
        "save-index" => command_save_index(root, payload),
        _ => Err("conversation_eye_collector_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "conversation_eye_collector_kernel") {
        Ok(v) => v,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "conversation_eye_collector_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);

    match dispatch(root, &command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt(
                "conversation_eye_collector_kernel",
                out,
            ));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "conversation_eye_collector_kernel_error",
                &err,
            ));
            1
        }
    }
}
