fn out_or_default(out: &Value, key: &str, default: Value) -> Value {
    out.get(key).cloned().unwrap_or(default)
}

fn event_or_default(event: &Value, key: &str, default: Value) -> Value {
    event.get(key).cloned().unwrap_or(default)
}

fn zero_number_value() -> Value {
    Value::Number(serde_json::Number::from_f64(0.0).unwrap_or(0.into()))
}

fn queue_utilization(queue_depth: usize, contract: &AttentionContract) -> f64 {
    let capacity = contract.max_queue_depth.max(1);
    (queue_depth as f64 / capacity as f64).clamp(0.0, 1.0)
}

fn backpressure_level(queue_depth: usize, contract: &AttentionContract) -> &'static str {
    if queue_depth >= contract.backpressure_hard_watermark {
        "critical"
    } else if queue_depth >= contract.backpressure_soft_watermark {
        "high"
    } else {
        "normal"
    }
}

fn backpressure_snapshot(queue_depth: usize, contract: &AttentionContract) -> Value {
    json!({
        "level": backpressure_level(queue_depth, contract),
        "queue_depth": queue_depth,
        "queue_utilization": queue_utilization(queue_depth, contract),
        "thresholds": {
            "soft_watermark": contract.backpressure_soft_watermark,
            "hard_watermark": contract.backpressure_hard_watermark,
            "max_queue_depth": contract.max_queue_depth
        }
    })
}

