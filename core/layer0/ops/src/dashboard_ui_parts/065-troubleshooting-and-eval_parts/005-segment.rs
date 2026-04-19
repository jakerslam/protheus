fn dashboard_troubleshooting_now_epoch_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_secs() as i64)
        .unwrap_or(0)
}

fn dashboard_payload_truthy_flag(payload: &Value, key: &str) -> bool {
    payload.get(key).is_some_and(|value| {
        value.as_bool().unwrap_or_else(|| {
            value
                .as_str()
                .map(|raw| {
                    let lowered = clean_text(raw, 24).to_ascii_lowercase();
                    matches!(lowered.as_str(), "1" | "true" | "yes" | "on")
                })
                .or_else(|| value.as_i64().map(|raw| raw != 0))
                .unwrap_or(false)
        })
    })
}

fn dashboard_payload_string_list(payload: &Value, key: &str, max_items: usize, max_len: usize) -> Vec<String> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|raw| clean_text(raw, max_len))
                .filter(|raw| !raw.is_empty())
                .take(max_items)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn dashboard_troubleshooting_deadletter_reason(row: &Value) -> String {
    clean_text(
        row.get("reason")
            .and_then(Value::as_str)
            .or_else(|| row.pointer("/row/last_error/error").and_then(Value::as_str))
            .unwrap_or("unknown"),
        120,
    )
    .to_ascii_lowercase()
}

fn dashboard_troubleshooting_issue_request_signature(row: &Value) -> String {
    let issue_request = row
        .get("issue_request")
        .cloned()
        .or_else(|| row.pointer("/row/issue_request").cloned())
        .unwrap_or_else(|| json!({}));
    let stable = json!({
        "title": clean_text(issue_request.get("title").and_then(Value::as_str).unwrap_or(""), 180),
        "body": clean_text(issue_request.get("body").and_then(Value::as_str).unwrap_or(""), 2000),
        "source": clean_text(issue_request.get("source").and_then(Value::as_str).unwrap_or(""), 80),
        "owner": clean_text(issue_request.get("owner").and_then(Value::as_str).unwrap_or(""), 120),
        "repo": clean_text(issue_request.get("repo").and_then(Value::as_str).unwrap_or(""), 120)
    });
    crate::deterministic_receipt_hash(&stable)
}

fn dashboard_troubleshooting_deadletter_row_signature(row: &Value) -> String {
    let reason = dashboard_troubleshooting_deadletter_reason(row);
    let request_sig = dashboard_troubleshooting_issue_request_signature(row);
    crate::v8_kernel::sha256_hex_str(&format!("{}:{}", reason, request_sig))
}

fn dashboard_troubleshooting_append_deadletter_row(root: &Path, row: &Value) -> bool {
    let mut rows = dashboard_troubleshooting_read_deadletter_all(root);
    let signature = dashboard_troubleshooting_deadletter_row_signature(row);
    let already_exists = rows.iter().any(|existing| {
        dashboard_troubleshooting_deadletter_row_signature(existing) == signature
    });
    if already_exists {
        return false;
    }
    rows.push(row.clone());
    if rows.len() > DASHBOARD_TROUBLESHOOTING_MAX_DEADLETTER {
        let keep_from = rows.len() - DASHBOARD_TROUBLESHOOTING_MAX_DEADLETTER;
        rows = rows.split_off(keep_from);
    }
    dashboard_troubleshooting_write_deadletter_rows(root, &rows);
    true
}

fn dashboard_troubleshooting_reason_histogram(rows: &[Value]) -> Vec<Value> {
    let mut counts = HashMap::<String, i64>::new();
    for row in rows {
        let reason = dashboard_troubleshooting_deadletter_reason(row);
        *counts.entry(reason).or_insert(0) += 1;
    }
    dashboard_troubleshooting_sorted_histogram(counts, "reason")
}

fn dashboard_troubleshooting_outbox_error_bucket(row: &Value) -> String {
    let error = clean_text(
        row.get("error_bucket")
            .and_then(Value::as_str)
            .or_else(|| row.pointer("/last_error/error").and_then(Value::as_str))
            .unwrap_or("unknown"),
        120,
    )
    .to_ascii_lowercase();
    if error.is_empty() {
        "unknown".to_string()
    } else {
        error
    }
}

