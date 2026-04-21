fn dashboard_troubleshooting_deadletter_requeue_lane(root: &Path, payload: &Value) -> LaneResult {
    let max_items = dashboard_payload_usize(payload, "max_items", 10, 1, 100);
    let deadletter = dashboard_troubleshooting_read_deadletter_all(root);
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
    let (eval_model, _) = dashboard_troubleshooting_resolve_eval_model(Some(root), Some(payload));
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