fn compact(root: &Path, flags: &BTreeMap<String, String>) -> i32 {
    let contract = load_contract(root);
    let run_context = flags
        .get("run-context")
        .cloned()
        .unwrap_or_else(|| "compact".to_string());
    let retain = parse_non_negative_limit(
        flags
            .get("retain")
            .or_else(|| flags.get("retain-acked"))
            .or_else(|| flags.get("retain_acked")),
        32,
        8_192,
    );
    let min_acked = parse_non_negative_limit(
        flags.get("min-acked").or_else(|| flags.get("min_acked")),
        1,
        1_000_000,
    );
    let (active_rows, expired_pruned) = load_active_queue(&contract);
    let queue_depth_before = active_rows.len();
    let mut cursor_state = load_cursor_state(&contract.cursor_state_path);
    let mut offsets_before = BTreeMap::<String, usize>::new();
    let mut offsets_after = BTreeMap::<String, usize>::new();
    let mut min_offset = queue_depth_before;

    if let Some(consumers) = cursor_state.get("consumers").and_then(Value::as_object) {
        for (consumer_id, state) in consumers {
            let offset = state
                .get("offset")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .min(queue_depth_before as u64) as usize;
            offsets_before.insert(consumer_id.clone(), offset);
            min_offset = min_offset.min(offset);
        }
    }
    if offsets_before.is_empty() {
        min_offset = 0;
    }
    let compact_count = if min_offset >= min_acked && min_offset > retain {
        min_offset.saturating_sub(retain)
    } else {
        0
    };
    let mut queue_depth_after = queue_depth_before;
    if compact_count > 0 {
        let kept_rows = active_rows
            .into_iter()
            .skip(compact_count.min(queue_depth_before))
            .collect::<Vec<_>>();
        queue_depth_after = kept_rows.len();
        write_jsonl(&contract.queue_path, &kept_rows);

        if let Some(consumers) = cursor_state
            .get_mut("consumers")
            .and_then(Value::as_object_mut)
        {
            for (consumer_id, state) in consumers.iter_mut() {
                let old_offset = state
                    .get("offset")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    .min(queue_depth_before as u64) as usize;
                let next_offset = old_offset
                    .saturating_sub(compact_count)
                    .min(queue_depth_after);
                state["offset"] = Value::Number((next_offset as u64).into());
                state["acked_at"] = Value::String(now_iso());
                state["run_context"] = Value::String(run_context.clone());
                offsets_after.insert(consumer_id.clone(), next_offset);
            }
        }
        cursor_state["updated_at"] = Value::String(now_iso());
        persist_cursor_state(&contract.cursor_state_path, &cursor_state);
    } else {
        offsets_after = offsets_before.clone();
    }

    let mut out = json!({
        "ok": true,
        "type": "attention_queue_compact",
        "ts": now_iso(),
        "run_context": run_context,
        "retain": retain,
        "min_acked": min_acked,
        "compacted_count": compact_count,
        "queue_depth_before": queue_depth_before,
        "queue_depth_after": queue_depth_after,
        "expired_pruned": expired_pruned,
        "backpressure_before": backpressure_snapshot(queue_depth_before, &contract),
        "backpressure_after": backpressure_snapshot(queue_depth_after, &contract),
        "min_consumer_offset": min_offset,
        "consumer_offsets_before": offsets_before,
        "consumer_offsets_after": offsets_after,
        "attention_contract": contract_snapshot(&contract),
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    append_jsonl(
        &contract.receipts_path,
        &json!({
            "ts": now_iso(),
            "type": "attention_queue_compact",
            "run_context": out_or_default(&out, "run_context", Value::String("compact".to_string())),
            "retain": out_or_default(&out, "retain", Value::Number(0.into())),
            "min_acked": out_or_default(&out, "min_acked", Value::Number(0.into())),
            "compacted_count": out_or_default(&out, "compacted_count", Value::Number(0.into())),
            "queue_depth_before": out_or_default(&out, "queue_depth_before", Value::Number(0.into())),
            "queue_depth_after": out_or_default(&out, "queue_depth_after", Value::Number(0.into())),
            "backpressure_before": out_or_default(&out, "backpressure_before", Value::Null),
            "backpressure_after": out_or_default(&out, "backpressure_after", Value::Null),
            "receipt_hash": out_or_default(&out, "receipt_hash", Value::String("".to_string())),
        }),
    );
    emit(&out);
    0
}

fn enqueue(root: &Path, flags: &BTreeMap<String, String>) -> i32 {
    let contract = load_contract(root);
    let run_context = flags
        .get("run-context")
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    let event_raw = match parse_event(flags) {
        Ok(v) => v,
        Err(reason) => {
            let mut out = json!({
                "ok": false,
                "type": "attention_queue_enqueue_error",
                "ts": now_iso(),
                "reason": reason,
                "run_context": run_context,
                "attention_contract": contract_snapshot(&contract)
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            emit(&out);
            return 2;
        }
    };

    let event = match normalize_event(&event_raw, &contract) {
        Ok(row) => row,
        Err(reason) => {
            let mut out = json!({
                "ok": false,
                "type": "attention_queue_enqueue_error",
                "ts": now_iso(),
                "reason": reason,
                "run_context": run_context,
                "attention_contract": contract_snapshot(&contract)
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            append_jsonl(
                &contract.receipts_path,
                &json!({
                    "ts": now_iso(),
                    "type": "attention_receipt",
                    "decision": "rejected_layer2_authority_unavailable",
                    "queued": false,
                    "run_context": run_context,
                    "reason": out.get("reason").cloned().unwrap_or(Value::String("layer2_priority_authority_unavailable".to_string())),
                    "receipt_hash": out.get("receipt_hash").cloned().unwrap_or(Value::String("".to_string()))
                }),
            );
            emit(&out);
            return 2;
        }
    };
    let queue_depth_before;
    let queue_depth_after;
    let action;
    let queued;

    let mut active_rows = Vec::new();
    let mut expired_pruned = 0usize;
    if contract.enabled && contract.push_attention_queue {
        let rows = read_jsonl(&contract.queue_path);
        let (pruned, dropped) = prune_expired(rows);
        active_rows = pruned;
        expired_pruned = dropped;
    }
    queue_depth_before = active_rows.len();

    let deduped = dedupe_hit(&active_rows, &event, contract.dedupe_window_hours);
    if !contract.enabled || !contract.push_attention_queue {
        action = "disabled".to_string();
        queued = false;
        queue_depth_after = queue_depth_before;
    } else if deduped {
        action = "deduped".to_string();
        queued = false;
        queue_depth_after = queue_depth_before;
    } else {
        let drop_rank = severity_rank(&contract.backpressure_drop_below);
        let severity = event
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("info");
        let queue_lane = event
            .get("queue_lane")
            .and_then(Value::as_str)
            .unwrap_or("standard");
        let sev_rank = severity_rank(severity);
        let event_band = event.get("band").and_then(Value::as_str).unwrap_or("p4");
        let high_importance = band_rank(event_band) >= band_rank("p2");
        let _at_or_over_soft = queue_depth_before >= contract.backpressure_soft_watermark;
        let at_or_over_hard = queue_depth_before >= contract.backpressure_hard_watermark;
        let should_drop_for_backpressure = at_or_over_hard
            && (queue_lane.eq_ignore_ascii_case("background")
                || (sev_rank < drop_rank && !high_importance));
        if should_drop_for_backpressure {
            action = "dropped_backpressure".to_string();
            queued = false;
            queue_depth_after = queue_depth_before;
        } else {
            action = if high_importance {
                "admitted_priority".to_string()
            } else {
                "admitted".to_string()
            };
            queued = true;
            active_rows.push(event.clone());
            sort_active_rows_with_authority(&mut active_rows);
            write_jsonl(&contract.queue_path, &active_rows);
            queue_depth_after = active_rows.len();
        }
    }

    let latest = update_latest(
        &contract,
        &action,
        queue_depth_after,
        if queued { Some(&event) } else { None },
        expired_pruned,
    );
    let queue_utilization_before = queue_utilization(queue_depth_before, &contract);
    let queue_utilization_after = queue_utilization(queue_depth_after, &contract);
    let backpressure_level_before = backpressure_level(queue_depth_before, &contract);
    let backpressure_level_after = backpressure_level(queue_depth_after, &contract);
    let event_severity = event
        .get("severity")
        .and_then(Value::as_str)
        .unwrap_or("info");
    let event_band = event.get("band").and_then(Value::as_str).unwrap_or("p4");
    let event_queue_lane = event
        .get("queue_lane")
        .and_then(Value::as_str)
        .unwrap_or("standard");
    let event_severity_rank = severity_rank(event_severity);
    let drop_threshold_rank = severity_rank(&contract.backpressure_drop_below);
    let event_high_importance = band_rank(event_band) >= band_rank("p2");
    let at_or_over_soft = queue_depth_before >= contract.backpressure_soft_watermark;
    let at_or_over_hard = queue_depth_before >= contract.backpressure_hard_watermark;
    let drop_candidate = at_or_over_hard
        && (event_queue_lane.eq_ignore_ascii_case("background")
            || (event_severity_rank < drop_threshold_rank && !event_high_importance));
    let backpressure_policy_action = if action == "dropped_backpressure" {
        "drop"
    } else if at_or_over_soft {
        "admit_under_pressure"
    } else {
        "admit"
    };

    let mut receipt = json!({
        "ok": true,
        "type": "attention_queue_enqueue",
        "ts": now_iso(),
        "decision": action,
        "queued": queued,
        "run_context": run_context,
        "queue_depth_before": queue_depth_before,
        "queue_depth_after": queue_depth_after,
        "expired_pruned": expired_pruned,
        "backpressure": {
            "level_before": backpressure_level_before,
            "level_after": backpressure_level_after,
            "queue_utilization_before": queue_utilization_before,
            "queue_utilization_after": queue_utilization_after,
            "at_or_over_soft": at_or_over_soft,
            "at_or_over_hard": at_or_over_hard,
            "drop_candidate": drop_candidate,
            "policy_action": backpressure_policy_action,
            "drop_below": contract.backpressure_drop_below,
            "thresholds": {
                "soft_watermark": contract.backpressure_soft_watermark,
                "hard_watermark": contract.backpressure_hard_watermark,
                "max_queue_depth": contract.max_queue_depth
            },
            "event_ranking": {
                "severity": event_severity,
                "severity_rank": event_severity_rank,
                "drop_threshold_rank": drop_threshold_rank,
                "queue_lane": event_queue_lane,
                "band": event_band,
                "high_importance": event_high_importance
            }
        },
        "attention_contract": contract_snapshot(&contract),
        "event": {
            "source": event_or_default(&event, "source", Value::String("unknown_source".to_string())),
            "source_type": event_or_default(&event, "source_type", Value::String("unknown_type".to_string())),
            "severity": event_or_default(&event, "severity", Value::String("info".to_string())),
            "priority": event_or_default(&event, "priority", Value::Number(20.into())),
            "score": event_or_default(&event, "score", zero_number_value()),
            "band": event_or_default(&event, "band", Value::String("p4".to_string())),
            "queue_lane": event_or_default(&event, "queue_lane", Value::String("standard".to_string())),
            "summary": event_or_default(&event, "summary", Value::String("attention_event".to_string())),
            "attention_key": event_or_default(&event, "attention_key", Value::String("".to_string())),
            "escalate_required": event_or_default(&event, "escalate_required", Value::Bool(false)),
            "initiative_action": event_or_default(&event, "initiative_action", Value::String("silent".to_string()))
        },
        "latest": latest
    });
    if queued {
        receipt["queued_event"] = event.clone();
    }
    receipt["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&receipt));

    append_jsonl(
        &contract.receipts_path,
        &json!({
            "ts": now_iso(),
            "type": "attention_receipt",
            "decision": action,
            "queued": queued,
            "queue_depth_before": queue_depth_before,
            "queue_depth_after": queue_depth_after,
            "expired_pruned": expired_pruned,
            "backpressure_level_before": backpressure_level_before,
            "backpressure_level_after": backpressure_level_after,
            "queue_utilization_before": queue_utilization_before,
            "queue_utilization_after": queue_utilization_after,
            "backpressure_policy_action": backpressure_policy_action,
            "backpressure_drop_candidate": drop_candidate,
            "backpressure_threshold_soft": contract.backpressure_soft_watermark,
            "backpressure_threshold_hard": contract.backpressure_hard_watermark,
            "backpressure_drop_below": contract.backpressure_drop_below,
            "severity": event_or_default(&event, "severity", Value::String("info".to_string())),
            "priority": event_or_default(&event, "priority", Value::Number(20.into())),
            "score": event_or_default(&event, "score", zero_number_value()),
            "band": event_or_default(&event, "band", Value::String("p4".to_string())),
            "queue_lane": event_or_default(&event, "queue_lane", Value::String("standard".to_string())),
            "attention_key": event_or_default(&event, "attention_key", Value::String("".to_string())),
            "escalate_required": event_or_default(&event, "escalate_required", Value::Bool(false)),
            "initiative_action": event_or_default(&event, "initiative_action", Value::String("silent".to_string())),
            "run_context": run_context,
            "receipt_hash": out_or_default(&receipt, "receipt_hash", Value::String("".to_string()))
        }),
    );

    emit(&receipt);
    if queued || action == "deduped" || action == "disabled" {
        0
    } else {
        2
    }
}

fn status(root: &Path) -> i32 {
    let contract = load_contract(root);
    let (active_rows, expired_pruned) = load_active_queue(&contract);
    let mut lane_counts: BTreeMap<String, usize> = BTreeMap::new();
    for row in &active_rows {
        let lane = row
            .get("queue_lane")
            .and_then(Value::as_str)
            .unwrap_or("standard")
            .trim()
            .to_ascii_lowercase();
        let key = if lane == "critical" || lane == "background" {
            lane
        } else {
            "standard".to_string()
        };
        *lane_counts.entry(key).or_insert(0) += 1;
    }
    let latest = read_json(&contract.latest_path).unwrap_or_else(|| json!({}));
    let mut out = json!({
        "ok": true,
        "type": "attention_queue_status",
        "ts": now_iso(),
        "queue_depth": active_rows.len(),
        "lane_counts": lane_counts,
        "expired_pruned": expired_pruned,
        "backpressure": backpressure_snapshot(active_rows.len(), &contract),
        "attention_contract": contract_snapshot(&contract),
        "latest": latest
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    emit(&out);
    0
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() {
        usage();
        return 2;
    }
    let command = argv[0].trim().to_ascii_lowercase();
    let flags = parse_cli_flags(&argv[1..]);
    match command.as_str() {
        "enqueue" => enqueue(root, &flags),
        "status" => status(root),
        "next" => next(root, &flags, false),
        "ack" => ack(root, &flags),
        "drain" => next(root, &flags, true),
        "compact" => compact(root, &flags),
        _ => {
            usage();
            let mut out = json!({
                "ok": false,
                "type": "attention_queue_cli_error",
                "ts": now_iso(),
                "reason": "unknown_command",
                "command": command
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            emit(&out);
            2
        }
    }
}
