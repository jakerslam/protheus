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
        eprintln!(
            "{}",
            json!({ "ok": false, "type": "task_submit_error", "error": err })
        );
        return 1;
    }
    let (bus, notes) = build_task_bus(root, &parsed.flags);
    let payload = create_payload_from_flags(parsed);
    let mut registry = match load_registry(&paths) {
        Ok(value) => value,
        Err(err) => {
            eprintln!(
                "{}",
                json!({ "ok": false, "type": "task_submit_error", "error": err })
            );
            return 1;
        }
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
        eprintln!(
            "{}",
            json!({ "ok": false, "type": "task_submit_error", "error": err })
        );
        return 1;
    }
    if let Err(err) = bus.enqueue(&payload) {
        eprintln!(
            "{}",
            json!({ "ok": false, "type": "task_submit_error", "error": err })
        );
        return 1;
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
        Err(err) => {
            eprintln!("{}", json!({ "ok": false, "type": "task_list_error", "error": err }));
            return 1;
        }
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
        Err(err) => {
            eprintln!("{}", json!({ "ok": false, "type": "task_status_error", "error": err }));
            return 1;
        }
    };
    let ticket = parse_non_empty(&parsed.flags, "ticket")
        .or_else(|| parse_non_empty(&parsed.flags, "ticket-id"))
        .or_else(|| parsed.positional.first().cloned())
        .map(|raw| clean_id(raw.as_str()))
        .filter(|value| !value.is_empty());
    if let Some(ticket_id) = ticket {
        let found = registry.tasks.into_iter().find(|row| row.id == ticket_id);
        match found {
            Some(task) => {
                println!("{}", json!({ "ok": true, "type": "task_status", "task": task }));
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
    let ticket = parse_non_empty(&parsed.flags, "ticket")
        .or_else(|| parse_non_empty(&parsed.flags, "ticket-id"))
        .or_else(|| parsed.positional.first().cloned())
        .map(|raw| clean_id(raw.as_str()))
        .filter(|value| !value.is_empty());
    let Some(ticket_id) = ticket else {
        eprintln!(
            "{}",
            json!({ "ok": false, "type": "task_cancel_error", "error": "missing_ticket_id" })
        );
        return 1;
    };
    let paths = task_paths(root);
    let mut registry = match load_registry(&paths) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{}", json!({ "ok": false, "type": "task_cancel_error", "error": err }));
            return 1;
        }
    };
    let mut cancelled = load_cancelled_set(&paths).unwrap_or_default();
    cancelled.insert(ticket_id.clone());
    if let Some(row) = find_record_mut(&mut registry, &ticket_id) {
        row.status = "cancelled".to_string();
        row.cancelled = true;
        row.updated_at_ms = now_epoch_ms();
    }
    if let Err(err) = save_registry(&paths, &registry) {
        eprintln!("{}", json!({ "ok": false, "type": "task_cancel_error", "error": err }));
        return 1;
    }
    if let Err(err) = save_cancelled_set(&paths, &cancelled) {
        eprintln!("{}", json!({ "ok": false, "type": "task_cancel_error", "error": err }));
        return 1;
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

fn set_record_status(paths: &TaskPaths, task_id: &str, status: &str, progress: u8) -> Result<(), String> {
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

fn run_worker(root: &Path, parsed: &ParsedCli) -> i32 {
    let paths = task_paths(root);
    if let Err(err) = ensure_task_state(&paths) {
        eprintln!("{}", json!({ "ok": false, "type": "task_worker_error", "error": err }));
        return 1;
    }
    let max_tasks = parse_u64_flag(&parsed.flags, "max-tasks", 1) as usize;
    let wait_ms = parse_u64_flag(&parsed.flags, "wait-ms", 800);
    let (bus, notes) = build_task_bus(root, &parsed.flags);
    let mut processed = 0usize;
    for _ in 0..max_tasks {
        let batch = match bus.dequeue(1, wait_ms) {
            Ok(rows) => rows,
            Err(err) => {
                eprintln!(
                    "{}",
                    json!({ "ok": false, "type": "task_worker_error", "error": err, "bus_mode": bus.mode() })
                );
                return 1;
            }
        };
        if batch.is_empty() {
            break;
        }
        for payload in batch {
            if let Err(err) = process_task(&paths, &payload) {
                eprintln!(
                    "{}",
                    json!({
                        "ok": false,
                        "type": "task_worker_task_error",
                        "error": err,
                        "task_id": payload.id
                    })
                );
                return 1;
            }
            processed += 1;
        }
    }
    println!(
        "{}",
        json!({
            "ok": true,
            "type": "task_worker",
            "processed": processed,
            "bus_mode": bus.mode(),
            "notes": notes
        })
    );
    0
}

fn run_slow_test(root: &Path, parsed: &ParsedCli) -> i32 {
    let paths = task_paths(root);
    let _ = ensure_task_state(&paths);
    let _ = write_queue_rows(&paths.queue_jsonl, &[]);

    let seconds = parse_u64_flag(&parsed.flags, "seconds", 30);
    let interval_seconds = parse_u64_flag(&parsed.flags, "progress-interval-seconds", 5).max(1);
    let mut submit_flags = parsed.flags.clone();
    submit_flags
        .entry("kind".to_string())
        .or_insert_with(|| "slow-analysis".to_string());
    submit_flags.insert("estimated-seconds".to_string(), seconds.to_string());
    submit_flags.insert(
        "steps".to_string(),
        ((seconds / interval_seconds).max(1)).to_string(),
    );
    let submit = ParsedCli {
        positional: Vec::new(),
        flags: submit_flags,
    };
    let submit_exit = submit_task(root, &submit);
    if submit_exit != 0 {
        return submit_exit;
    }
    let worker_flags = BTreeMap::from([
        ("max-tasks".to_string(), "1".to_string()),
        (
            "wait-ms".to_string(),
            (interval_seconds.saturating_mul(1000)).to_string(),
        ),
        (
            "bus".to_string(),
            parse_non_empty(&parsed.flags, "bus").unwrap_or_else(|| "auto".to_string()),
        ),
    ]);
    run_worker(
        root,
        &ParsedCli {
            positional: Vec::new(),
            flags: worker_flags,
        },
    )
}
