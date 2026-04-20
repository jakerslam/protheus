fn dashboard_troubleshooting_read_deadletter_rows(root: &Path, limit: usize) -> Vec<Value> {
    let path = root.join(DASHBOARD_TROUBLESHOOTING_ISSUE_DEADLETTER_REL);
    let raw = fs::read_to_string(path).unwrap_or_default();
    if raw.is_empty() {
        return Vec::new();
    }
    let mut rows = Vec::<Value>::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            rows.push(value);
        }
    }
    if rows.len() > limit {
        rows.split_off(rows.len().saturating_sub(limit))
    } else {
        rows
    }
}

fn dashboard_troubleshooting_read_deadletter_all(root: &Path) -> Vec<Value> {
    dashboard_troubleshooting_read_deadletter_rows(root, usize::MAX)
}

fn dashboard_troubleshooting_write_deadletter_rows(root: &Path, rows: &[Value]) {
    let path = root.join(DASHBOARD_TROUBLESHOOTING_ISSUE_DEADLETTER_REL);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if rows.is_empty() {
        let _ = fs::write(path, "");
        return;
    }
    let mut out = String::new();
    for row in rows {
        if let Ok(line) = serde_json::to_string(row) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    let _ = fs::write(path, out);
}

fn dashboard_troubleshooting_outbox_flush_lane(root: &Path, payload: &Value) -> LaneResult {
    let max_items = dashboard_payload_usize(payload, "max_items", 10, 1, 50);
    let force = dashboard_payload_truthy_flag(payload, "force")
        || dashboard_payload_truthy_flag(payload, "ignore_cooldown");
    let dry_run = dashboard_payload_truthy_flag(payload, "dry_run")
        || dashboard_payload_truthy_flag(payload, "preview_only")
        || dashboard_payload_truthy_flag(payload, "preview");
    let max_attempts = dashboard_payload_usize(payload, "max_attempts", 6, 1, 20) as i64;
    let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
    let mut items = dashboard_troubleshooting_read_issue_outbox(root);
    if dry_run {
        let mut ready = Vec::<Value>::new();
        let mut cooldown_blocked = Vec::<Value>::new();
        let mut max_attempt_blocked = Vec::<Value>::new();
        for row in items.iter() {
            if ready.len() + cooldown_blocked.len() + max_attempt_blocked.len() >= max_items {
                break;
            }
            let attempts = row.get("attempts").and_then(Value::as_i64).unwrap_or(0);
            if attempts >= max_attempts {
                max_attempt_blocked.push(json!({
                    "item_id": clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
                    "attempts": attempts,
                    "status": "would_quarantine_max_attempts"
                }));
                continue;
            }
            let next_retry_epoch = row
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            if !force && next_retry_epoch > now_epoch {
                cooldown_blocked.push(json!({
                    "item_id": clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
                    "next_retry_after_epoch_s": next_retry_epoch,
                    "next_retry_after_seconds": next_retry_epoch - now_epoch,
                    "status": "cooldown_blocked"
                }));
                continue;
            }
            ready.push(json!({
                "item_id": clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
                "attempts": attempts,
                "status": "ready"
            }));
        }
        return LaneResult {
            ok: true,
            status: 0,
            argv: vec!["dashboard.troubleshooting.outbox.flush".to_string()],
            payload: Some(json!({
                "ok": true,
                "type": "dashboard_troubleshooting_outbox_flush_preview",
                "dry_run": true,
                "force": force,
                "max_attempts": max_attempts,
                "ready_count": ready.len(),
                "cooldown_blocked_count": cooldown_blocked.len(),
                "max_attempt_blocked_count": max_attempt_blocked.len(),
                "remaining_depth": items.len(),
                "error_histogram": dashboard_troubleshooting_outbox_reason_histogram(&items),
                "ready": ready,
                "cooldown_blocked": cooldown_blocked,
                "max_attempt_blocked": max_attempt_blocked
            })),
        };
    }
    let mut remaining = Vec::<Value>::new();
    let mut submitted = Vec::<Value>::new();
    let mut failed = Vec::<Value>::new();
    let mut quarantined = Vec::<Value>::new();
    let mut skipped_due_cooldown = 0usize;
    let mut auth_blocked_count = 0usize;
    let mut transport_failed_count = 0usize;
    let mut validation_failed_count = 0usize;
    let mut failed_error_counts = HashMap::<String, i64>::new();
    for row in items.drain(..) {
        if submitted.len() + failed.len() >= max_items {
            remaining.push(row);
            continue;
        }
        let attempts = row.get("attempts").and_then(Value::as_i64).unwrap_or(0);
        if attempts >= max_attempts {
            let mut quarantine_row = json!({
                "item_id": clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
                "attempts": attempts,
                "status": "quarantined_max_attempts"
            });
            let deadletter_inserted = dashboard_troubleshooting_append_deadletter_row(
                root,
                &json!({
                    "type": "dashboard_troubleshooting_issue_deadletter",
                    "ts": now_iso(),
                    "reason": "quarantined_max_attempts",
                    "row": row.clone()
                }),
            );
            *failed_error_counts
                .entry("quarantined_max_attempts".to_string())
                .or_insert(0) += 1;
            if let Some(obj) = quarantine_row.as_object_mut() {
                obj.insert("deadletter_inserted".to_string(), json!(deadletter_inserted));
            }
            quarantined.push(quarantine_row);
            continue;
        }
        let next_retry_epoch = row
            .get("next_retry_after_epoch_s")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if !force && next_retry_epoch > now_epoch {
            skipped_due_cooldown += 1;
            remaining.push(row);
            continue;
        }
        let request = row
            .get("issue_request")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let lane = run_action(root, "dashboard.github.issue.create", &request);
        if lane.ok {
            submitted.push(json!({
                "item_id": clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
                "issue": lane.payload.clone().unwrap_or_else(|| json!({}))
            }));
            dashboard_troubleshooting_clear_active_context(root, "issue_outbox_submission_succeeded");
            continue;
        }
        let issue_error = clean_text(
            lane.payload
                .as_ref()
                .and_then(|value| value.get("error"))
                .and_then(Value::as_str)
                .unwrap_or("github_issue_transport_error"),
            120,
        )
        .to_ascii_lowercase();
        *failed_error_counts.entry(issue_error.clone()).or_insert(0) += 1;
        let (error_bucket, retry_lane, retry_after_seconds) = if issue_error == "github_issue_auth_missing" {
            auth_blocked_count += 1;
            ("auth_missing", "auth_required", 3600)
        } else if issue_error.starts_with("github_issue_http_4") {
            validation_failed_count += 1;
            ("http_client", "request_fix_required", 900)
        } else if issue_error.starts_with("github_issue_http_5") {
            transport_failed_count += 1;
            ("http_server", "transport_retry", (attempts + 1) * 45)
        } else if issue_error == "github_issue_transport_error" {
            transport_failed_count += 1;
            ("transport", "transport_retry", (attempts + 1) * 45)
        } else {
            validation_failed_count += 1;
            ("other", "request_fix_required", 600)
        };
        let mut updated = row.clone();
        if let Some(obj) = updated.as_object_mut() {
            let attempts = obj.get("attempts").and_then(Value::as_i64).unwrap_or(0) + 1;
            let retry_after_seconds = retry_after_seconds.clamp(30, 3600);
            obj.insert("attempts".to_string(), json!(attempts));
            obj.insert("last_attempt_at".to_string(), json!(now_iso()));
            obj.insert(
                "retry_after_seconds".to_string(),
                json!(retry_after_seconds),
            );
            obj.insert(
                "next_retry_after_epoch_s".to_string(),
                json!(now_epoch + retry_after_seconds),
            );
            obj.insert(
                "last_error".to_string(),
                lane.payload.clone().unwrap_or_else(|| json!({})),
            );
            obj.insert("error_bucket".to_string(), json!(error_bucket));
            obj.insert("retry_lane".to_string(), json!(retry_lane));
        }
        failed.push(updated.clone());
        remaining.push(updated);
    }
    dashboard_troubleshooting_write_issue_outbox(root, &remaining);
    let next_retry_after_epoch_s = remaining
        .iter()
        .filter_map(|row| row.get("next_retry_after_epoch_s").and_then(Value::as_i64))
        .filter(|epoch| *epoch > now_epoch)
        .min()
        .unwrap_or(0);
    let next_retry_after_seconds = if next_retry_after_epoch_s > now_epoch {
        next_retry_after_epoch_s - now_epoch
    } else {
        0
    };
    let deadletter_depth = dashboard_troubleshooting_read_deadletter_rows(root, usize::MAX).len();
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.outbox.flush".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_outbox_flush",
            "force": force,
            "max_attempts": max_attempts,
            "skipped_due_cooldown_count": skipped_due_cooldown,
            "auth_blocked_count": auth_blocked_count,
            "transport_failed_count": transport_failed_count,
            "validation_failed_count": validation_failed_count,
            "submitted_count": submitted.len(),
            "failed_count": failed.len(),
            "quarantined_count": quarantined.len(),
            "remaining_depth": remaining.len(),
            "deadletter_depth": deadletter_depth,
            "next_retry_after_epoch_s": next_retry_after_epoch_s,
            "next_retry_after_seconds": next_retry_after_seconds,
            "failed_error_histogram": dashboard_troubleshooting_sorted_histogram(failed_error_counts, "error"),
            "submitted": submitted,
            "failed": failed,
            "quarantined": quarantined
        })),
    }
}

