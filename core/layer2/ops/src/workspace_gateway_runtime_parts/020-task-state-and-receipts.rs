fn ensure_task_state(paths: &TaskPaths) -> Result<(), String> {
    fs::create_dir_all(&paths.root).map_err(|err| format!("task_state_mkdir_failed:{err}"))?;
    if !paths.registry_json.exists() {
        let init = TaskRegistry {
            version: "v1".to_string(),
            tasks: Vec::new(),
        };
        write_json_pretty(&paths.registry_json, &init)?;
    }
    if !paths.cancelled_json.exists() {
        write_json_pretty(&paths.cancelled_json, &json!({ "ids": [] }))?;
    }
    if !paths.worker_state_json.exists() {
        write_json_pretty(
            &paths.worker_state_json,
            &json!({
                "type": "task_worker_state",
                "updated_at_ms": now_epoch_ms(),
                "active_workers": {},
                "total_hibernations": 0,
                "last_hibernated": Value::Null
            }),
        )?;
    }
    Ok(())
}

fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("parent_mkdir_failed:{err}"))?;
    }
    let body =
        serde_json::to_vec_pretty(value).map_err(|err| format!("serialize_pretty_failed:{err}"))?;
    fs::write(path, body).map_err(|err| format!("write_failed:{err}"))
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("jsonl_parent_mkdir_failed:{err}"))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("jsonl_open_failed:{err}"))?;
    let mut encoded =
        serde_json::to_vec(row).map_err(|err| format!("jsonl_encode_failed:{err}"))?;
    encoded.push(b'\n');
    file.write_all(&encoded)
        .map_err(|err| format!("jsonl_write_failed:{err}"))
}

fn load_registry(paths: &TaskPaths) -> Result<TaskRegistry, String> {
    ensure_task_state(paths)?;
    let raw = fs::read_to_string(&paths.registry_json)
        .map_err(|err| format!("read_registry_failed:{err}"))?;
    serde_json::from_str::<TaskRegistry>(&raw).map_err(|err| format!("parse_registry_failed:{err}"))
}

fn save_registry(paths: &TaskPaths, registry: &TaskRegistry) -> Result<(), String> {
    write_json_pretty(&paths.registry_json, registry)
}

fn load_cancelled_set(paths: &TaskPaths) -> Result<BTreeSet<String>, String> {
    ensure_task_state(paths)?;
    let raw = fs::read_to_string(&paths.cancelled_json)
        .map_err(|err| format!("read_cancelled_failed:{err}"))?;
    let parsed: Value =
        serde_json::from_str(&raw).map_err(|err| format!("parse_cancelled_failed:{err}"))?;
    let ids = parsed
        .get("ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(ids
        .into_iter()
        .filter_map(|row| row.as_str().map(clean_id))
        .filter(|row| !row.is_empty())
        .collect())
}

fn save_cancelled_set(paths: &TaskPaths, cancelled: &BTreeSet<String>) -> Result<(), String> {
    let rows = cancelled.iter().cloned().collect::<Vec<_>>();
    write_json_pretty(&paths.cancelled_json, &json!({ "ids": rows }))
}

fn find_record_mut<'a>(registry: &'a mut TaskRegistry, id: &str) -> Option<&'a mut TaskRecord> {
    registry.tasks.iter_mut().find(|row| row.id == id)
}

fn upsert_task_record(registry: &mut TaskRegistry, record: TaskRecord) {
    if let Some(existing) = find_record_mut(registry, &record.id) {
        *existing = record;
    } else {
        registry.tasks.push(record);
    }
}

fn sorted_tasks_desc(tasks: &[TaskRecord]) -> Vec<TaskRecord> {
    let mut rows = tasks.to_vec();
    rows.sort_by(|left, right| right.created_at_ms.cmp(&left.created_at_ms));
    rows
}

fn receipt_round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn payload_timestamp_ms(payload: &Value) -> u64 {
    payload
        .get("ts_ms")
        .and_then(Value::as_u64)
        .or_else(|| payload.get("completed_at_ms").and_then(Value::as_u64))
        .or_else(|| payload.get("created_at_ms").and_then(Value::as_u64))
        .unwrap_or_else(now_epoch_ms)
}

