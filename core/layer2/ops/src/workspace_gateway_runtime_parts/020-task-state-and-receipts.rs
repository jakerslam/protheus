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

fn verity_receipt(event_type: &str, payload: &Value) -> Value {
    // TEMPORARY SCAFFOLDING — NATS JetStream. To be replaced with native InfRing task ions built from baryons later.
    let mut receipt = json!({
        "type": "task_verity_receipt",
        "event_type": event_type,
        "ts_ms": now_epoch_ms(),
        "fidelity_score": 1.0,
        "drift_delta": 0.0,
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
    let receipt = verity_receipt("task_progress", &payload);
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
    let receipt = verity_receipt("task_result", &payload);
    append_jsonl(&paths.receipts_jsonl, &receipt)?;
    println!("{}", payload);
    println!("{}", receipt);
    Ok(())
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
