fn maybe_checkpoint(root: &Path, task_id: &str, metrics: &Value, root_dir: Option<&str>) -> Value {
    let loaded = match load_scratchpad(root, task_id, root_dir) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "orchestration_checkpoint_tick",
                "reason_code": err,
                "task_id": task_id
            });
        }
    };

    let should_write = should_checkpoint(&loaded.scratchpad, metrics, &Value::Null);
    if !should_write {
        return json!({
            "ok": true,
            "type": "orchestration_checkpoint_tick",
            "checkpoint_written": false,
            "task_id": task_id,
            "checkpoint_path": loaded.file_path
        });
    }

    let checkpoint = build_checkpoint(task_id, metrics, "interval");
    let appended = append_checkpoint(root, task_id, &checkpoint, root_dir);
    if appended.get("ok").and_then(Value::as_bool) != Some(true) {
        return json!({
            "ok": false,
            "type": "orchestration_checkpoint_tick",
            "reason_code": appended.get("reason_code").cloned().unwrap_or(Value::String("checkpoint_append_failed".to_string())),
            "task_id": task_id
        });
    }

    json!({
        "ok": true,
        "type": "orchestration_checkpoint_tick",
        "checkpoint_written": true,
        "task_id": task_id,
        "checkpoint_path": appended.get("file_path").cloned().unwrap_or(Value::Null),
        "checkpoint": checkpoint
    })
}

fn handle_timeout(root: &Path, task_id: &str, metrics: &Value, root_dir: Option<&str>) -> Value {
    let retry_count = get_i64_any(metrics, &["retry_count"], 0);
    let retry_allowed = retry_count < MAX_AUTO_RETRIES;
    let mut checkpoint = build_checkpoint(task_id, metrics, "timeout");
    if let Value::Object(map) = &mut checkpoint {
        map.insert("retry_allowed".to_string(), Value::Bool(retry_allowed));
    }

    let appended = append_checkpoint(root, task_id, &checkpoint, root_dir);
    if appended.get("ok").and_then(Value::as_bool) != Some(true) {
        return json!({
            "ok": false,
            "type": "orchestration_checkpoint_timeout",
            "reason_code": appended.get("reason_code").cloned().unwrap_or(Value::String("checkpoint_append_failed".to_string())),
            "task_id": task_id
        });
    }

    if let Err(err) = write_scratchpad(
        root,
        task_id,
        &json!({
            "progress": {
                "processed": get_i64_any(metrics, &["processed_count", "processed"], 0),
                "total": get_i64_any(metrics, &["total_count", "total"], 0)
            }
        }),
        root_dir,
    ) {
        return json!({
            "ok": false,
            "type": "orchestration_checkpoint_timeout",
            "reason_code": err,
            "task_id": task_id,
            "checkpoint_path": appended.get("file_path").cloned().unwrap_or(Value::Null)
        });
    }

    json!({
        "ok": true,
        "type": "orchestration_checkpoint_timeout",
        "task_id": task_id,
        "checkpoint_path": appended.get("file_path").cloned().unwrap_or(Value::Null),
        "checkpoint": checkpoint,
        "partial_results": checkpoint.get("partial_results").cloned().unwrap_or(Value::Array(Vec::new())),
        "retry_allowed": retry_allowed
    })
}

fn partial_count_from_group(task_group: &Value) -> i64 {
    let agents = task_group
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut total = 0;
    for agent in agents {
        let details = agent.get("details").cloned().unwrap_or(Value::Null);
        let count = get_i64_any(&details, &["partial_results_count"], {
            details
                .get("partial_results")
                .and_then(Value::as_array)
                .map(|rows| rows.len() as i64)
                .unwrap_or(0)
        });
        if count > 0 {
            total += 1;
        }
    }
    total
}

fn completion_summary(task_group: &Value) -> Value {
    let counts = status_counts(task_group);
    let total = get_i64_any(&counts, &["total"], 0);
    let done = get_i64_any(&counts, &["done"], 0);
    let failed = get_i64_any(&counts, &["failed"], 0);
    let timeout = get_i64_any(&counts, &["timeout"], 0);
    let pending = get_i64_any(&counts, &["pending"], 0);
    let running = get_i64_any(&counts, &["running"], 0);
    let terminal_total = done + failed + timeout;
    let status = {
        let value = to_clean_string(task_group.get("status"));
        if value.is_empty() {
            "pending".to_string()
        } else {
            value
        }
    };

    json!({
        "task_group_id": to_clean_string(task_group.get("task_group_id")).to_ascii_lowercase(),
        "status": status,
        "completed_count": done,
        "failed_count": failed,
        "timeout_count": timeout,
        "pending_count": pending,
        "running_count": running,
        "partial_count": partial_count_from_group(task_group),
        "total_count": total,
        "complete": total > 0 && terminal_total == total,
        "counts": counts
    })
}