fn read_last_receipt_state(path: &Path) -> Option<(String, f64)> {
    let raw = fs::read_to_string(path).ok()?;
    for line in raw.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(row) = serde_json::from_str::<Value>(trimmed) {
            let hash = row
                .get("receipt_hash")
                .and_then(Value::as_str)
                .map(clean_id)
                .unwrap_or_default();
            if hash.is_empty() {
                continue;
            }
            let fidelity = row
                .get("fidelity_score")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            return Some((hash, fidelity.clamp(0.0, 1.0)));
        }
    }
    None
}

fn target_fidelity_for_event(event_type: &str, payload: &Value) -> f64 {
    match event_type {
        "task_progress" => {
            let progress = payload
                .get("progress_percent")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .min(100) as f64;
            receipt_round4((0.85 + (progress / 100.0) * 0.15).clamp(0.0, 1.0))
        }
        "task_result" => {
            let status = payload
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            if status == "done" {
                1.0
            } else if status == "cancelled" {
                0.95
            } else {
                0.8
            }
        }
        _ => 0.9,
    }
}

fn verity_receipt(paths: &TaskPaths, event_type: &str, payload: &Value) -> Value {
    let now = now_epoch_ms();
    let (parent_hash, previous_fidelity) = read_last_receipt_state(&paths.receipts_jsonl)
        .unwrap_or_else(|| ("genesis".to_string(), 1.0));
    let fidelity_score = target_fidelity_for_event(event_type, payload);
    let drift_delta = receipt_round4(fidelity_score - previous_fidelity);
    let timestamp_drift_ms = now.abs_diff(payload_timestamp_ms(payload));
    let mut receipt = json!({
        "type": "task_verity_receipt",
        "event_type": event_type,
        "ts_ms": now,
        "parent_receipt_hash": parent_hash,
        "fidelity_score": fidelity_score,
        "drift_delta": drift_delta,
        "timestamp_drift_ms": timestamp_drift_ms,
        "payload": payload
    });
    receipt["receipt_hash"] = Value::String(deterministic_receipt_hash(&receipt));
    receipt
}

fn emit_event(paths: &TaskPaths, event_type: &str, payload: Value) -> Result<Value, String> {
    let row = json!({
        "type": "task_event",
        "event_type": event_type,
        "ts_ms": now_epoch_ms(),
        "payload": payload
    });
    append_jsonl(&paths.events_jsonl, &row)?;
    Ok(row)
}

fn emit_conduit_update(paths: &TaskPaths, update: &ProgressUpdate) -> Result<(), String> {
    let payload = json!({
        "type": "conduit_message",
        "origin": "task-runtime",
        "task_id": update.id,
        "progress_percent": update.progress_percent,
        "message": update.message,
        "step": update.step,
        "total_steps": update.total_steps,
        "ts_ms": update.ts_ms
    });
    append_jsonl(&paths.conduit_jsonl, &payload)?;
    let receipt = verity_receipt(paths, "task_progress", &payload);
    append_jsonl(&paths.receipts_jsonl, &receipt)?;
    println!("{}", payload);
    println!("{}", receipt);
    Ok(())
}

fn emit_final_result(paths: &TaskPaths, result: &TaskResult) -> Result<(), String> {
    let payload = json!({
        "type": "conduit_message",
        "origin": "task-runtime",
        "task_id": result.id,
        "status": result.status,
        "summary": result.summary,
        "completed_at_ms": result.completed_at_ms,
        "duration_ms": result.duration_ms
    });
    append_jsonl(&paths.conduit_jsonl, &payload)?;
    let receipt = verity_receipt(paths, "task_result", &payload);
    append_jsonl(&paths.receipts_jsonl, &receipt)?;
    println!("{}", payload);
    println!("{}", receipt);
    Ok(())
}

fn load_worker_state(paths: &TaskPaths) -> Result<Value, String> {
    ensure_task_state(paths)?;
    let raw = fs::read_to_string(&paths.worker_state_json)
        .map_err(|err| format!("read_worker_state_failed:{err}"))?;
    serde_json::from_str::<Value>(&raw).map_err(|err| format!("parse_worker_state_failed:{err}"))
}

fn save_worker_state(paths: &TaskPaths, state: &Value) -> Result<(), String> {
    write_json_pretty(&paths.worker_state_json, state)
}