fn dashboard_troubleshooting_outbox_reason_histogram(rows: &[Value]) -> Vec<Value> {
    let mut counts = HashMap::<String, i64>::new();
    for row in rows {
        let bucket = dashboard_troubleshooting_outbox_error_bucket(row);
        *counts.entry(bucket).or_insert(0) += 1;
    }
    dashboard_troubleshooting_sorted_histogram(counts, "error_bucket")
}

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
            let quarantine_row = json!({
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

fn dashboard_troubleshooting_deadletter_requeue_lane(root: &Path, payload: &Value) -> LaneResult {
    let max_items = dashboard_payload_usize(payload, "max_items", 10, 1, 100);
    let mut deadletter = dashboard_troubleshooting_read_deadletter_all(root);
    if deadletter.is_empty() {
        return LaneResult {
            ok: true,
            status: 0,
            argv: vec!["dashboard.troubleshooting.deadletter.requeue".to_string()],
            payload: Some(json!({
                "ok": true,
                "type": "dashboard_troubleshooting_deadletter_requeue",
                "requeued_count": 0,
                "deadletter_depth_after": 0,
                "outbox_depth_after": dashboard_troubleshooting_read_issue_outbox(root).len()
            })),
        };
    }
    let mut outbox = dashboard_troubleshooting_read_issue_outbox(root);
    let outbox_capacity = DASHBOARD_TROUBLESHOOTING_MAX_OUTBOX.saturating_sub(outbox.len());
    let target = max_items.min(outbox_capacity);
    let dry_run = dashboard_payload_truthy_flag(payload, "dry_run")
        || dashboard_payload_truthy_flag(payload, "preview_only")
        || dashboard_payload_truthy_flag(payload, "preview");
    let selected_ids = dashboard_payload_string_list(payload, "item_ids", 100, 120);
    let selected_filter_applied = !selected_ids.is_empty();
    let mut outbox_signatures = outbox
        .iter()
        .map(dashboard_troubleshooting_issue_request_signature)
        .collect::<Vec<_>>();
    let mut requeued = Vec::<Value>::new();
    let mut keep = Vec::<Value>::new();
    let mut skipped_not_selected = 0usize;
    let mut skipped_duplicate = 0usize;
    if dry_run {
        let mut would_requeue = 0usize;
        let mut would_skip_duplicate = 0usize;
        let mut would_skip_not_selected = 0usize;
        for row in deadletter.iter().rev() {
            let issue_id = clean_text(
                row.pointer("/row/id").and_then(Value::as_str).unwrap_or(""),
                80,
            );
            if selected_filter_applied && !selected_ids.iter().any(|id| id == &issue_id) {
                would_skip_not_selected += 1;
                continue;
            }
            if let Some(issue_row) = row.get("row").cloned() {
                let issue_signature = dashboard_troubleshooting_issue_request_signature(&issue_row);
                if !issue_signature.is_empty()
                    && outbox_signatures.iter().any(|existing| existing == &issue_signature)
                {
                    would_skip_duplicate += 1;
                    continue;
                }
                if would_requeue < target {
                    would_requeue += 1;
                }
            }
        }
        return LaneResult {
            ok: true,
            status: 0,
            argv: vec!["dashboard.troubleshooting.deadletter.requeue".to_string()],
            payload: Some(json!({
                "ok": true,
                "type": "dashboard_troubleshooting_deadletter_requeue_preview",
                "dry_run": true,
                "selected_filter_applied": selected_filter_applied,
                "selected_item_count": selected_ids.len(),
                "target_capacity": target,
                "would_requeue_count": would_requeue,
                "would_skip_not_selected_count": would_skip_not_selected,
                "would_skip_duplicate_count": would_skip_duplicate,
                "deadletter_depth": deadletter.len(),
                "outbox_depth": outbox.len()
            })),
        };
    }

    for row in deadletter.into_iter().rev() {
        let issue_id = clean_text(
            row.pointer("/row/id").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        if selected_filter_applied && !selected_ids.iter().any(|id| id == &issue_id) {
            skipped_not_selected += 1;
            keep.push(row);
            continue;
        }
        if requeued.len() < target {
            if let Some(issue_row) = row.get("row").cloned() {
                let issue_signature = dashboard_troubleshooting_issue_request_signature(&issue_row);
                if !issue_signature.is_empty()
                    && outbox_signatures.iter().any(|existing| existing == &issue_signature)
                {
                    skipped_duplicate += 1;
                    keep.push(row);
                    continue;
                }
                let mut restored = issue_row;
                if let Some(obj) = restored.as_object_mut() {
                    obj.insert("attempts".to_string(), json!(0));
                    obj.insert("retry_after_seconds".to_string(), json!(0));
                    obj.insert("next_retry_after_epoch_s".to_string(), json!(0));
                    obj.insert("last_requeued_at".to_string(), json!(now_iso()));
                }
                outbox.push(restored.clone());
                if !issue_signature.is_empty() {
                    outbox_signatures.push(issue_signature);
                }
                requeued.push(json!({
                    "item_id": clean_text(
                        restored.get("id").and_then(Value::as_str).unwrap_or(""),
                        80
                    ),
                    "status": "requeued"
                }));
                continue;
            }
        }
        keep.push(row);
    }

    keep.reverse();
    dashboard_troubleshooting_write_deadletter_rows(root, &keep);
    dashboard_troubleshooting_write_issue_outbox(root, &outbox);

    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.deadletter.requeue".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_deadletter_requeue",
            "selected_filter_applied": selected_filter_applied,
            "selected_item_count": selected_ids.len(),
            "skipped_not_selected_count": skipped_not_selected,
            "skipped_duplicate_count": skipped_duplicate,
            "requeued_count": requeued.len(),
            "deadletter_depth_after": keep.len(),
            "outbox_depth_after": outbox.len(),
            "deadletter_reason_histogram_after": dashboard_troubleshooting_reason_histogram(&keep),
            "outbox_error_histogram_after": dashboard_troubleshooting_outbox_reason_histogram(&outbox),
            "requeued": requeued
        })),
    }
}

