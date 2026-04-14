fn build_stress_index_rows(op: usize) -> Vec<DbIndexEntry> {
    let count = 12 + (op % 12);
    (0..count)
        .map(|idx| DbIndexEntry {
            node_id: format!("stress.node.{op}.{idx}"),
            uid: format!("UID{op}{idx}"),
            file_rel: format!("client/memory/2026-03-{:02}.md", (idx % 28) + 1),
            summary: format!("stress summary {op}-{idx}"),
            tags: vec!["stress".to_string(), "episodic".to_string()],
            kind: "episodic".to_string(),
        })
        .collect::<Vec<DbIndexEntry>>()
}

fn build_stress_embedding_rows(op: usize) -> Vec<(String, Vec<f32>, Value)> {
    let count = 8 + (op % 10);
    (0..count)
        .map(|idx| {
            let node_id = format!("stress.semantic.{op}.{idx}");
            let vector = (0..32)
                .map(|dim| (((op + idx + dim) % 17) as f32) / 17.0)
                .collect::<Vec<f32>>();
            (node_id.clone(), vector, json!({"node_id": node_id, "source": "stress_semantic"}))
        })
        .collect::<Vec<(String, Vec<f32>, Value)>>()
}

fn resolve_stress_db_path(root: &Path, db_path_raw: &str) -> PathBuf {
    let candidate = PathBuf::from(db_path_raw);
    if candidate.is_absolute() {
        return candidate;
    }
    root.join(candidate)
}

fn inject_fragmentation_churn(root: &Path, db_path_raw: &str, op: usize) {
    let db_path = resolve_stress_db_path(root, db_path_raw);
    if let Some(parent) = db_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let Ok(conn) = rusqlite::Connection::open(db_path) else {
        return;
    };
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS __stress_fragmentation (
           row_id INTEGER PRIMARY KEY,
           payload TEXT NOT NULL
         );",
    );
    let payload = "z".repeat(2_048 + (op % 512));
    for slot in 0..20usize {
        let _ = conn.execute(
            "INSERT INTO __stress_fragmentation (row_id, payload)
             VALUES (?1, ?2)
             ON CONFLICT(row_id) DO UPDATE SET payload = excluded.payload",
            rusqlite::params![((op * 32) + slot) as i64, payload],
        );
    }
    let _ = conn.execute(
        "DELETE FROM __stress_fragmentation WHERE (row_id % 3) = 0",
        [],
    );
}

fn predictive_defrag_stress_payload(args: &HashMap<String, String>) -> Value {
    let ops =
        parse_u32_clamped(&arg_any(args, &["ops", "operations"]), 100, 50_000, 5_000) as usize;
    let root = PathBuf::from(arg_or_default(
        args,
        "root",
        detect_default_root().to_string_lossy().as_ref(),
    ));
    let mode_hint = resolve_predictive_mode_hint(args);
    let db_path_raw = arg_or_default(
        args,
        "db-path",
        "local/state/memory/runtime_memory_predictive_stress.sqlite",
    );
    let state = Arc::new(Mutex::new(PredictiveDefragMonitorState::default()));
    let mut energy_before_total = 0.0;
    let mut energy_after_total = 0.0;
    let mut latency_before_total = 0.0;
    let mut latency_after_total = 0.0;
    let mut db = match MemoryDb::open(&root, &db_path_raw) {
        Ok(db) => db,
        Err(err) => {
            return json!({"ok": false, "error": err, "ops_completed": 0});
        }
    };
    for op in 0..ops {
        let _ = db.set_hot_state_json(
            &format!("stress.working.{op}"),
            &json!({"op": op, "tier": "working", "payload": "x".repeat(64 + (op % 48))}),
        );
        if op % 25 == 0 {
            let rows = build_stress_index_rows(op);
            let _ = db.replace_index_entries(&rows, "stress_episodic");
        }
        if op % 40 == 0 {
            let rows = build_stress_embedding_rows(op);
            let _ = db.replace_embeddings(&rows, "stress_semantic");
        }
        if op % 15 == 0 {
            inject_fragmentation_churn(&root, &db_path_raw, op);
        }
        if op % 10 == 0 {
            if let Ok(before_stats) = db.fragmentation_stats() {
                let before_percent = before_stats.fragmentation_ratio * 100.0;
                energy_before_total += estimate_memory_energy_units(before_percent, &before_stats);
                latency_before_total +=
                    estimate_context_switch_latency_ms(before_percent, &before_stats);
            }
            run_predictive_defrag_cycle(&root, &db_path_raw, &mode_hint, &state);
            if let Ok(after_db) = MemoryDb::open(&root, &db_path_raw) {
                if let Ok(after_stats) = after_db.fragmentation_stats() {
                    let after_percent = after_stats.fragmentation_ratio * 100.0;
                    energy_after_total += estimate_memory_energy_units(after_percent, &after_stats);
                    latency_after_total +=
                        estimate_context_switch_latency_ms(after_percent, &after_stats);
                }
            }
        }
    }
    let snapshot = state
        .lock()
        .ok()
        .map(|guard| guard.clone())
        .unwrap_or_default();
    let energy_improvement_percent = if energy_before_total > 0.0 {
        round4(((energy_before_total - energy_after_total) / energy_before_total) * 100.0)
    } else {
        0.0
    };
    let latency_improvement_percent = if latency_before_total > 0.0 {
        round4(((latency_before_total - latency_after_total) / latency_before_total) * 100.0)
    } else {
        0.0
    };
    let mut anomalies = Vec::<String>::new();
    if snapshot.trigger_count == 0 {
        anomalies.push("no_predictive_realignments_triggered".to_string());
    }
    if energy_improvement_percent <= 0.0 {
        anomalies.push("no_measurable_energy_improvement".to_string());
    }
    if latency_improvement_percent <= 0.0 {
        anomalies.push("no_measurable_latency_improvement".to_string());
    }
    if snapshot.last_drift_delta <= 0.0 && snapshot.trigger_count > 0 {
        anomalies.push("non_positive_fidelity_drift_delta".to_string());
    }
    json!({
        "ok": true,
        "type": "memory_predictive_defrag_stress_report",
        "operations": ops,
        "fragmentation_trigger_percent": snapshot.last_trigger_fragmentation_percent,
        "energy_improvement_percent": energy_improvement_percent,
        "latency_improvement_percent": latency_improvement_percent,
        "verity_receipts_generated": snapshot.trigger_count,
        "anomalies": anomalies,
        "verity_receipt_summary": {
            "trigger_count": snapshot.trigger_count,
            "last_receipt_hash": snapshot.last_receipt_hash,
            "last_receipt_path": snapshot.last_receipt_path,
            "before_fidelity_score": snapshot.last_before_fidelity_score,
            "after_fidelity_score": snapshot.last_after_fidelity_score,
            "drift_delta": snapshot.last_drift_delta
        }
    })
}