fn mark_worker_started(
    paths: &TaskPaths,
    worker_id: &str,
    bus_mode: &str,
    service_mode: bool,
) -> Result<(), String> {
    let mut state = load_worker_state(paths)?;
    if !state
        .get("active_workers")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["active_workers"] = json!({});
    }
    let now = now_epoch_ms();
    state["active_workers"][worker_id] = json!({
        "worker_id": worker_id,
        "bus_mode": bus_mode,
        "service_mode": service_mode,
        "started_at_ms": now,
        "last_poll_ms": now,
        "poll_wait_ms": 0,
        "queue_empty": false
    });
    state["last_event"] = json!({
        "type": "worker_started",
        "worker_id": worker_id,
        "ts_ms": now
    });
    state["updated_at_ms"] = json!(now);
    save_worker_state(paths, &state)
}

fn mark_worker_poll(
    paths: &TaskPaths,
    worker_id: &str,
    queue_empty: bool,
    poll_wait_ms: u64,
) -> Result<(), String> {
    let mut state = load_worker_state(paths)?;
    let now = now_epoch_ms();
    if state
        .get("active_workers")
        .and_then(Value::as_object)
        .is_none()
    {
        state["active_workers"] = json!({});
    }
    state["active_workers"][worker_id]["last_poll_ms"] = json!(now);
    state["active_workers"][worker_id]["queue_empty"] = json!(queue_empty);
    state["active_workers"][worker_id]["poll_wait_ms"] = json!(poll_wait_ms);
    state["updated_at_ms"] = json!(now);
    save_worker_state(paths, &state)
}

fn mark_worker_hibernated(
    paths: &TaskPaths,
    worker_id: &str,
    idle_ms: u64,
    processed: usize,
) -> Result<(), String> {
    let mut state = load_worker_state(paths)?;
    let now = now_epoch_ms();
    if let Some(active) = state
        .get_mut("active_workers")
        .and_then(Value::as_object_mut)
    {
        active.remove(worker_id);
    } else {
        state["active_workers"] = json!({});
    }
    let next_total = state
        .get("total_hibernations")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(1);
    state["total_hibernations"] = json!(next_total);
    state["last_hibernated"] = json!({
        "worker_id": worker_id,
        "idle_ms": idle_ms,
        "processed": processed,
        "ts_ms": now
    });
    state["last_event"] = json!({
        "type": "worker_hibernated",
        "worker_id": worker_id,
        "idle_ms": idle_ms,
        "processed": processed,
        "ts_ms": now
    });
    state["updated_at_ms"] = json!(now);
    save_worker_state(paths, &state)
}

fn mark_worker_stopped(paths: &TaskPaths, worker_id: &str, processed: usize) -> Result<(), String> {
    let mut state = load_worker_state(paths)?;
    let now = now_epoch_ms();
    if let Some(active) = state
        .get_mut("active_workers")
        .and_then(Value::as_object_mut)
    {
        active.remove(worker_id);
    } else {
        state["active_workers"] = json!({});
    }
    state["last_event"] = json!({
        "type": "worker_stopped",
        "worker_id": worker_id,
        "processed": processed,
        "ts_ms": now
    });
    state["updated_at_ms"] = json!(now);
    save_worker_state(paths, &state)
}

fn read_queue_rows(path: &Path) -> Result<Vec<TaskPayload>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path).map_err(|err| format!("queue_open_failed:{err}"))?;
    let reader = BufReader::new(file);
    let mut rows = Vec::<TaskPayload>::new();
    for line in reader.lines() {
        let raw = line.map_err(|err| format!("queue_read_line_failed:{err}"))?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(payload) = serde_json::from_str::<TaskPayload>(trimmed) {
            rows.push(payload);
        }
    }
    Ok(rows)
}

fn write_queue_rows(path: &Path, rows: &[TaskPayload]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("queue_mkdir_failed:{err}"))?;
    }
    let mut file = fs::File::create(path).map_err(|err| format!("queue_create_failed:{err}"))?;
    for row in rows {
        let mut encoded =
            serde_json::to_vec(row).map_err(|err| format!("queue_encode_failed:{err}"))?;
        encoded.push(b'\n');
        file.write_all(&encoded)
            .map_err(|err| format!("queue_write_failed:{err}"))?;
    }
    Ok(())
}
