const PROACTIVE_DAEMON_REACTIVE_COMPACTION_PRESSURE_RATIO: f64 = 0.95;

fn normalize_compaction_mode(raw: Option<String>) -> String {
    let normalized = clean_id(raw, "reactive");
    match normalized.as_str() {
        "snip" | "micro" | "full" | "reactive" => normalized,
        _ => "reactive".to_string(),
    }
}

fn ensure_hand_memory_tiers(hand: &mut Value) {
    if !hand.get("memory").map(Value::is_object).unwrap_or(false) {
        hand["memory"] = json!({"core":[],"archival":[],"external":[]});
    }
    for tier in ["core", "archival", "external"] {
        if !hand["memory"]
            .get(tier)
            .map(Value::is_array)
            .unwrap_or(false)
        {
            hand["memory"][tier] = Value::Array(Vec::new());
        }
    }
}

fn memory_tier(hand: &Value, tier: &str) -> Vec<Value> {
    hand.pointer(&format!("/memory/{tier}"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn set_memory_tier(hand: &mut Value, tier: &str, rows: Vec<Value>) {
    hand["memory"][tier] = Value::Array(rows);
}

fn entry_text(row: &Value) -> String {
    if let Some(v) = row.get("text").and_then(Value::as_str) {
        return v.trim().to_string();
    }
    if let Some(v) = row.as_str() {
        return v.trim().to_string();
    }
    if let Some(v) = row.get("key").and_then(Value::as_str) {
        return v.trim().to_string();
    }
    String::new()
}

fn compact_rows(rows: Vec<Value>, keep_ratio: f64, min_keep: usize) -> (Vec<Value>, Vec<Value>) {
    if rows.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let mut keep = ((rows.len() as f64) * keep_ratio.clamp(0.05, 0.95)).round() as usize;
    keep = keep.max(min_keep).min(rows.len());
    let cut = rows.len().saturating_sub(keep);
    let removed = rows.iter().take(cut).cloned().collect::<Vec<_>>();
    let kept = rows.into_iter().skip(cut).collect::<Vec<_>>();
    (kept, removed)
}

fn compact_hand_memory(
    root: &Path,
    hand_id: &str,
    mode: &str,
    pressure_ratio: f64,
    auto_compact_pct: Option<f64>,
) -> Result<Value, String> {
    let path = hand_path(root, hand_id);
    let mut hand = read_json(&path).unwrap_or_else(|| {
        json!({
            "version":"v1",
            "hand_id": hand_id,
            "memory":{"core":[],"archival":[],"external":[]}
        })
    });
    ensure_hand_memory_tiers(&mut hand);

    let before_core = memory_tier(&hand, "core");
    let before_archival = memory_tier(&hand, "archival");
    let before_external = memory_tier(&hand, "external");

    let (mut core_ratio, mut archival_ratio, mut external_ratio) = match mode {
        "snip" => (0.30, 0.22, 0.18),
        "micro" => (0.82, 0.70, 0.62),
        "full" => (0.62, 0.48, 0.40),
        _ => {
            let p = pressure_ratio.clamp(0.0, 1.0);
            if p >= 0.95 {
                (0.48, 0.34, 0.28)
            } else if p >= 0.85 {
                (0.62, 0.46, 0.40)
            } else if p >= 0.70 {
                (0.74, 0.58, 0.52)
            } else {
                (0.84, 0.72, 0.64)
            }
        }
    };
    if let Some(pct) = auto_compact_pct {
        let keep = (1.0 - (pct / 100.0)).clamp(0.05, 0.95);
        core_ratio = keep;
        archival_ratio = keep;
        external_ratio = keep;
    }

    let (core_next, core_removed) = compact_rows(before_core.clone(), core_ratio, 4);
    let (arch_next, arch_removed) = compact_rows(before_archival.clone(), archival_ratio, 8);
    let (ext_next, ext_removed) = compact_rows(before_external.clone(), external_ratio, 6);

    set_memory_tier(&mut hand, "core", core_next.clone());
    let mut archival_out = arch_next.clone();
    if mode == "full"
        && (!core_removed.is_empty() || !arch_removed.is_empty() || !ext_removed.is_empty())
    {
        let summary = json!({
            "type": "dream_compaction_keyframe",
            "captured_at": now_iso(),
            "mode": mode,
            "summary": format!(
                "full compaction removed core={} archival={} external={}",
                core_removed.len(),
                arch_removed.len(),
                ext_removed.len()
            ),
            "removed_preview": core_removed
                .iter()
                .chain(arch_removed.iter())
                .chain(ext_removed.iter())
                .map(entry_text)
                .filter(|v| !v.is_empty())
                .take(6)
                .collect::<Vec<_>>()
        });
        archival_out.push(summary);
    }
    set_memory_tier(&mut hand, "archival", archival_out.clone());
    set_memory_tier(&mut hand, "external", ext_next.clone());
    hand["updated_at"] = Value::String(now_iso());
    write_json(&path, &hand)?;

    Ok(json!({
        "ok": true,
        "hand_id": hand_id,
        "mode": mode,
        "pressure_ratio": pressure_ratio,
        "before": {
            "core": before_core.len(),
            "archival": before_archival.len(),
            "external": before_external.len()
        },
        "after": {
            "core": core_next.len(),
            "archival": archival_out.len(),
            "external": ext_next.len()
        },
        "removed": {
            "core": core_removed.len(),
            "archival": arch_removed.len(),
            "external": ext_removed.len()
        }
    }))
}

fn run_tiered_compaction(root: &Path, argv: &[String]) -> i32 {
    let strict = parse_bool(parse_flag(argv, "strict").as_deref(), true);
    if let Some(mut denied) = conduit_guard(argv, strict) {
        return emit_receipt(root, &mut denied);
    }
    let hand_id = clean_id(
        parse_flag(argv, "hand-id").or_else(|| parse_flag(argv, "id")),
        "hand-default",
    );
    let mode =
        normalize_compaction_mode(parse_flag(argv, "mode").or_else(|| parse_positional(argv, 1)));
    let pressure_ratio = parse_flag(argv, "pressure-ratio")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.85)
        .clamp(0.0, 1.0);
    let auto_compact_pct = parse_flag(argv, "auto-compact-pct").and_then(|v| v.parse::<f64>().ok());

    let result = match compact_hand_memory(root, &hand_id, &mode, pressure_ratio, auto_compact_pct)
    {
        Ok(payload) => payload,
        Err(err) => {
            let mut out = cli_error_receipt(argv, &format!("compact_failed:{err}"), 2);
            out["type"] = json!("autonomy_tiered_compact");
            return emit_receipt(root, &mut out);
        }
    };
    let mut out = json!({
        "ok": true,
        "type": "autonomy_tiered_compact",
        "lane": LANE_ID,
        "strict": strict,
        "result": result,
        "claim_evidence": [
            {
                "id": "V6-MEMORY-032.1",
                "claim": "tiered_compaction_modes_are_policy_selectable_with_pressure_aware_autocompaction",
                "evidence": {"hand_id": hand_id, "mode": mode}
            }
        ]
    });
    emit_receipt(root, &mut out)
}
fn dream_events_path(root: &Path) -> PathBuf {
    state_root(root).join("dream").join("consolidation.jsonl")
}

fn run_dream_consolidation(root: &Path, argv: &[String]) -> i32 {
    let strict = parse_bool(parse_flag(argv, "strict").as_deref(), true);
    if let Some(mut denied) = conduit_guard(argv, strict) {
        return emit_receipt(root, &mut denied);
    }
    let hand_id = clean_id(
        parse_flag(argv, "hand-id").or_else(|| parse_flag(argv, "id")),
        "hand-default",
    );
    let path = hand_path(root, &hand_id);
    let mut hand = read_json(&path)
        .unwrap_or_else(|| json!({"memory":{"core":[],"archival":[],"external":[]}}));
    ensure_hand_memory_tiers(&mut hand);

    let core = memory_tier(&hand, "core");
    let archival = memory_tier(&hand, "archival");
    let external = memory_tier(&hand, "external");

    let orient = core
        .iter()
        .rev()
        .chain(archival.iter().rev())
        .filter_map(|v| {
            let text = entry_text(v);
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        })
        .take(8)
        .collect::<Vec<_>>();
    let orient_tokens = orient
        .iter()
        .flat_map(|t| t.split_whitespace().map(|w| w.to_ascii_lowercase()))
        .filter(|w| w.len() >= 4)
        .take(24)
        .collect::<std::collections::BTreeSet<_>>();
    let gathered = core
        .iter()
        .chain(archival.iter())
        .chain(external.iter())
        .filter(|row| {
            let txt = entry_text(row).to_ascii_lowercase();
            !txt.is_empty() && orient_tokens.iter().any(|token| txt.contains(token))
        })
        .take(64)
        .cloned()
        .collect::<Vec<_>>();

    let consolidate = json!({
        "type": "dream_consolidated_keyframe",
        "captured_at": now_iso(),
        "orient_count": orient.len(),
        "gather_count": gathered.len(),
        "summary": orient.iter().take(4).cloned().collect::<Vec<_>>().join(" | ")
    });
    let mut archival_next = archival.clone();
    archival_next.push(consolidate.clone());
    if archival_next.len() > 256 {
        let trim = archival_next.len().saturating_sub(256);
        archival_next.drain(0..trim);
    }
    set_memory_tier(&mut hand, "archival", archival_next);

    let mut external_next = external.clone();
    if external_next.len() > 192 {
        let trim = external_next.len().saturating_sub(192);
        external_next.drain(0..trim);
    }
    set_memory_tier(&mut hand, "external", external_next.clone());
    hand["updated_at"] = Value::String(now_iso());
    if let Err(err) = write_json(&path, &hand) {
        let mut out = cli_error_receipt(argv, &format!("dream_write_failed:{err}"), 2);
        out["type"] = json!("autonomy_dream_consolidation");
        return emit_receipt(root, &mut out);
    }

    let phase_receipts = ["orient", "gather", "consolidate", "prune"]
        .iter()
        .map(|phase| {
            let row = json!({
                "type": "dream_phase",
                "phase": phase,
                "hand_id": hand_id,
                "ts": now_iso(),
                "stats": {"orient": orient.len(), "gathered": gathered.len(), "external_after": external_next.len()}
            });
            json!({"phase": phase, "receipt": receipt_hash(&row)})
        })
        .collect::<Vec<_>>();
    let event = json!({
        "type": "dream_consolidation_event",
        "hand_id": hand_id,
        "ts": now_iso(),
        "phase_receipts": phase_receipts,
        "orient": orient,
        "gathered_count": gathered.len(),
        "consolidated": consolidate
    });
    if let Err(err) = append_jsonl(&dream_events_path(root), &event) {
        let mut out = cli_error_receipt(argv, &format!("dream_event_append_failed:{err}"), 2);
        out["type"] = json!("autonomy_dream_consolidation");
        return emit_receipt(root, &mut out);
    }

    let mut out = json!({
        "ok": true,
        "type": "autonomy_dream_consolidation",
        "lane": LANE_ID,
        "strict": strict,
        "event": event,
        "claim_evidence": [
            {
                "id": "V6-MEMORY-032.2",
                "claim": "dream_consolidation_executes_orient_gather_consolidate_prune_with_phase_receipts",
                "evidence": {"hand_id": hand_id}
            }
        ]
    });
    emit_receipt(root, &mut out)
}
fn proactive_daemon_state_path(root: &Path) -> PathBuf {
    state_root(root).join("proactive_daemon").join("state.json")
}

fn proactive_daemon_logs_dir(root: &Path) -> PathBuf {
    state_root(root).join("proactive_daemon").join("logs")
}

fn proactive_daemon_daily_log_path(root: &Path, ymd: &str) -> PathBuf {
    proactive_daemon_logs_dir(root).join(format!("{ymd}.jsonl"))
}

fn now_epoch_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn proactive_daemon_today_ymd() -> String {
    now_iso().chars().take(10).collect()
}

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
    ] {
        if !state.get(key).map(Value::is_array).unwrap_or(false) {
            state[key] = Value::Array(Vec::new());
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
        "compact_hand_memory" => 800,
        "pattern_log" => 200,
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

fn persist_proactive_daemon_state(root: &Path, state: &mut Value, strict: bool) -> Result<(), String> {
    let path = proactive_daemon_state_path(root);
    state["write_discipline"]["state_write_confirmed"] = json!(false);
    state["write_discipline"]["last_state_write_at"] = json!(now_iso());
    state["write_discipline"]["state_path"] = json!(path.display().to_string());
    write_json(&path, state)?;
    let persisted = read_json(&path).unwrap_or(Value::Null);
    let confirmed = persisted.get("updated_at") == state.get("updated_at");
    state["write_discipline"]["state_write_confirmed"] = json!(confirmed);
    if strict && !confirmed {
        return Err("proactive_daemon_state_write_confirm_failed".to_string());
    }
    write_json(&path, state)?;
    Ok(())
}

fn run_proactive_daemon_daemon(root: &Path, argv: &[String]) -> i32 {
    let strict = parse_bool(parse_flag(argv, "strict").as_deref(), true);
    if let Some(mut denied) = conduit_guard(argv, strict) {
        return emit_receipt(root, &mut denied);
    }
    let action = clean_id(
        parse_flag(argv, "action").or_else(|| parse_positional(argv, 1)),
        "status",
    );
    let auto = parse_bool(parse_flag(argv, "auto").as_deref(), false);
    let force_cycle = parse_bool(parse_flag(argv, "force").as_deref(), false);
    let tick_ms = parse_u64(parse_flag(argv, "tick-ms").as_deref(), 5000, 1000, 60_000);
    let jitter_ms = parse_u64(
        parse_flag(argv, "jitter-ms").as_deref(),
        400,
        0,
        tick_ms.min(5_000),
    );
    let window_sec = parse_u64(parse_flag(argv, "window-sec").as_deref(), 900, 10, 86_400);
    let max_messages = parse_u64(parse_flag(argv, "max-proactive").as_deref(), 2, 1, 64);
    let blocking_budget_ms = parse_u64(
        parse_flag(argv, "block-budget-ms").as_deref(),
        15_000,
        50,
        120_000,
    );
    let brief_mode = parse_bool(parse_flag(argv, "brief").as_deref(), true);
    let now_ms = now_epoch_ms();

    let mut state = read_json(&proactive_daemon_state_path(root)).unwrap_or_else(proactive_daemon_default_state);
    ensure_proactive_daemon_state_shape(&mut state);
    state["heartbeat"]["tick_ms"] = json!(tick_ms);
    state["heartbeat"]["jitter_ms"] = json!(jitter_ms);
    state["proactive"]["window_sec"] = json!(window_sec);
    state["proactive"]["max_messages"] = json!(max_messages);
    state["proactive"]["brief_mode"] = json!(brief_mode);
    state["budgets"]["blocking_ms"] = json!(blocking_budget_ms);
    rollover_proactive_window(&mut state, now_ms);

    let mut cycle_log_row = Value::Null;
    match action.as_str() {
        "pause" => {
            state["paused"] = json!(true);
            state["last_decision"] = json!("paused");
        }
        "resume" => {
            state["paused"] = json!(false);
            state["last_decision"] = json!("resumed");
        }
        "cycle" | "run" => {
            if !state
                .get("paused")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                let next_tick_after = state
                    .pointer("/heartbeat/next_tick_after_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                if !force_cycle && next_tick_after > now_ms {
                    state["last_decision"] = json!("tick_deferred");
                    state["tick_deferred_reason"] = json!("heartbeat_not_due");
                } else {
                    let swarm = read_json(&root.join("local/state/ops/swarm_runtime/latest.json"))
                        .unwrap_or_else(|| json!({}));
                    let dead_letters = swarm
                        .get("dead_letters")
                        .and_then(Value::as_array)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    let sessions = swarm
                        .get("sessions")
                        .and_then(Value::as_object)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    let mut intents = vec![];
                    if dead_letters > 0 {
                        intents.push(json!({"kind":"reliability","task":"sweep_dead_letters","priority":"medium","count":dead_letters}));
                    }
                    if sessions > 2000 {
                        intents.push(json!({"kind":"capacity","task":"autoscale_review","priority":"high","session_count":sessions}));
                    }
                    for hand_file in std::fs::read_dir(state_root(root).join("hands"))
                        .ok()
                        .into_iter()
                        .flat_map(|it| it.flatten())
                    {
                        let hand = read_json(&hand_file.path()).unwrap_or_else(|| json!({}));
                        let hand_id = clean_id(
                            hand.get("hand_id")
                                .and_then(Value::as_str)
                                .map(|v| v.to_string()),
                            "hand-default",
                        );
                        let core_count = hand
                            .pointer("/memory/core")
                            .and_then(Value::as_array)
                            .map(|v| v.len())
                            .unwrap_or(0);
                        if core_count >= 96 {
                            intents.push(json!({"kind":"memory","task":"compact_hand_memory","hand_id":hand_id,"mode":"reactive","priority":"medium"}));
                        }
                    }
                    if intents.is_empty() {
                        intents.push(
                            json!({"kind":"maintenance","task":"pattern_log","priority":"low"}),
                        );
                    }
                    let mut executed = vec![];
                    let mut deferred = vec![];
                    let mut blocking_used_ms = 0u64;
                    let mut sent_in_window = state
                        .pointer("/proactive/sent_in_window")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    for intent in intents.iter() {
                        let estimate_ms = intent_estimated_blocking_ms(intent);
                        if auto {
                            if sent_in_window >= max_messages {
                                deferred.push(json!({"intent": intent, "reason":"rate_limit"}));
                                continue;
                            }
                            if blocking_used_ms.saturating_add(estimate_ms) > blocking_budget_ms {
                                deferred
                                    .push(json!({"intent": intent, "reason":"blocking_budget"}));
                                continue;
                            }
                            if intent.get("task").and_then(Value::as_str)
                                == Some("compact_hand_memory")
                            {
                                if let Some(hand_id) = intent.get("hand_id").and_then(Value::as_str)
                                {
                                    let compact_result = compact_hand_memory(
                                        root,
                                        hand_id,
                                        "reactive",
                                        PROACTIVE_DAEMON_REACTIVE_COMPACTION_PRESSURE_RATIO,
                                        None,
                                    );
                                    if compact_result.is_err() {
                                        deferred.push(
                                            json!({"intent": intent, "reason":"compact_failed"}),
                                        );
                                        continue;
                                    }
                                }
                            }
                            sent_in_window = sent_in_window.saturating_add(1);
                            blocking_used_ms = blocking_used_ms.saturating_add(estimate_ms);
                            let mut execution = json!({
                                "intent": intent,
                                "estimated_blocking_ms": estimate_ms
                            });
                            if intent.get("task").and_then(Value::as_str)
                                == Some("compact_hand_memory")
                            {
                                execution["pressure_ratio"] =
                                    json!(PROACTIVE_DAEMON_REACTIVE_COMPACTION_PRESSURE_RATIO);
                            }
                            executed.push(execution);
                        }
                    }
                    state["last_intents"] = Value::Array(intents.clone());
                    state["last_executed_intents"] = Value::Array(executed.clone());
                    state["last_deferred_intents"] = Value::Array(deferred.clone());
                    state["proactive"]["sent_in_window"] = json!(sent_in_window);

                    let cycles = state.get("cycles").and_then(Value::as_u64).unwrap_or(0) + 1;
                    state["cycles"] = json!(cycles);
                    state["last_cycle_at"] = json!(now_iso());
                    state["heartbeat"]["last_tick_ms"] = json!(now_ms);
                    let jitter_offset = deterministic_jitter_ms(cycles, jitter_ms);
                    state["heartbeat"]["next_tick_after_ms"] =
                        json!(now_ms.saturating_add(tick_ms).saturating_add(jitter_offset));
                    state["last_decision"] = if auto {
                        json!("cycle_executed_auto")
                    } else {
                        json!("cycle_executed_intent_only")
                    };
                    state["last_blocking_budget_used_ms"] = json!(blocking_used_ms);
                    cycle_log_row = json!({
                        "type": "proactive_daemon_tick",
                        "ts": now_iso(),
                        "action": action,
                        "auto": auto,
                        "brief_mode": brief_mode,
                        "intents": intents,
                        "executed": executed,
                        "deferred": deferred,
                        "blocking_budget_ms": blocking_budget_ms,
                        "blocking_used_ms": blocking_used_ms,
                        "window_sec": window_sec,
                        "max_proactive": max_messages,
                        "state_hash": receipt_hash(&state)
                    });
                }
            } else {
                state["last_decision"] = json!("paused_skip");
            }
        }
        _ => {}
    }
    state["updated_at"] = json!(now_iso());
    if cycle_log_row != Value::Null {
        if let Err(err) = append_proactive_daemon_log(root, &cycle_log_row, strict) {
            let mut out = cli_error_receipt(argv, &format!("proactive_daemon_log_failed:{err}"), 2);
            out["type"] = json!("autonomy_proactive_daemon");
            return emit_receipt(root, &mut out);
        }
    }
    if let Err(err) = persist_proactive_daemon_state(root, &mut state, strict) {
        let mut out = cli_error_receipt(argv, &format!("proactive_daemon_state_persist_failed:{err}"), 2);
        out["type"] = json!("autonomy_proactive_daemon");
        return emit_receipt(root, &mut out);
    }
    let mut out = json!({
        "ok": true,
        "type": "autonomy_proactive_daemon",
        "lane": LANE_ID,
        "strict": strict,
        "action": action,
        "state": state,
        "policy": {
            "tick_ms": tick_ms,
            "jitter_ms": jitter_ms,
            "window_sec": window_sec,
            "max_proactive": max_messages,
            "blocking_budget_ms": blocking_budget_ms,
            "brief_mode": brief_mode
        },
        "claim_evidence": [
            {"id":"V6-AUTONOMY-003.1","claim":"proactive_daemon_background_daemon_tracks_runtime_state_and_receipts_actions"},
            {"id":"V6-AUTONOMY-003.2","claim":"proactive_daemon_generates_proactive_micro_tasks_with_policy_bounded_auto_execution"},
            {"id":"V6-AUTONOMY-004","claim":"proactive_daemon_tick_heartbeat_rate_limits_blocking_budget_and_append_only_daily_logs_enforce_proactive_safety"}
        ]
    });
    emit_receipt(root, &mut out)
}