fn build_completion_notification(summary: &Value, task_group: &Value) -> Value {
    json!({
        "type": "orchestration_completion_notification",
        "task_group_id": summary.get("task_group_id").cloned().unwrap_or(Value::Null),
        "coordinator_session": task_group.get("coordinator_session").cloned().unwrap_or(Value::Null),
        "status": summary.get("status").cloned().unwrap_or(Value::Null),
        "completed_count": summary.get("completed_count").cloned().unwrap_or(Value::Null),
        "failed_count": summary.get("failed_count").cloned().unwrap_or(Value::Null),
        "timeout_count": summary.get("timeout_count").cloned().unwrap_or(Value::Null),
        "partial_count": summary.get("partial_count").cloned().unwrap_or(Value::Null),
        "total_count": summary.get("total_count").cloned().unwrap_or(Value::Null),
        "generated_at": now_iso()
    })
}

fn ensure_and_summarize(root: &Path, task_group_id: &str, root_dir: Option<&str>) -> Value {
    let ensured = ensure_task_group(root, &json!({ "task_group_id": task_group_id }), root_dir);
    if ensured.get("ok").and_then(Value::as_bool) != Some(true) {
        return ensured;
    }
    let task_group = ensured
        .get("task_group")
        .cloned()
        .unwrap_or(Value::Object(Map::new()));
    let summary = completion_summary(&task_group);
    json!({
        "ok": true,
        "type": "orchestration_completion_summary",
        "task_group": task_group,
        "summary": summary,
        "notification": if summary.get("complete").and_then(Value::as_bool) == Some(true) {
            build_completion_notification(&summary, &task_group)
        } else {
            Value::Null
        }
    })
}

fn track_agent_completion(
    root: &Path,
    task_group_id: &str,
    update: &Value,
    root_dir: Option<&str>,
) -> Value {
    let agent_id = get_string_any(update, &["agent_id", "agentId"]);
    let status = get_string_any(update, &["status"]).to_ascii_lowercase();
    if agent_id.is_empty() {
        return json!({
            "ok": false,
            "type": "orchestration_completion_track",
            "reason_code": "missing_agent_id"
        });
    }
    if !allowed_agent_status(&status) {
        return json!({
            "ok": false,
            "type": "orchestration_completion_track",
            "reason_code": format!("invalid_agent_status:{}", if status.is_empty() { "<empty>" } else { &status })
        });
    }

    let details = update
        .get("details")
        .cloned()
        .unwrap_or(Value::Object(Map::new()));
    let updated = update_agent_status(root, task_group_id, &agent_id, &status, &details, root_dir);
    if updated.get("ok").and_then(Value::as_bool) != Some(true) {
        return updated;
    }

    let task_group = updated
        .get("task_group")
        .cloned()
        .unwrap_or(Value::Object(Map::new()));
    let summary = completion_summary(&task_group);
    json!({
        "ok": true,
        "type": "orchestration_completion_track",
        "task_group": task_group,
        "summary": summary,
        "notification": if summary.get("complete").and_then(Value::as_bool) == Some(true) {
            build_completion_notification(&summary, &task_group)
        } else {
            Value::Null
        }
    })
}

fn track_batch_completion(
    root: &Path,
    task_group_id: &str,
    updates: &[Value],
    root_dir: Option<&str>,
) -> Value {
    let mut results = Vec::new();
    for update in updates {
        let tracked = track_agent_completion(root, task_group_id, update, root_dir);
        if tracked.get("ok").and_then(Value::as_bool) != Some(true) {
            return json!({
                "ok": false,
                "type": "orchestration_completion_track_batch",
                "reason_code": tracked.get("reason_code").cloned().unwrap_or(Value::String("batch_update_failed".to_string())),
                "failed_update": update
            });
        }
        results.push(json!({
            "agent_id": get_string_any(update, &["agent_id", "agentId"]),
            "status": get_string_any(update, &["status"]).to_ascii_lowercase(),
            "summary": tracked.get("summary").cloned().unwrap_or(Value::Null)
        }));
    }

    let query = query_task_group(root, task_group_id, root_dir);
    if query.get("ok").and_then(Value::as_bool) != Some(true) {
        return query;
    }

    let task_group = query
        .get("task_group")
        .cloned()
        .unwrap_or(Value::Object(Map::new()));
    let summary = completion_summary(&task_group);

    json!({
        "ok": true,
        "type": "orchestration_completion_track_batch",
        "task_group": task_group,
        "summary": summary,
        "updates_applied": results.len(),
        "updates": results,
        "notification": if summary.get("complete").and_then(Value::as_bool) == Some(true) {
            build_completion_notification(&summary, &task_group)
        } else {
            Value::Null
        }
    })
}

