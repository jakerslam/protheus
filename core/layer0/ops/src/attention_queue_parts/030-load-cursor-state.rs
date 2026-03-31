fn load_cursor_state(path: &Path) -> Value {
    let mut state = read_json(path).unwrap_or_else(|| json!({}));
    if !state.is_object() {
        state = json!({});
    }
    if !state
        .get("consumers")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["consumers"] = json!({});
    }
    state["schema_id"] = Value::String("attention_queue_cursor_state".to_string());
    state["schema_version"] = Value::String("1.0".to_string());
    state
}

fn persist_cursor_state(path: &Path, state: &Value) {
    write_json(path, state);
}

fn read_consumer_offset(state: &Value, consumer_id: &str) -> usize {
    state
        .pointer(&format!("/consumers/{consumer_id}/offset"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn write_consumer_offset(
    state: &mut Value,
    consumer_id: &str,
    offset: usize,
    last_token: Option<&str>,
    run_context: &str,
) {
    if !state
        .get("consumers")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["consumers"] = json!({});
    }
    state["updated_at"] = Value::String(now_iso());
    state["consumers"][consumer_id] = json!({
        "offset": offset,
        "acked_at": now_iso(),
        "last_cursor_token": last_token,
        "run_context": run_context
    });
}

fn cursor_token_for_event(
    contract: &AttentionContract,
    consumer_id: &str,
    index: usize,
    event: &Value,
) -> String {
    let seed = json!({
        "type": "attention_cursor_token",
        "consumer_id": consumer_id,
        "index": index,
        "queue_path": contract.queue_path.to_string_lossy().to_string(),
        "event_receipt_hash": event.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "attention_key": event.get("attention_key").cloned().unwrap_or(Value::Null),
        "event_ts": event.get("ts").cloned().unwrap_or(Value::Null)
    });
    deterministic_receipt_hash(&seed)
}

fn load_active_queue(contract: &AttentionContract) -> (Vec<Value>, usize) {
    if !(contract.enabled && contract.push_attention_queue) {
        return (Vec::new(), 0);
    }
    let rows = read_jsonl(&contract.queue_path);
    let (mut active, expired_pruned) = prune_expired(rows);
    sort_active_rows_with_authority(&mut active);
    if expired_pruned > 0 {
        write_jsonl(&contract.queue_path, &active);
    }
    (active, expired_pruned)
}

fn next(root: &Path, flags: &BTreeMap<String, String>, auto_ack: bool) -> i32 {
    let contract = load_contract(root);
    let run_context = flags.get("run-context").cloned().unwrap_or_else(|| {
        if auto_ack {
            "drain".to_string()
        } else {
            "next".to_string()
        }
    });
    let consumer_id = normalize_consumer_id(
        flags
            .get("consumer")
            .map(String::as_str)
            .or_else(|| flags.get("consumer-id").map(String::as_str)),
    );
    if consumer_id.is_empty() {
        let mut out = json!({
            "ok": false,
            "type": if auto_ack { "attention_queue_drain_error" } else { "attention_queue_next_error" },
            "ts": now_iso(),
            "reason": "consumer_missing_or_invalid",
            "run_context": run_context,
            "attention_contract": contract_snapshot(&contract)
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        emit(&out);
        return 2;
    }
    let limit = parse_limit(
        flags.get("limit").or_else(|| flags.get("max-events")),
        1,
        contract.max_batch_size,
    );
    let wait_ms = parse_wait_ms(
        flags.get("wait-ms").or_else(|| flags.get("wait_ms")),
        0,
        300_000,
    );
    let wait_started_ms = Utc::now().timestamp_millis();
    let (active_rows, expired_pruned) = loop {
        let (rows, pruned) = load_active_queue(&contract);
        if wait_ms == 0 || !rows.is_empty() {
            break (rows, pruned);
        }
        let elapsed_ms = Utc::now()
            .timestamp_millis()
            .saturating_sub(wait_started_ms)
            .max(0) as u64;
        if elapsed_ms >= wait_ms {
            break (rows, pruned);
        }
        let remaining = wait_ms.saturating_sub(elapsed_ms);
        let sleep_ms = remaining.clamp(25, 250);
        let wait_tick = crossbeam_channel::after(Duration::from_millis(sleep_ms));
        let _ = wait_tick.recv();
    };
    let waited_ms = Utc::now()
        .timestamp_millis()
        .saturating_sub(wait_started_ms)
        .max(0) as u64;

    let mut cursor_state = load_cursor_state(&contract.cursor_state_path);
    let mut cursor_offset = read_consumer_offset(&cursor_state, &consumer_id);
    if cursor_offset > active_rows.len() {
        cursor_offset = active_rows.len();
    }
    let end = active_rows.len().min(cursor_offset.saturating_add(limit));
    let mut events = Vec::new();
    for (idx, event) in active_rows
        .iter()
        .enumerate()
        .skip(cursor_offset)
        .take(end.saturating_sub(cursor_offset))
    {
        events.push(json!({
            "cursor_index": idx,
            "cursor_token": cursor_token_for_event(&contract, &consumer_id, idx, event),
            "event": event
        }));
    }

    let mut acked_through_index = Value::Null;
    if auto_ack && !events.is_empty() {
        let through_index = end.saturating_sub(1);
        let last_token = events
            .last()
            .and_then(|row| row.get("cursor_token"))
            .and_then(Value::as_str);
        write_consumer_offset(
            &mut cursor_state,
            &consumer_id,
            through_index.saturating_add(1),
            last_token,
            &run_context,
        );
        persist_cursor_state(&contract.cursor_state_path, &cursor_state);
        acked_through_index = Value::Number((through_index as u64).into());
    }

    let cursor_after = if auto_ack { end } else { cursor_offset };
    let mut batch_lane_counts: BTreeMap<String, usize> = BTreeMap::new();
    for row in &events {
        let lane = row
            .pointer("/event/queue_lane")
            .and_then(Value::as_str)
            .unwrap_or("standard")
            .trim()
            .to_ascii_lowercase();
        let key = if lane == "critical" || lane == "background" {
            lane
        } else {
            "standard".to_string()
        };
        *batch_lane_counts.entry(key).or_insert(0) += 1;
    }
    let mut out = json!({
        "ok": true,
        "type": if auto_ack { "attention_queue_drain" } else { "attention_queue_next" },
        "ts": now_iso(),
        "run_context": run_context,
        "consumer_id": consumer_id,
        "limit": limit,
        "wait_ms": wait_ms,
        "waited_ms": waited_ms,
        "queue_depth": active_rows.len(),
        "expired_pruned": expired_pruned,
        "cursor_offset": cursor_offset,
        "cursor_offset_after": cursor_after,
        "batch_count": events.len(),
        "batch_lane_counts": batch_lane_counts,
        "acked": auto_ack && !events.is_empty(),
        "acked_through_index": acked_through_index,
        "events": events,
        "attention_contract": contract_snapshot(&contract)
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    append_jsonl(
        &contract.receipts_path,
        &json!({
            "ts": now_iso(),
            "type": if auto_ack { "attention_consumer_drain" } else { "attention_consumer_next" },
            "consumer_id": out.get("consumer_id").cloned().unwrap_or(Value::Null),
            "batch_count": out.get("batch_count").cloned().unwrap_or(Value::Number(0.into())),
            "cursor_offset": out.get("cursor_offset").cloned().unwrap_or(Value::Number(0.into())),
            "cursor_offset_after": out.get("cursor_offset_after").cloned().unwrap_or(Value::Number(0.into())),
            "run_context": out.get("run_context").cloned().unwrap_or(Value::String("unknown".to_string())),
            "receipt_hash": out.get("receipt_hash").cloned().unwrap_or(Value::String("".to_string()))
        }),
    );
    emit(&out);
    0
}

fn ack(root: &Path, flags: &BTreeMap<String, String>) -> i32 {
    let contract = load_contract(root);
    let run_context = flags
        .get("run-context")
        .cloned()
        .unwrap_or_else(|| "ack".to_string());
    let consumer_id = normalize_consumer_id(
        flags
            .get("consumer")
            .map(String::as_str)
            .or_else(|| flags.get("consumer-id").map(String::as_str)),
    );
    if consumer_id.is_empty() {
        let mut out = json!({
            "ok": false,
            "type": "attention_queue_ack_error",
            "ts": now_iso(),
            "reason": "consumer_missing_or_invalid",
            "run_context": run_context,
            "attention_contract": contract_snapshot(&contract)
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        emit(&out);
        return 2;
    }
    let through_index = parse_through_index(
        flags
            .get("through-index")
            .or_else(|| flags.get("through_index"))
            .or_else(|| flags.get("index")),
    );
    let Some(through_index) = through_index else {
        let mut out = json!({
            "ok": false,
            "type": "attention_queue_ack_error",
            "ts": now_iso(),
            "reason": "through_index_missing_or_invalid",
            "run_context": run_context,
            "attention_contract": contract_snapshot(&contract)
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        emit(&out);
        return 2;
    };
    let cursor_token = clean_text(flags.get("cursor-token").map(String::as_str), 200);
    if cursor_token.is_empty() {
        let mut out = json!({
            "ok": false,
            "type": "attention_queue_ack_error",
            "ts": now_iso(),
            "reason": "cursor_token_missing",
            "run_context": run_context,
            "attention_contract": contract_snapshot(&contract)
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        emit(&out);
        return 2;
    }

    let (active_rows, expired_pruned) = load_active_queue(&contract);
    if through_index >= active_rows.len() {
        let mut out = json!({
            "ok": false,
            "type": "attention_queue_ack_error",
            "ts": now_iso(),
            "reason": "through_index_out_of_range",
            "run_context": run_context,
            "consumer_id": consumer_id,
            "through_index": through_index,
            "queue_depth": active_rows.len(),
            "expired_pruned": expired_pruned,
            "attention_contract": contract_snapshot(&contract)
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        emit(&out);
        return 2;
    }

    let mut cursor_state = load_cursor_state(&contract.cursor_state_path);
    let old_offset = read_consumer_offset(&cursor_state, &consumer_id).min(active_rows.len());
    if through_index.saturating_add(1) < old_offset {
        let mut out = json!({
            "ok": false,
            "type": "attention_queue_ack_error",
            "ts": now_iso(),
            "reason": "ack_before_cursor_offset",
            "run_context": run_context,
            "consumer_id": consumer_id,
            "through_index": through_index,
            "cursor_offset": old_offset,
            "attention_contract": contract_snapshot(&contract)
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        emit(&out);
        return 2;
    }

    let expected_token = cursor_token_for_event(
        &contract,
        &consumer_id,
        through_index,
        &active_rows[through_index],
    );
    if expected_token != cursor_token {
        let mut out = json!({
            "ok": false,
            "type": "attention_queue_ack_error",
            "ts": now_iso(),
            "reason": "cursor_token_mismatch",
            "run_context": run_context,
            "consumer_id": consumer_id,
            "through_index": through_index,
            "attention_contract": contract_snapshot(&contract)
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        emit(&out);
        return 2;
    }

    let next_offset = through_index.saturating_add(1);
    write_consumer_offset(
        &mut cursor_state,
        &consumer_id,
        next_offset,
        Some(&cursor_token),
        &run_context,
    );
    persist_cursor_state(&contract.cursor_state_path, &cursor_state);

    let mut out = json!({
        "ok": true,
        "type": "attention_queue_ack",
        "ts": now_iso(),
        "run_context": run_context,
        "consumer_id": consumer_id,
        "through_index": through_index,
        "cursor_offset_before": old_offset,
        "cursor_offset_after": next_offset,
        "acked_count": next_offset.saturating_sub(old_offset),
        "queue_depth": active_rows.len(),
        "expired_pruned": expired_pruned,
        "attention_contract": contract_snapshot(&contract)
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    append_jsonl(
        &contract.receipts_path,
        &json!({
            "ts": now_iso(),
            "type": "attention_consumer_ack",
            "consumer_id": out.get("consumer_id").cloned().unwrap_or(Value::Null),
            "through_index": out.get("through_index").cloned().unwrap_or(Value::Null),
            "cursor_offset_before": out.get("cursor_offset_before").cloned().unwrap_or(Value::Null),
            "cursor_offset_after": out.get("cursor_offset_after").cloned().unwrap_or(Value::Null),
            "run_context": out.get("run_context").cloned().unwrap_or(Value::String("unknown".to_string())),
            "receipt_hash": out.get("receipt_hash").cloned().unwrap_or(Value::String("".to_string()))
        }),
    );
    emit(&out);
    0
}

