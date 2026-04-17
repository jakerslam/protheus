                    .take(8)
                    .map(Value::String)
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();

    let recall_queued = payload
        .get("recall")
        .and_then(Value::as_object)
        .and_then(|rec| rec.get("attention"))
        .and_then(Value::as_object)
        .and_then(|attn| attn.get("queued"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let memory_row = json!({
        "ts": now_iso(),
        "source": "conversation_eye",
        "node_id": node_id,
        "hex_id": clean_text(node.get("hex_id").and_then(Value::as_str), 24),
        "node_kind": clean_text(node.get("node_kind").and_then(Value::as_str), 32),
        "level": level,
        "level_token": clean_text(node.get("level_token").and_then(Value::as_str), 16),
        "tags": tags,
        "edges_to": edges_to,
        "title": title,
        "preview": preview,
        "xml": clean_text(node.get("xml").and_then(Value::as_str), 1600),
    });

    let collect_item = json!({
        "collected_at": now_iso(),
        "id": sha16(&format!("{}|{}", clean_text(Some(&node_id), 80), title)),
        "url": format!("https://local.workspace/conversation/{}/{}", date, clean_text(Some(&node_id), 80)),
        "title": title,
        "content_preview": preview,
        "topics": payload.get("topics").cloned().unwrap_or_else(|| Value::Array(normalize_topics(payload))),
        "node_id": clean_text(Some(&node_id), 80),
        "node_hex_id": clean_text(node.get("hex_id").and_then(Value::as_str), 24),
        "node_kind": clean_text(node.get("node_kind").and_then(Value::as_str), 32),
        "node_level": level,
        "node_level_token": clean_text(node.get("level_token").and_then(Value::as_str), 16),
        "node_tags": memory_row.get("tags").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "edges_to": memory_row.get("edges_to").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "recall_matches": recall_matches,
        "recall_queued": recall_queued,
        "bytes": std::cmp::min(8192_usize, title.len() + preview.len() + 160)
    });

    json!({
        "ok": true,
        "allowed": true,
        "promoted": promoted,
        "quota_skipped": false,
        "week_key": week_key,
        "index": index,
        "memory_row": memory_row,
        "collect_item": collect_item,
    })
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn normalize_topics_includes_defaults() {
        let payload = json!({"topics": ["alpha", "decision"]});
        let out = command_normalize_topics(lane_utils::payload_obj(&payload));
        let topics = out
            .get("topics")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(topics
            .iter()
            .any(|row| row.as_str() == Some("conversation")));
        assert!(topics.iter().any(|row| row.as_str() == Some("alpha")));
        assert_eq!(
            topics,
            vec![
                json!("conversation"),
                json!("decision"),
                json!("insight"),
                json!("directive"),
                json!("t1"),
                json!("alpha"),
            ]
        );
    }

    #[test]
    fn process_nodes_dedupes_edges_and_preserves_topic_order() {
        let payload = json!({
            "index": { "emitted_node_ids": {} },
            "topics": ["conversation", "decision", "insight", "directive", "t1", "browser", "fetch"],
            "max_items": 1,
            "candidates": [
                {
                    "node": {
                        "node_id": "n1",
                        "ts": "2026-01-01T00:00:00Z",
                        "title": "First node",
                        "preview": "Collected from the web",
                        "level": 3,
                        "node_tags": ["collector", "collector", "web"],
                        "edges_to": ["alpha", "alpha", "beta"]
                    }
                }
            ]
        });
        let out = command_process_nodes(lane_utils::payload_obj(&payload));
        let items = out
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(items.len(), 1);
        let item = items[0].as_object().cloned().unwrap_or_default();
        assert_eq!(
            item.get("topics").cloned().unwrap_or(Value::Null),
            json!(["conversation", "decision", "insight", "directive", "t1", "browser", "fetch"])
        );
        assert_eq!(
            item.get("edges_to").cloned().unwrap_or(Value::Null),
            json!(["alpha", "beta"])
        );
    }

    #[test]
    fn apply_node_rejects_duplicate() {
        let payload = json!({
            "index": { "emitted_node_ids": {"abc": "2026-01-01T00:00:00Z"} },
            "node": {"node_id": "abc", "ts": "2026-01-01T00:00:00Z", "level": 3}
        });
        let out = command_apply_node(lane_utils::payload_obj(&payload));
        assert_eq!(out.get("allowed").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("reason").and_then(Value::as_str), Some("duplicate"));
    }

    #[test]
    fn process_nodes_batches_and_limits_items() {
        let payload = json!({
            "index": { "emitted_node_ids": {} },
            "topics": ["conversation", "decision"],
            "max_items": 1,
            "candidates": [
                {
                    "node": { "node_id": "n1", "title": "one", "preview":"p1", "ts":"2026-03-27T00:00:00Z", "level": 3 },
                    "recall": { "matches":[{"node_id":"x"}], "attention": {"queued": true} }
                },
                {
                    "node": { "node_id": "n2", "title": "two", "preview":"p2", "ts":"2026-03-27T00:00:01Z", "level": 3 },
                    "recall": { "matches":[{"node_id":"y"}], "attention": {"queued": true} }
                }
            ]
        });
        let out = command_process_nodes(lane_utils::payload_obj(&payload));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(out.get("node_writes").and_then(Value::as_u64), Some(1));
    }

    #[test]
    fn begin_collection_hydrates_runtime_payload() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        let history_path = root.join(DEFAULT_HISTORY_REL);
        if let Some(parent) = history_path.parent() {
            fs::create_dir_all(parent).expect("mkdir history parent");
        }
        fs::write(
            &history_path,
            "{\"id\":\"row1\",\"text\":\"hello from history\"}\n",
        )
        .expect("write history jsonl");

        let out = command_begin_collection(
            root,
            lane_utils::payload_obj(&json!({
                "budgets": { "max_items": 3 }
            })),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("success").and_then(Value::as_bool), Some(true));
        assert!(out
            .get("source_rows")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("topics")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|r| r.as_str() == Some("conversation")))
            .unwrap_or(false));
    }
}