fn normalize_decision(raw: &str, has_partial_results: bool) -> String {
    let normalized = raw.trim().to_ascii_lowercase();
    if matches!(normalized.as_str(), "retry" | "continue" | "abort") {
        normalized
    } else if has_partial_results {
        "continue".to_string()
    } else {
        "retry".to_string()
    }
}

fn extract_partial_from_session_entry(entry: &Value) -> Option<Value> {
    if !entry.is_object() {
        return None;
    }

    let candidates = [
        entry.get("partial_results"),
        entry.get("partialResults"),
        entry.get("partial"),
        entry.get("findings"),
        entry.get("result").and_then(|v| v.get("partial_results")),
        entry.get("result").and_then(|v| v.get("findings")),
        entry.get("output").and_then(|v| v.get("partial_results")),
        entry.get("output").and_then(|v| v.get("findings")),
        entry.get("payload").and_then(|v| v.get("partial_results")),
        entry.get("payload").and_then(|v| v.get("findings")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if let Some(rows) = candidate.as_array() {
            if rows.is_empty() {
                continue;
            }
            let items_completed = get_i64_any(
                entry,
                &["items_completed", "processed_count"],
                rows.len() as i64,
            );
            return Some(json!({
                "partial_results": rows,
                "items_completed": items_completed,
                "checkpoint_path": entry.get("checkpoint_path").or_else(|| entry.get("checkpointPath")).cloned().unwrap_or(Value::Null),
                "source_session_id": entry.get("session_id").or_else(|| entry.get("sessionId")).cloned().unwrap_or(Value::Null)
            }));
        }
    }

    None
}

fn from_session_history(history: &[Value]) -> Value {
    for entry in history.iter().rev() {
        if let Some(extracted) = extract_partial_from_session_entry(entry) {
            return json!({
                "ok": true,
                "type": "orchestration_partial_from_session_history",
                "source": "session_history",
                "items_completed": extracted.get("items_completed").cloned().unwrap_or(Value::Null),
                "findings_sofar": extracted.get("partial_results").cloned().unwrap_or(Value::Array(Vec::new())),
                "checkpoint_path": extracted.get("checkpoint_path").cloned().unwrap_or(Value::Null),
                "source_session_id": extracted.get("source_session_id").cloned().unwrap_or(Value::Null)
            });
        }
    }
    json!({
        "ok": false,
        "type": "orchestration_partial_from_session_history",
        "reason_code": "session_history_no_partial_results"
    })
}

fn latest_checkpoint_from_scratchpad(root: &Path, task_id: &str, root_dir: Option<&str>) -> Value {
    let loaded = match load_scratchpad(root, task_id, root_dir) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "orchestration_partial_checkpoint_fallback",
                "reason_code": err,
                "task_id": task_id,
                "checkpoint_path": Value::Null
            });
        }
    };

    let checkpoints = loaded
        .scratchpad
        .get("checkpoints")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let latest = checkpoints
        .iter()
        .rev()
        .find(|checkpoint| {
            checkpoint
                .get("partial_results")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
        })
        .cloned()
        .unwrap_or(Value::Null);
    let partial_results = latest
        .get("partial_results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if partial_results.is_empty() {
        return json!({
            "ok": false,
            "type": "orchestration_partial_checkpoint_fallback",
            "reason_code": "checkpoint_no_partial_results",
            "task_id": task_id,
            "checkpoint_path": loaded.file_path
        });
    }

    json!({
        "ok": true,
        "type": "orchestration_partial_checkpoint_fallback",
        "source": "checkpoint",
        "task_id": task_id,
        "checkpoint_path": loaded.file_path,
        "items_completed": get_i64_any(&latest, &["processed_count"], partial_results.len() as i64),
        "findings_sofar": partial_results,
        "retry_allowed": latest.get("retry_allowed").and_then(Value::as_bool).unwrap_or(false)
    })
}
