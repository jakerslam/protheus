fn dashboard_troubleshooting_issue_request_from_report(
    payload: &Value,
    snapshot: &Value,
    eval_report: &Value,
) -> Value {
    let source = clean_text(
        payload
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("dashboard_report_message"),
        80,
    );
    let title_hint = clean_text(
        payload
            .get("title")
            .and_then(Value::as_str)
            .or_else(|| eval_report.get("summary").and_then(Value::as_str))
            .unwrap_or("Dashboard troubleshooting report"),
        110,
    );
    let snapshot_id = clean_text(
        snapshot.get("snapshot_id").and_then(Value::as_str).unwrap_or("unknown"),
        80,
    );
    let report_id = clean_text(
        eval_report.get("report_id").and_then(Value::as_str).unwrap_or(""),
        80,
    );
    let eval_summary = clean_text(
        eval_report.get("summary").and_then(Value::as_str).unwrap_or(""),
        1600,
    );
    let eval_model = clean_text(
        eval_report
            .pointer("/eval/model")
            .and_then(Value::as_str)
            .unwrap_or(DASHBOARD_TROUBLESHOOTING_DEFAULT_EVAL_MODEL),
        120,
    );
    let recent_summaries = snapshot
        .get("entries")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .rev()
                .take(5)
                .filter_map(|row| row.pointer("/process_summary/current").and_then(Value::as_str))
                .map(|raw| format!("- {}", clean_text(raw, 280)))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    let user_note = clean_text(
        payload
            .get("note")
            .or_else(|| payload.get("description"))
            .or_else(|| payload.get("body"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        3000,
    );
    let body = format!(
        "source: {source}\nsnapshot_id: {snapshot_id}\neval_report_id: {report_id}\neval_model: {eval_model}\n\nsummary:\n{eval_summary}\n\nrecent_process_summaries:\n{recent_summaries}\n\nuser_note:\n{user_note}"
    );
    let mut request = json!({
        "title": title_hint,
        "body": body,
        "source": source
    });
    if let Some(obj) = request.as_object_mut() {
        if let Some(owner) = payload.get("owner").and_then(Value::as_str) {
            let owner_clean = clean_text(owner, 120);
            if !owner_clean.is_empty() {
                obj.insert("owner".to_string(), json!(owner_clean));
            }
        }
        if let Some(repo) = payload.get("repo").and_then(Value::as_str) {
            let repo_clean = clean_text(repo, 120);
            if !repo_clean.is_empty() {
                obj.insert("repo".to_string(), json!(repo_clean));
            }
        }
        for key in [
            "__github_issue_mock_auth_missing",
            "__github_issue_mock_token",
            "__github_issue_mock_status",
            "__github_issue_mock_body",
        ] {
            if let Some(value) = payload.get(key) {
                obj.insert(key.to_string(), value.clone());
            }
        }
    }
    request
}
fn dashboard_troubleshooting_enqueue_outbox(
    root: &Path,
    issue_request: &Value,
    issue_lane: &LaneResult,
    snapshot: &Value,
    eval_report: &Value,
) -> Value {
    let mut items = dashboard_troubleshooting_read_issue_outbox(root);
    let outbox_row = json!({
        "id": format!(
            "outbox_{}",
            &crate::v8_kernel::sha256_hex_str(&format!(
                "{}:{}",
                now_iso(),
                clean_text(
                    issue_request.get("title").and_then(Value::as_str).unwrap_or("issue"),
                    120
                )
            ))[..12]
        ),
        "created_at": now_iso(),
        "attempts": 0,
        "snapshot_id": clean_text(snapshot.get("snapshot_id").and_then(Value::as_str).unwrap_or(""), 80),
        "eval_report_id": clean_text(eval_report.get("report_id").and_then(Value::as_str).unwrap_or(""), 80),
        "issue_request": issue_request.clone(),
        "last_error": issue_lane.payload.clone().unwrap_or_else(|| json!({})),
        "last_status": issue_lane.status
    });
    items.push(outbox_row.clone());
    if items.len() > DASHBOARD_TROUBLESHOOTING_MAX_OUTBOX {
        let keep_from = items.len() - DASHBOARD_TROUBLESHOOTING_MAX_OUTBOX;
        items = items.split_off(keep_from);
    }
    dashboard_troubleshooting_write_issue_outbox(root, &items);
    outbox_row
}
fn dashboard_payload_usize(value: &Value, key: &str, fallback: usize, min: usize, max: usize) -> usize {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map(|raw| (raw as usize).clamp(min, max))
        .unwrap_or(fallback)
}
