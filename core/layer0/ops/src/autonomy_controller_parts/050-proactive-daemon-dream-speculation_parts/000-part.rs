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

fn compaction_ratios(
    mode: &str,
    pressure_ratio: f64,
    auto_compact_pct: Option<f64>,
) -> (f64, f64, f64) {
    let mut ratios = match mode {
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
        ratios = (keep, keep, keep);
    }
    ratios
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

    let (core_ratio, archival_ratio, external_ratio) =
        compaction_ratios(mode, pressure_ratio, auto_compact_pct);

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
