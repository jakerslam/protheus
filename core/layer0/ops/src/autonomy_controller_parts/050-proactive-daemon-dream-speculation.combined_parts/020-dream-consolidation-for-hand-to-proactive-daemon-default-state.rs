
fn run_dream_consolidation_for_hand(root: &Path, hand_id: &str) -> Result<Value, String> {
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
    write_json(&path, &hand).map_err(|err| format!("dream_write_failed:{err}"))?;

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
    append_jsonl(&dream_events_path(root), &event)
        .map_err(|err| format!("dream_event_append_failed:{err}"))?;
    Ok(event)
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
    let event = match run_dream_consolidation_for_hand(root, &hand_id) {
        Ok(event) => event,
        Err(err) => {
            let mut out = cli_error_receipt(argv, &err, 2);
            out["type"] = json!("autonomy_dream_consolidation");
            return emit_receipt(root, &mut out);
        }
    };
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

fn parse_iso_epoch_ms(raw: &str) -> Option<u64> {
    let parsed = chrono::DateTime::parse_from_rfc3339(raw.trim()).ok()?;
    let ms = parsed.timestamp_millis();
    if ms <= 0 {
        None
    } else {
        Some(ms as u64)
    }
}

fn value_epoch_ms(row: Option<&Value>) -> Option<u64> {
    match row {
        Some(Value::Number(num)) => num.as_u64(),
        Some(Value::String(text)) => parse_iso_epoch_ms(text),
        _ => None,
    }
}

fn file_modified_epoch_ms(path: &Path) -> Option<u64> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|value| value.as_millis() as u64)
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
