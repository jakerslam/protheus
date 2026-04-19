fn dashboard_troubleshooting_outbox_flush_lane(root: &Path, payload: &Value) -> LaneResult {
    let max_items = dashboard_payload_usize(payload, "max_items", 10, 1, 50);
    let mut items = dashboard_troubleshooting_read_issue_outbox(root);
    let mut remaining = Vec::<Value>::new();
    let mut submitted = Vec::<Value>::new();
    let mut failed = Vec::<Value>::new();
    for row in items.drain(..) {
        if submitted.len() + failed.len() >= max_items {
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
        let mut updated = row.clone();
        if let Some(obj) = updated.as_object_mut() {
            let attempts = obj.get("attempts").and_then(Value::as_i64).unwrap_or(0) + 1;
            obj.insert("attempts".to_string(), json!(attempts));
            obj.insert("last_attempt_at".to_string(), json!(now_iso()));
            obj.insert(
                "last_error".to_string(),
                lane.payload.clone().unwrap_or_else(|| json!({})),
            );
        }
        failed.push(updated.clone());
        remaining.push(updated);
    }
    dashboard_troubleshooting_write_issue_outbox(root, &remaining);
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.outbox.flush".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_outbox_flush",
            "submitted_count": submitted.len(),
            "failed_count": failed.len(),
            "remaining_depth": remaining.len(),
            "submitted": submitted,
            "failed": failed
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
            "issue_error": issue_lane
                .payload
                .as_ref()
                .and_then(|row| row.get("error"))
                .cloned()
                .unwrap_or_else(|| json!("github_issue_transport_error")),
            "issue_error_hint": "Issue pipeline queued locally; run dashboard.troubleshooting.outbox.flush after auth/pipeline recovery.",
            "outbox_item": outbox_item
        })),
    }
}
