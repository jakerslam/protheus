
fn append_event(
    conn: &Connection,
    queue_name: &str,
    lane_id: &str,
    event_type: &str,
    payload: &Value,
    ts: Option<&str>,
) -> Result<Value, String> {
    ensure_schema(conn)?;
    let normalized_queue = normalize_queue_name(queue_name);
    let normalized_lane = clean_lane_id(lane_id);
    let normalized_type = if event_type.trim().is_empty() {
        "event".to_string()
    } else {
        clean_text(Some(&Value::String(event_type.to_string())), 80)
    };
    let normalized_ts = if ts.unwrap_or("").trim().is_empty() {
        now_iso()
    } else {
        ts.unwrap().trim().to_string()
    };
    let payload_json = canonical_json(payload);
    let event_id = sha256_hex(&format!(
        "{}|{}|{}|{}|{}",
        normalized_queue, normalized_lane, normalized_type, payload_json, normalized_ts
    ));
    conn.execute(
        "INSERT OR IGNORE INTO backlog_queue_events (event_id, queue_name, lane_id, event_type, payload_json, ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            event_id,
            normalized_queue,
            if normalized_lane.is_empty() { None::<String> } else { Some(normalized_lane) },
            normalized_type,
            payload_json,
            normalized_ts
        ],
    )
    .map_err(|err| format!("queue_sqlite_kernel_insert_failed:{err}"))?;
    Ok(json!({ "ok": true, "event_id": event_id }))
}

fn insert_receipt(conn: &Connection, lane_id: &str, receipt: &Value) -> Result<Value, String> {
    ensure_schema(conn)?;
    let payload_json = canonical_json(receipt);
    let receipt_id = sha256_hex(&payload_json);
    let ts = clean_text(receipt.get("ts"), 120);
    let final_ts = if ts.is_empty() { now_iso() } else { ts };
    conn.execute(
        "INSERT OR REPLACE INTO backlog_queue_receipts (receipt_id, lane_id, receipt_json, ts) VALUES (?1, ?2, ?3, ?4)",
        params![receipt_id, clean_lane_id(lane_id), payload_json, final_ts],
    )
    .map_err(|err| format!("queue_sqlite_kernel_insert_failed:{err}"))?;
    Ok(json!({
        "ok": true,
        "receipt_id": receipt_id,
        "ts": final_ts
    }))
}

fn queue_stats(conn: &Connection, queue_name: &str) -> Result<Value, String> {
    ensure_schema(conn)?;
    let normalized_queue = normalize_queue_name(queue_name);
    let items: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM backlog_queue_items WHERE queue_name = ?1",
            params![normalized_queue.clone()],
            |row| row.get(0),
        )
        .map_err(|err| format!("queue_sqlite_kernel_query_failed:{err}"))?;
    let events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM backlog_queue_events WHERE queue_name = ?1",
            params![normalized_queue.clone()],
            |row| row.get(0),
        )
        .map_err(|err| format!("queue_sqlite_kernel_query_failed:{err}"))?;
    let receipts: i64 = conn
        .query_row("SELECT COUNT(*) FROM backlog_queue_receipts", [], |row| {
            row.get(0)
        })
        .map_err(|err| format!("queue_sqlite_kernel_query_failed:{err}"))?;
    Ok(json!({
        "ok": true,
        "queue_name": normalized_queue,
        "items": items,
        "events": events,
        "receipts": receipts
    }))
}

fn parse_watermark(payload: &Map<String, Value>, key: &str, fallback: i64) -> i64 {
    as_i64(payload.get(key), fallback).clamp(1, 5_000_000)
}

fn parse_policy_name(payload: &Map<String, Value>, key: &str, fallback: &str) -> String {
    let value = clean_text(payload.get(key), 64).to_ascii_lowercase();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

fn backpressure_policy(
    conn: &Connection,
    queue_name: &str,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    ensure_schema(conn)?;
    let normalized_queue = normalize_queue_name(queue_name);
    let observed_depth = as_i64(payload.get("depth_override"), -1);
    let depth = if observed_depth >= 0 {
        observed_depth
    } else {
        conn.query_row(
            "SELECT COUNT(*) FROM backlog_queue_items WHERE queue_name = ?1 AND status IN ('queued', 'staged', 'running')",
            params![normalized_queue.clone()],
            |row| row.get(0),
        )
        .map_err(|err| format!("queue_sqlite_kernel_query_failed:{err}"))?
    };

    let soft = parse_watermark(payload, "soft_watermark", 600);
    let hard = parse_watermark(payload, "hard_watermark", soft + 600).max(soft + 1);
    let quarantine = parse_watermark(payload, "quarantine_watermark", hard + 600).max(hard + 1);

    let defer_policy = parse_policy_name(payload, "defer_policy", "defer_noncritical");
    let shed_policy = parse_policy_name(payload, "shed_policy", "shed_low_priority");
    let quarantine_policy = parse_policy_name(payload, "quarantine_policy", "quarantine_new_work");

    let incoming_priority = clean_text(payload.get("incoming_priority"), 40).to_ascii_lowercase();
    let incoming_priority = if incoming_priority.is_empty() {
        "normal".to_string()
    } else {
        incoming_priority
    };
    let priority_aging_base = as_i64(payload.get("priority_aging_base"), 1).clamp(1, 8);
    let priority_aging_max = as_i64(payload.get("priority_aging_max"), 5).clamp(priority_aging_base, 16);

    let (pressure_state, action, reason_code, incoming_decision) = if depth >= quarantine {
        (
            "quarantine",
            quarantine_policy.clone(),
            "depth_ge_quarantine",
            "quarantine",
        )
    } else if depth >= hard {
        (
            "shed",
            shed_policy.clone(),
            "depth_ge_hard",
            if incoming_priority == "critical" {
                "defer"
            } else {
                "drop"
            },
        )
    } else if depth >= soft {
        (
            "defer",
            defer_policy.clone(),
            "depth_ge_soft",
            if incoming_priority == "critical" || incoming_priority == "high" {
                "admit"
            } else {
                "defer"
            },
        )
    } else {
        ("normal", "admit".to_string(), "depth_below_soft", "admit")
    };

    let pressure_ratio = if quarantine <= 0 {
        0.0
    } else {
        (depth as f64 / quarantine as f64).clamp(0.0, 3.0)
    };
    let priority_aging_multiplier = (priority_aging_base as f64 + pressure_ratio * 2.0)
        .ceil()
        .clamp(priority_aging_base as f64, priority_aging_max as f64) as i64;

    Ok(json!({
        "ok": true,
        "queue_name": normalized_queue,
        "observed_depth": depth,
        "thresholds": {
            "soft_watermark": soft,
            "hard_watermark": hard,
            "quarantine_watermark": quarantine
        },
        "pressure_state": pressure_state,
        "action": action,
        "reason_code": reason_code,
        "incoming_priority": incoming_priority,
        "incoming_decision": incoming_decision,
        "priority_aging_multiplier": priority_aging_multiplier,
        "policies": {
            "defer_policy": defer_policy,
            "shed_policy": shed_policy,
            "quarantine_policy": quarantine_policy
        },
        "explain": [
            format!("depth={depth}"),
            format!("soft={soft}"),
            format!("hard={hard}"),
            format!("quarantine={quarantine}"),
            format!("reason={reason_code}"),
            format!("incoming_decision={incoming_decision}")
        ]
    }))
}