fn dashboard_troubleshooting_deadletter_purge_lane(root: &Path, payload: &Value) -> LaneResult {
    let remove_all = dashboard_payload_truthy_flag(payload, "all");
    let reason_filter = clean_text(
        payload.get("reason").and_then(Value::as_str).unwrap_or(""),
        120,
    )
    .to_ascii_lowercase();
    let selected_ids = dashboard_payload_string_list(payload, "item_ids", 300, 120);
    let dry_run = dashboard_payload_truthy_flag(payload, "dry_run")
        || dashboard_payload_truthy_flag(payload, "preview_only")
        || dashboard_payload_truthy_flag(payload, "preview");
    if !remove_all && reason_filter.is_empty() && selected_ids.is_empty() {
        return LaneResult {
            ok: true,
            status: 0,
            argv: vec!["dashboard.troubleshooting.deadletter.purge".to_string()],
            payload: Some(json!({
                "ok": false,
                "type": "dashboard_troubleshooting_deadletter_purge",
                "error": "deadletter_purge_selector_required",
                "summary": "Specify all=true, reason, or item_ids for deadletter purge."
            })),
        };
    }
    let mut rows = dashboard_troubleshooting_read_deadletter_all(root);
    let mut removed = Vec::<Value>::new();
    let mut keep = Vec::<Value>::new();
    for row in rows.drain(..) {
        let issue_id = clean_text(
            row.pointer("/row/id").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        let row_reason = dashboard_troubleshooting_deadletter_reason(&row);
        let reason_match = !reason_filter.is_empty() && row_reason == reason_filter;
        let id_match = !selected_ids.is_empty() && selected_ids.iter().any(|id| id == &issue_id);
        let should_remove = remove_all || reason_match || id_match;
        if should_remove {
            removed.push(row);
        } else {
            keep.push(row);
        }
    }
    if dry_run {
        return LaneResult {
            ok: true,
            status: 0,
            argv: vec!["dashboard.troubleshooting.deadletter.purge".to_string()],
            payload: Some(json!({
                "ok": true,
                "type": "dashboard_troubleshooting_deadletter_purge_preview",
                "dry_run": true,
                "all": remove_all,
                "reason_filter": reason_filter,
                "selected_item_count": selected_ids.len(),
                "removed_count": removed.len(),
                "remaining_depth": keep.len(),
                "removed_reason_histogram": dashboard_troubleshooting_reason_histogram(&removed),
                "remaining_reason_histogram": dashboard_troubleshooting_reason_histogram(&keep)
            })),
        };
    }
    dashboard_troubleshooting_write_deadletter_rows(root, &keep);
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.deadletter.purge".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_deadletter_purge",
            "all": remove_all,
            "reason_filter": reason_filter,
            "selected_item_count": selected_ids.len(),
            "removed_count": removed.len(),
            "remaining_depth": keep.len(),
            "removed_reason_histogram": dashboard_troubleshooting_reason_histogram(&removed),
            "remaining_reason_histogram": dashboard_troubleshooting_reason_histogram(&keep)
        })),
    }
}

