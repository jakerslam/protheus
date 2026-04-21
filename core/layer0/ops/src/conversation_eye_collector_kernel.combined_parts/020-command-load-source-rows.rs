
fn command_load_source_rows(root: &Path, payload: &Map<String, Value>) -> Value {
    let history_path = resolve_path(root, payload, "history_path", DEFAULT_HISTORY_REL);
    let latest_path = resolve_path(root, payload, "latest_path", DEFAULT_LATEST_REL);
    let max_rows = clamp_u64(payload, "max_rows", 24, 1, 512) as usize;

    let history_rows = read_jsonl_tail(&history_path, max_rows);
    if !history_rows.is_empty() {
        return json!({
            "ok": true,
            "rows": history_rows,
            "source": "history"
        });
    }

    let latest = read_json(&latest_path, Value::Null);
    if latest.is_object() {
        return json!({
            "ok": true,
            "rows": [latest],
            "source": "latest"
        });
    }

    json!({
        "ok": true,
        "rows": [],
        "source": "none"
    })
}

fn command_normalize_topics(payload: &Map<String, Value>) -> Value {
    json!({
        "ok": true,
        "topics": normalize_topics(payload)
    })
}

fn command_load_index(root: &Path, payload: &Map<String, Value>) -> Value {
    let index_path = resolve_index_path(root, payload);
    let index = normalize_index(Some(&read_json(&index_path, Value::Object(Map::new()))));

    json!({
        "ok": true,
        "index_path": index_path.display().to_string(),
        "index": index
    })
}

fn command_apply_node(payload: &Map<String, Value>) -> Value {
    let mut index = normalize_index(payload.get("index"));
    let index_obj = lane_utils::payload_obj(&index).clone();

    let node = match payload.get("node").and_then(Value::as_object) {
        Some(v) => v,
        None => {
            return denied_node(&index, "invalid_node", false);
        }
    };

    let node_id = clean_text(node.get("node_id").and_then(Value::as_str), 120);
    if node_id.is_empty() {
        return denied_node(&index, "missing_node_id", false);
    }

    let now = clean_text(payload.get("now_ts").and_then(Value::as_str), 80);
    let now = if now.is_empty() { now_iso() } else { now };
    let week_key = to_iso_week(clean_text(node.get("ts").and_then(Value::as_str), 80).as_str());
    let level = node
        .get("level")
        .and_then(Value::as_u64)
        .map(|n| n.clamp(1, 3))
        .unwrap_or(3);
    let weekly_limit = clamp_u64(payload, "weekly_node_limit", 10, 1, 50);
    let promotion_overrides = clamp_u64(payload, "weekly_promotion_overrides", 2, 0, 20);

    let emitted = index_obj
        .get("emitted_node_ids")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if emitted.contains_key(&node_id) {
        return denied_node(&index, "duplicate", false);
    }

    let mut weekly_counts = index_obj
        .get("weekly_counts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut weekly_promotions = index_obj
        .get("weekly_promotions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let count = weekly_counts
        .get(&week_key)
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let promotions = weekly_promotions
        .get(&week_key)
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let promoted = count >= weekly_limit && level == 1 && promotions < promotion_overrides;
    let allowed = count < weekly_limit || promoted;
    if !allowed {
        return json!({
            "ok": true,
            "allowed": false,
            "reason": "quota",
            "quota_skipped": true,
            "week_key": week_key,
            "index": index,
        });
    }

    let mut emitted_new = emitted;
    emitted_new.insert(node_id.clone(), Value::String(now.clone()));
    weekly_counts.insert(week_key.clone(), Value::Number((count + 1).into()));
    if promoted {
        weekly_promotions.insert(week_key.clone(), Value::Number((promotions + 1).into()));
    }

    index = normalize_index(Some(&json!({
        "updated_ts": now,
        "emitted_node_ids": emitted_new,
        "weekly_counts": weekly_counts,
        "weekly_promotions": weekly_promotions,
    })));

    let tags = clean_tags(node.get("node_tags"));
    let edges_to = clean_edges(node.get("edges_to"));
    let title = clean_text(node.get("title").and_then(Value::as_str), 180);
    let title = if title.is_empty() {
        "[Conversation Eye] synthesized signal".to_string()
    } else {
        title
    };
    let preview = clean_text(node.get("preview").and_then(Value::as_str), 240);
    let preview = if preview.is_empty() {
        "conversation_eye synthesized runtime node".to_string()
    } else {
        preview
    };
    let date = clean_text(node.get("date").and_then(Value::as_str), 20);
    let date = if date.is_empty() {
        now_iso()[..10].to_string()
    } else {
        date
    };

    let recall_matches = payload
        .get("recall")
        .and_then(Value::as_object)
        .and_then(|rec| rec.get("matches"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_object().cloned())
        .take(3)
        .map(|row| {
            json!({
                "node_id": clean_text(row.get("node_id").and_then(Value::as_str), 120),
                "score": row.get("score").and_then(Value::as_f64).unwrap_or(0.0),
                "shared_tags": row
                    .get("shared_tags")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|tag| tag.as_str().map(|raw| clean_text(Some(raw), 64)))
                    .filter(|tag| !tag.is_empty())

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
