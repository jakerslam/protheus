fn emit_task_worker_error(error: &str) {
    eprintln!(
        "{}",
        json!({ "ok": false, "type": "task_worker_error", "error": error })
    );
}

fn emit_task_worker_bus_error(error: &str, bus_mode: &str) {
    eprintln!(
        "{}",
        json!({ "ok": false, "type": "task_worker_error", "error": error, "bus_mode": bus_mode })
    );
}

fn run_worker(root: &Path, parsed: &ParsedCli) -> i32 {
    let paths = task_paths(root);
    if let Err(err) = ensure_task_state(&paths) {
        emit_task_worker_error(&err);
        return 1;
    }
    let max_tasks = parse_u64_flag(&parsed.flags, "max-tasks", 0) as usize;
    let wait_ms = parse_u64_flag(&parsed.flags, "wait-ms", DEFAULT_WORKER_MIN_POLL_MS)
        .clamp(DEFAULT_WORKER_MIN_POLL_MS, DEFAULT_WORKER_MAX_POLL_MS);
    let idle_hibernate_ms = parse_u64_flag(
        &parsed.flags,
        "idle-hibernate-ms",
        DEFAULT_WORKER_IDLE_HIBERNATE_MS,
    )
    .clamp(1_000, 900_000);
    let service_mode = parse_bool_flag(&parsed.flags, "service", max_tasks == 0);
    let (bus, notes) = build_task_bus(root, &parsed.flags);
    let worker_id = Uuid::new_v4().to_string();
    let _ = mark_worker_started(&paths, &worker_id, bus.mode(), service_mode);
    let mut processed = 0usize;
    let mut polls = 0usize;
    let mut hibernated = false;
    let mut idle_since_ms = now_epoch_ms();
    let mut poll_wait_ms = wait_ms;
    let mut cancel_sync_total = 0usize;
    loop {
        if max_tasks > 0 && processed >= max_tasks {
            break;
        }
        let cancel_wait_ms = (poll_wait_ms / 4).clamp(10, 100);
        let synced = match apply_bus_cancellations(&paths, bus.as_ref(), cancel_wait_ms) {
            Ok(value) => value,
            Err(err) => {
                emit_task_worker_bus_error(&err, bus.mode());
                return 1;
            }
        };
        cancel_sync_total = cancel_sync_total.saturating_add(synced);
        let batch = match bus.dequeue(1, poll_wait_ms) {
            Ok(rows) => rows,
            Err(err) => {
                emit_task_worker_bus_error(&err, bus.mode());
                return 1;
            }
        };
        if batch.is_empty() {
            polls += 1;
            let now = now_epoch_ms();
            let idle_ms = now.saturating_sub(idle_since_ms);
            let _ = mark_worker_poll(&paths, &worker_id, true, poll_wait_ms);
            if service_mode {
                if idle_ms >= idle_hibernate_ms {
                    hibernated = true;
                    let _ = mark_worker_hibernated(&paths, &worker_id, idle_ms, processed);
                    break;
                }
                poll_wait_ms = poll_wait_ms
                    .saturating_mul(2)
                    .clamp(DEFAULT_WORKER_MIN_POLL_MS, DEFAULT_WORKER_MAX_POLL_MS);
                continue;
            }
            break;
        }
        idle_since_ms = now_epoch_ms();
        poll_wait_ms = wait_ms;
        let _ = mark_worker_poll(&paths, &worker_id, false, poll_wait_ms);
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
    if !hibernated {
        let _ = mark_worker_stopped(&paths, &worker_id, processed);
    }
    println!(
        "{}",
        json!({
            "ok": true,
            "type": "task_worker",
            "processed": processed,
            "polls": polls,
            "hibernated": hibernated,
            "service_mode": service_mode,
            "idle_hibernate_ms": idle_hibernate_ms,
            "cancel_sync_total": cancel_sync_total,
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
