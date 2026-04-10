fn print_task_error(kind: &str, err: impl Into<Value>) -> i32 {
    eprintln!(
        "{}",
        json!({
            "ok": false,
            "type": kind,
            "error": err.into()
        })
    );
    1
}

fn parse_ticket_id(parsed: &ParsedCli) -> Option<String> {
    parse_non_empty(&parsed.flags, "ticket")
        .or_else(|| parse_non_empty(&parsed.flags, "ticket-id"))
        .or_else(|| parsed.positional.first().cloned())
        .map(|raw| clean_id(raw.as_str()))
        .filter(|value| !value.is_empty())
}

fn create_payload_from_flags(parsed: &ParsedCli) -> TaskPayload {
    let id = Uuid::new_v4().to_string();
    let kind = parse_non_empty(&parsed.flags, "kind").unwrap_or_else(|| "analysis".to_string());
    let estimated_seconds = parse_u64_flag(&parsed.flags, "estimated-seconds", 30);
    let steps = parse_u64_flag(&parsed.flags, "steps", (estimated_seconds / 5).max(1));
    let mut metadata = BTreeMap::<String, String>::new();
    for (key, value) in parsed.flags.iter() {
        if key.starts_with("meta-") {
            metadata.insert(key.clone(), value.clone());
        }
    }
    TaskPayload {
        id,
        kind,
        requested_by: "infring_cli".to_string(),
        estimated_seconds,
        steps: steps.max(1),
        created_at_ms: now_epoch_ms(),
        metadata,
    }
}

fn submit_task(root: &Path, parsed: &ParsedCli) -> i32 {
    let paths = task_paths(root);
    if let Err(err) = ensure_task_state(&paths) {
        return print_task_error("task_submit_error", err);
    }
    let (bus, notes) = build_task_bus(root, &parsed.flags);
    let payload = create_payload_from_flags(parsed);
    let mut registry = match load_registry(&paths) {
        Ok(value) => value,
        Err(err) => return print_task_error("task_submit_error", err),
    };
    let now = now_epoch_ms();
    let record = TaskRecord {
        id: payload.id.clone(),
        kind: payload.kind.clone(),
        status: "queued".to_string(),
        bus_mode: bus.mode().to_string(),
        progress_percent: 0,
        estimated_seconds: payload.estimated_seconds,
        created_at_ms: now,
        updated_at_ms: now,
        cancelled: false,
        result: None,
    };
    upsert_task_record(&mut registry, record);
    if let Err(err) = save_registry(&paths, &registry) {
        return print_task_error("task_submit_error", err);
    }
    if let Err(err) = bus.enqueue(&payload) {
        return print_task_error("task_submit_error", err);
    }
    let ticket = TaskTicket {
        id: payload.id.clone(),
        status: "queued".to_string(),
        estimated_seconds: payload.estimated_seconds,
        bus_mode: bus.mode().to_string(),
    };
    let _ = emit_event(
        &paths,
        "task_submit",
        json!({
            "ticket": ticket,
            "notes": notes
        }),
    );
    println!(
        "{}",
        json!({
            "ok": true,
            "type": "task_ticket",
            "ticket": ticket,
            "notes": notes
        })
    );
    0
}

fn list_tasks(root: &Path, parsed: &ParsedCli) -> i32 {
    let paths = task_paths(root);
    let registry = match load_registry(&paths) {
        Ok(value) => value,
        Err(err) => return print_task_error("task_list_error", err),
    };
    let limit = parse_u64_flag(&parsed.flags, "limit", 100) as usize;
    let tasks = sorted_tasks_desc(&registry.tasks)
        .into_iter()
        .take(limit)
        .collect::<Vec<_>>();
    println!(
        "{}",
        json!({
            "ok": true,
            "type": "task_list",
            "count": tasks.len(),
            "tasks": tasks
        })
    );
    0
}

fn status_task(root: &Path, parsed: &ParsedCli) -> i32 {
    let paths = task_paths(root);
    let registry = match load_registry(&paths) {
        Ok(value) => value,
        Err(err) => return print_task_error("task_status_error", err),
    };
    let ticket = parse_ticket_id(parsed);
    if let Some(ticket_id) = ticket {
        let found = registry.tasks.into_iter().find(|row| row.id == ticket_id);
        match found {
            Some(task) => {
                println!(
                    "{}",
                    json!({ "ok": true, "type": "task_status", "task": task })
                );
                0
            }
            None => {
                eprintln!(
                    "{}",
                    json!({
                        "ok": false,
                        "type": "task_status_error",
                        "error": "task_not_found",
                        "ticket_id": ticket_id
                    })
                );
                1
            }
        }
    } else {
        let mut queued = 0u64;
        let mut running = 0u64;
        let mut done = 0u64;
        let mut cancelled = 0u64;
        for task in registry.tasks {
            match task.status.as_str() {
                "queued" => queued += 1,
                "running" => running += 1,
                "done" => done += 1,
                "cancelled" => cancelled += 1,
                _ => {}
            }
        }
        println!(
            "{}",
            json!({
                "ok": true,
                "type": "task_status_summary",
                "queued": queued,
                "running": running,
                "done": done,
                "cancelled": cancelled
            })
        );
        0
    }
}

