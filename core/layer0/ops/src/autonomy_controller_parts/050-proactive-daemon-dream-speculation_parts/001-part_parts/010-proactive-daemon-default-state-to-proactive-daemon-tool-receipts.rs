fn proactive_daemon_default_state() -> Value {
    json!({
        "version": "v2",
        "paused": false,
        "cycles": 0u64,
        "last_intents": [],
        "last_executed_intents": [],
        "last_deferred_intents": [],
        "heartbeat": {
            "tick_ms": 5000u64,
            "jitter_ms": 400u64,
            "last_tick_ms": 0u64,
            "next_tick_after_ms": 0u64
        },
        "proactive": {
            "window_sec": 900u64,
            "max_messages": 2u64,
            "sent_in_window": 0u64,
            "window_started_at_ms": 0u64,
            "brief_mode": true
        },
        "budgets": {
            "blocking_ms": 15000u64
        },
        "tool_surfaces": {
            "policy_tier": "observe",
            "enabled": ["subscribe_pr", "push_notification", "send_user_file"],
            "receipts_written": 0u64,
            "last_receipt_id": Value::Null
        },
        "pattern_log": {
            "history": [],
            "last_cycle_patterns": [],
            "suggestions": [],
            "last_logged_at": Value::Null
        },
        "failure_isolation": {
            "history": [],
            "quarantine": [],
            "last_isolation": Value::Null
        },
        "recovery_matrix": {
            "history": [],
            "attempts_by_task": {},
            "last_strategy": Value::Null,
            "last_outcome": Value::Null
        },
        "dream": {
            "max_idle_ms": 6u64 * 60u64 * 60u64 * 1000u64,
            "max_without_dream_ms": 24u64 * 60u64 * 60u64 * 1000u64,
            "last_dream_at_ms": 0u64,
            "last_dream_reason": Value::Null,
            "last_dream_hand_id": Value::Null,
            "last_cleanup_ok": Value::Null
        },
        "write_discipline": {
            "state_write_confirmed": false,
            "last_state_write_at": Value::Null
        }
    })
}

fn ensure_proactive_daemon_state_shape(state: &mut Value) {
    if !state.is_object() {
        *state = proactive_daemon_default_state();
    }
    if !state
        .get("heartbeat")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["heartbeat"] = proactive_daemon_default_state()["heartbeat"].clone();
    }
    if !state
        .get("proactive")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["proactive"] = proactive_daemon_default_state()["proactive"].clone();
    }
    if !state.get("budgets").map(Value::is_object).unwrap_or(false) {
        state["budgets"] = proactive_daemon_default_state()["budgets"].clone();
    }
    if !state
        .get("tool_surfaces")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["tool_surfaces"] = proactive_daemon_default_state()["tool_surfaces"].clone();
    }
    if !state
        .get("pattern_log")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["pattern_log"] = proactive_daemon_default_state()["pattern_log"].clone();
    }
    if !state
        .get("failure_isolation")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["failure_isolation"] = proactive_daemon_default_state()["failure_isolation"].clone();
    }
    if !state
        .get("recovery_matrix")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["recovery_matrix"] = proactive_daemon_default_state()["recovery_matrix"].clone();
    }
    if !state.get("dream").map(Value::is_object).unwrap_or(false) {
        state["dream"] = proactive_daemon_default_state()["dream"].clone();
    }
    if !state
        .get("write_discipline")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["write_discipline"] = proactive_daemon_default_state()["write_discipline"].clone();
    }
    for key in [
        "last_intents",
        "last_executed_intents",
        "last_deferred_intents",
        "pattern_log/history",
        "pattern_log/last_cycle_patterns",
        "pattern_log/suggestions",
        "failure_isolation/history",
        "failure_isolation/quarantine",
        "recovery_matrix/history",
    ] {
        let exists = if key.contains('/') {
            state.pointer(&format!("/{key}"))
                .map(Value::is_array)
                .unwrap_or(false)
        } else {
            state.get(key).map(Value::is_array).unwrap_or(false)
        };
        if !exists {
            if key.contains('/') {
                let mut pieces = key.split('/');
                let head = pieces.next().unwrap_or_default();
                let tail = pieces.next().unwrap_or_default();
                if !head.is_empty() && !tail.is_empty() {
                    state[head][tail] = Value::Array(Vec::new());
                }
            } else {
                state[key] = Value::Array(Vec::new());
            }
        }
    }
}

fn intent_estimated_blocking_ms(intent: &Value) -> u64 {
    match intent
        .get("task")
        .and_then(Value::as_str)
        .unwrap_or_default()
    {
        "sweep_dead_letters" => 5_000,
        "autoscale_review" => 4_000,
        "dream_consolidation" => 2_500,
        "compact_hand_memory" => 800,
        "pattern_log" => 200,
        "subscribe_pr" => 220,
        "push_notification" => 180,
        "send_user_file" => 260,
        _ => 1_000,
    }
}

fn deterministic_jitter_ms(cycle: u64, jitter_ms: u64) -> u64 {
    if jitter_ms == 0 {
        return 0;
    }
    let seed = receipt_hash(&json!({"cycle": cycle, "jitter_ms": jitter_ms}));
    let n = u64::from_str_radix(seed.get(0..8).unwrap_or("0"), 16).unwrap_or(0);
    n % (jitter_ms.saturating_mul(2).saturating_add(1))
}

fn rollover_proactive_window(state: &mut Value, now_ms: u64) {
    let window_sec = state
        .pointer("/proactive/window_sec")
        .and_then(Value::as_u64)
        .unwrap_or(900);
    let window_started = state
        .pointer("/proactive/window_started_at_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let window_ms = window_sec.saturating_mul(1000);
    if window_started == 0 || now_ms.saturating_sub(window_started) >= window_ms {
        state["proactive"]["window_started_at_ms"] = json!(now_ms);
        state["proactive"]["sent_in_window"] = json!(0u64);
    }
}

fn append_proactive_daemon_log(root: &Path, row: &Value, strict: bool) -> Result<(), String> {
    let path = proactive_daemon_daily_log_path(root, &proactive_daemon_today_ymd());
    append_jsonl(&path, row)?;
    if strict {
        let rows = read_jsonl(&path);
        if rows.is_empty() {
            return Err("proactive_daemon_log_append_verification_failed".to_string());
        }
    }
    Ok(())
}

fn proactive_daemon_tool_receipts_path(root: &Path) -> PathBuf {
    state_root(root)
        .join("proactive_daemon")
        .join("tool_receipts.jsonl")
}
