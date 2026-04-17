fn run_predictive_defrag_cycle(
    root: &Path,
    db_path_raw: &str,
    mode_hint: &str,
    state: &Arc<Mutex<PredictiveDefragMonitorState>>,
) -> u64 {
    let policy = load_predictive_defrag_policy(root, mode_hint);
    let active_threshold_percent = policy.active_threshold_percent.clamp(0.0, 100.0);
    let reactive_threshold_percent = policy
        .reactive_threshold_percent
        .clamp(active_threshold_percent, 100.0);
    let checked_at = now_epoch_ms();
    if let Ok(mut guard) = state.lock() {
        guard.checks = guard.checks.saturating_add(1);
        guard.last_checked_at_ms = checked_at;
        guard.mode = policy.mode.clone();
        guard.enabled = policy.enabled;
        guard.active_threshold_percent = active_threshold_percent;
        guard.reactive_threshold_percent = reactive_threshold_percent;
        guard.last_error.clear();
    }
    if !policy.enabled {
        return policy.poll_interval_ms;
    }
    let db = match MemoryDb::open(root, db_path_raw) {
        Ok(db) => db,
        Err(err) => {
            if let Ok(mut guard) = state.lock() {
                guard.last_error = err;
            }
            return policy.poll_interval_ms;
        }
    };
    let before_stats = match db.fragmentation_stats() {
        Ok(stats) => stats,
        Err(err) => {
            if let Ok(mut guard) = state.lock() {
                guard.last_error = err;
            }
            return policy.poll_interval_ms;
        }
    };
    let before_percent = before_stats.fragmentation_ratio * 100.0;
    if before_percent + f64::EPSILON < active_threshold_percent {
        return policy.poll_interval_ms;
    }
    let last_triggered_at_ms = state
        .lock()
        .ok()
        .map(|guard| guard.last_triggered_at_ms)
        .unwrap_or(0);
    if last_triggered_at_ms > 0
        && checked_at.saturating_sub(last_triggered_at_ms) < PREDICTIVE_DEFRAG_TRIGGER_COOLDOWN_MS
    {
        return policy.poll_interval_ms;
    }

    // MEMORY PROXY + VERITY: Predictive defrag at 6% threshold to prevent fragmentation drift and latency spikes
    let before_fidelity = memory_fidelity_score(before_percent);
    let before_energy = estimate_memory_energy_units(before_percent, &before_stats);
    let before_latency = estimate_context_switch_latency_ms(before_percent, &before_stats);
    if let Err(err) = db.predictive_realign_compaction() {
        if let Ok(mut guard) = state.lock() {
            guard.last_error = err;
        }
        return policy.poll_interval_ms;
    }
    let after_stats = match db.fragmentation_stats() {
        Ok(stats) => stats,
        Err(err) => {
            if let Ok(mut guard) = state.lock() {
                guard.last_error = err;
            }
            return policy.poll_interval_ms;
        }
    };
    let after_percent = after_stats.fragmentation_ratio * 100.0;
    let after_fidelity = memory_fidelity_score(after_percent);
    let drift_delta = round4(after_fidelity - before_fidelity);
    let after_energy = estimate_memory_energy_units(after_percent, &after_stats);
    let after_latency = estimate_context_switch_latency_ms(after_percent, &after_stats);
    let energy_improvement_percent = if before_energy > 0.0 {
        round4(((before_energy - after_energy) / before_energy) * 100.0)
    } else {
        0.0
    };
    let latency_improvement_percent = if before_latency > 0.0 {
        round4(((before_latency - after_latency) / before_latency) * 100.0)
    } else {
        0.0
    };
    let triggered_at = now_epoch_ms();
    let mut receipt = json!({
        "ok": true,
        "type": "verity_memory_predictive_realignment_receipt",
        "mode": policy.mode,
        "memory": { "predictive_defrag": { "enabled": policy.enabled } },
        "policy_version": policy.policy_version,
        "signature_valid": policy.signature_valid,
        "config_path": policy.config_path,
        "trigger_threshold_percent": active_threshold_percent,
        "reactive_threshold_percent": reactive_threshold_percent,
        "fragmentation_percent_before": round4(before_percent),
        "fragmentation_percent_after": round4(after_percent),
        "before_fidelity_score": before_fidelity,
        "after_fidelity_score": after_fidelity,
        "drift_delta": drift_delta,
        "tiers_before": {
            "working": before_stats.working_rows,
            "episodic": before_stats.episodic_rows,
            "semantic": before_stats.semantic_rows
        },
        "tiers_after": {
            "working": after_stats.working_rows,
            "episodic": after_stats.episodic_rows,
            "semantic": after_stats.semantic_rows
        },
        "energy_units_before": before_energy,
        "energy_units_after": after_energy,
        "energy_improvement_percent": energy_improvement_percent,
        "latency_ms_before": before_latency,
        "latency_ms_after": after_latency,
        "latency_improvement_percent": latency_improvement_percent,
        "ts_ms": triggered_at
    });
    let receipt_hash =
        sha256_hex(&serde_json::to_string(&receipt).unwrap_or_else(|_| "{}".to_string()));
    receipt["receipt_hash"] = Value::String(receipt_hash.clone());
    let receipt_path = append_predictive_defrag_receipt(root, &receipt);
    if let Ok(mut guard) = state.lock() {
        guard.trigger_count = guard.trigger_count.saturating_add(1);
        guard.last_triggered_at_ms = triggered_at;
        guard.last_trigger_fragmentation_percent = round4(before_percent);
        guard.last_before_fidelity_score = before_fidelity;
        guard.last_after_fidelity_score = after_fidelity;
        guard.last_drift_delta = drift_delta;
        guard.last_energy_improvement_percent = energy_improvement_percent;
        guard.last_latency_improvement_percent = latency_improvement_percent;
        guard.last_receipt_hash = receipt_hash;
        guard.last_receipt_path = receipt_path;
    }
    policy.poll_interval_ms
}

fn start_predictive_defrag_monitor(args: &HashMap<String, String>) -> PredictiveDefragMonitorHandle {
    let root = PathBuf::from(arg_or_default(
        args,
        "root",
        detect_default_root().to_string_lossy().as_ref(),
    ));
    let mode_hint = resolve_predictive_mode_hint(args);
    let db_path_raw = arg_any(args, &["db-path", "db_path"]);
    let state = Arc::new(Mutex::new(PredictiveDefragMonitorState::default()));
    let shutdown = Arc::new(AtomicBool::new(false));
    let worker_root = root.clone();
    let worker_mode = mode_hint.clone();
    let worker_db_path = db_path_raw.clone();
    let worker_state = Arc::clone(&state);
    let worker_shutdown = Arc::clone(&shutdown);
    let worker = thread::spawn(move || {
        while !worker_shutdown.load(AtomicOrdering::Relaxed) {
            let poll_ms =
                run_predictive_defrag_cycle(&worker_root, &worker_db_path, &worker_mode, &worker_state)
                    .max(250);
            let mut waited = 0u64;
            while waited < poll_ms {
                if worker_shutdown.load(AtomicOrdering::Relaxed) {
                    return;
                }
                let slice = (poll_ms - waited).min(200);
                thread::sleep(Duration::from_millis(slice));
                waited += slice;
            }
        }
    });
    PredictiveDefragMonitorHandle {
        root,
        mode_hint,
        db_path_raw,
        state,
        shutdown,
        worker: Some(worker),
    }
}