fn dashboard_troubleshooting_deadletter_state_lane(root: &Path, payload: &Value) -> LaneResult {
    let limit = dashboard_payload_usize(payload, "limit", 20, 1, 200);
    let reason_filter = clean_text(
        payload.get("reason").and_then(Value::as_str).unwrap_or(""),
        120,
    )
    .to_ascii_lowercase();
    let all_rows = dashboard_troubleshooting_read_deadletter_all(root);
    let mut rows = all_rows.clone();
    if !reason_filter.is_empty() {
        rows.retain(|row| dashboard_troubleshooting_deadletter_reason(row) == reason_filter);
    }
    if rows.len() > limit {
        rows = rows.split_off(rows.len().saturating_sub(limit));
    }
    let oldest_ts = all_rows
        .first()
        .and_then(|row| row.get("ts").and_then(Value::as_str))
        .map(|raw| clean_text(raw, 80))
        .unwrap_or_default();
    let latest_ts = all_rows
        .last()
        .and_then(|row| row.get("ts").and_then(Value::as_str))
        .map(|raw| clean_text(raw, 80))
        .unwrap_or_default();
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.deadletter.state".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_deadletter_state",
            "reason_filter": reason_filter,
            "depth": all_rows.len(),
            "filtered_depth": rows.len(),
            "oldest_ts": oldest_ts,
            "latest_ts": latest_ts,
            "reason_histogram": dashboard_troubleshooting_reason_histogram(&all_rows),
            "items": rows
        })),
    }
}