fn cancel_task(root: &Path, parsed: &ParsedCli) -> i32 {
    let ticket = parse_ticket_id(parsed);
    let Some(ticket_id) = ticket else {
        return print_task_error("task_cancel_error", "missing_ticket_id");
    };
    let paths = task_paths(root);
    let mut registry = match load_registry(&paths) {
        Ok(value) => value,
        Err(err) => return print_task_error("task_cancel_error", err),
    };
    let mut cancelled = load_cancelled_set(&paths).unwrap_or_default();
    cancelled.insert(ticket_id.clone());
    if let Some(row) = find_record_mut(&mut registry, &ticket_id) {
        row.status = "cancelled".to_string();
        row.cancelled = true;
        row.updated_at_ms = now_epoch_ms();
    }
    if let Err(err) = save_registry(&paths, &registry) {
        return print_task_error("task_cancel_error", err);
    }
    if let Err(err) = save_cancelled_set(&paths, &cancelled) {
        return print_task_error("task_cancel_error", err);
    }
    let (bus, notes) = build_task_bus(root, &parsed.flags);
    let _ = bus.publish_cancel(&ticket_id);
    let _ = emit_event(
        &paths,
        "task_cancel",
        json!({ "ticket_id": ticket_id, "bus_mode": bus.mode(), "notes": notes }),
    );
    println!(
        "{}",
        json!({
            "ok": true,
            "type": "task_cancelled",
            "ticket_id": ticket_id,
            "bus_mode": bus.mode()
        })
    );
    0
}

fn set_record_status(
    paths: &TaskPaths,
    task_id: &str,
    status: &str,
    progress: u8,
) -> Result<(), String> {
    let mut registry = load_registry(paths)?;
    let now = now_epoch_ms();
    if let Some(record) = find_record_mut(&mut registry, task_id) {
        record.status = status.to_string();
        record.progress_percent = progress;
        record.updated_at_ms = now;
        if status == "cancelled" {
            record.cancelled = true;
        }
    }
    save_registry(paths, &registry)
}

fn complete_record(paths: &TaskPaths, task_id: &str, result: TaskResult) -> Result<(), String> {
    let mut registry = load_registry(paths)?;
    if let Some(record) = find_record_mut(&mut registry, task_id) {
        record.status = result.status.clone();
        record.progress_percent = 100;
        record.updated_at_ms = now_epoch_ms();
        record.result = Some(result);
    }
    save_registry(paths, &registry)
}

fn apply_bus_cancellations(
    paths: &TaskPaths,
    bus: &dyn TaskBus,
    wait_ms: u64,
) -> Result<usize, String> {
    let received = bus.pull_cancelled(64, wait_ms)?;
    if received.is_empty() {
        return Ok(0);
    }
    let mut registry = load_registry(paths)?;
    let mut cancelled = load_cancelled_set(paths)?;
    let mut applied = 0usize;
    let now = now_epoch_ms();
    for task_id in received {
        let clean = clean_id(&task_id);
        if clean.is_empty() || !cancelled.insert(clean.clone()) {
            continue;
        }
        if let Some(record) = find_record_mut(&mut registry, &clean) {
            record.status = "cancelled".to_string();
            record.cancelled = true;
            record.updated_at_ms = now;
        }
        applied = applied.saturating_add(1);
    }
    if applied > 0 {
        save_cancelled_set(paths, &cancelled)?;
        save_registry(paths, &registry)?;
        let _ = emit_event(
            paths,
            "task_cancel_sync",
            json!({
                "cancelled_count": applied,
                "bus_mode": bus.mode(),
                "ts_ms": now
            }),
        );
    }
    Ok(applied)
}

fn process_task(paths: &TaskPaths, payload: &TaskPayload) -> Result<(), String> {
    let started_at_ms = now_epoch_ms();
    set_record_status(paths, &payload.id, "running", 0)?;
    let mut cancelled = load_cancelled_set(paths)?;
    if cancelled.contains(&payload.id) {
        set_record_status(paths, &payload.id, "cancelled", 0)?;
        return Ok(());
    }

    let step_count = payload.steps.max(1);
    let total_ms = payload.estimated_seconds.max(1) * 1000;
    let step_ms = (total_ms / step_count).max(50);
    for step in 1..=step_count {
        thread::sleep(Duration::from_millis(step_ms));
        cancelled = load_cancelled_set(paths)?;
        if cancelled.contains(&payload.id) {
            set_record_status(paths, &payload.id, "cancelled", 0)?;
            let result = TaskResult {
                id: payload.id.clone(),
                status: "cancelled".to_string(),
                summary: "task cancelled by ticket".to_string(),
                completed_at_ms: now_epoch_ms(),
                duration_ms: now_epoch_ms().saturating_sub(started_at_ms),
            };
            complete_record(paths, &payload.id, result.clone())?;
            emit_final_result(paths, &result)?;
            return Ok(());
        }
        let percent = (((step as f64 / step_count as f64) * 100.0).round() as u8).min(100);
        set_record_status(paths, &payload.id, "running", percent)?;
        let update = ProgressUpdate {
            id: payload.id.clone(),
            progress_percent: percent,
            step,
            total_steps: step_count,
            message: format!("{} step {}/{}", payload.kind, step, step_count),
            ts_ms: now_epoch_ms(),
        };
        emit_conduit_update(paths, &update)?;
    }
    let result = TaskResult {
        id: payload.id.clone(),
        status: "done".to_string(),
        summary: format!("task {} completed", payload.kind),
        completed_at_ms: now_epoch_ms(),
        duration_ms: now_epoch_ms().saturating_sub(started_at_ms),
    };
    complete_record(paths, &payload.id, result.clone())?;
    emit_final_result(paths, &result)?;
    Ok(())
}