fn dashboard_troubleshooting_report_message_lane(root: &Path, payload: &Value) -> LaneResult {
    let (eval_model, _) = dashboard_troubleshooting_resolve_eval_model(Some(payload));
    let snapshot = dashboard_troubleshooting_capture_snapshot(
        root,
        "user_report",
        &json!({
            "source": clean_text(payload.get("source").and_then(Value::as_str).unwrap_or("dashboard_report_message"), 80),
            "session_id": clean_text(payload.get("session_id").or_else(|| payload.get("sessionId")).and_then(Value::as_str).unwrap_or(""), 160),
            "message_id": clean_text(payload.get("message_id").or_else(|| payload.get("messageId")).and_then(Value::as_str).unwrap_or(""), 160)
        }),
    );
    let queue_item = dashboard_troubleshooting_enqueue_eval(
        root,
        &snapshot,
        "user_report",
        Some(&eval_model),
    );
    let eval_drain = dashboard_troubleshooting_eval_drain_internal(root, 1, "user_report");
    let eval_report = eval_drain
        .get("reports")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .cloned()
        .or_else(|| read_json_file(&root.join(DASHBOARD_TROUBLESHOOTING_EVAL_LATEST_REL)))
        .unwrap_or_else(|| json!({}));
    let issue_request = dashboard_troubleshooting_issue_request_from_report(payload, &snapshot, &eval_report);
    let issue_lane = run_action(root, "dashboard.github.issue.create", &issue_request);
    if issue_lane.ok {
        dashboard_troubleshooting_clear_active_context(root, "issue_submission_succeeded");
        return LaneResult {
            ok: true,
            status: 0,
            argv: vec!["dashboard.troubleshooting.report_message".to_string()],
            payload: Some(json!({
                "ok": true,
                "type": "dashboard_troubleshooting_report",
                "submitted": true,
                "queued": false,
                "snapshot_id": snapshot.get("snapshot_id").cloned().unwrap_or(Value::Null),
                "eval_report_id": eval_report.get("report_id").cloned().unwrap_or(Value::Null),
                "issue": issue_lane.payload.unwrap_or_else(|| json!({}))
            })),
        };
    }
    let outbox_item =
        dashboard_troubleshooting_enqueue_outbox(root, &issue_request, &issue_lane, &snapshot, &eval_report);
    let issue_error = issue_lane
        .payload
        .as_ref()
        .and_then(|row| row.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("github_issue_transport_error")
        .to_string();
    let issue_error_hint = if issue_error == "github_issue_auth_missing" {
        "no github auth token, please input your token first"
    } else {
        "Issue pipeline queued locally; run dashboard.troubleshooting.outbox.flush after auth/pipeline recovery."
    };
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.report_message".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_report",
            "submitted": false,
            "queued": true,
            "snapshot_id": snapshot.get("snapshot_id").cloned().unwrap_or(Value::Null),
            "eval_report_id": eval_report.get("report_id").cloned().unwrap_or(Value::Null),
            "queue_item": queue_item,
            "eval_drain": eval_drain,
            "issue_error": issue_error,
            "issue_error_hint": issue_error_hint,
            "outbox_item": outbox_item
        })),
    }
}
