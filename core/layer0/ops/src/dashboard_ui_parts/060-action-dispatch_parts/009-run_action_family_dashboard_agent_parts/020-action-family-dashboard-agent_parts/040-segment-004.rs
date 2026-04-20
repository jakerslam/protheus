            let mut tasks = crate::dashboard_compat_api_comms_store::read_tasks(root);
            if crate::dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                crate::dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            }
            let filtered = tasks
                .into_iter()
                .filter(|row| {
                    let status = clean_text(
                        row.get("status").and_then(Value::as_str).unwrap_or("queued"),
                        40,
                    )
                    .to_ascii_lowercase();
                    let is_done = matches!(
                        status.as_str(),
                        "completed" | "failed" | "timed_out" | "paused" | "cancelled" | "canceled" | "aborted"
                    );
                    if !include_completed && is_done {
                        return false;
                    }
                    if assigned_to.is_empty() {
                        return true;
                    }
                    clean_agent_id(
                        row.get("assigned_to")
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                    ) == assigned_to
                })
                .collect::<Vec<_>>();
            let total_count = filtered.len() as i64;
            let rows = filtered.into_iter().take(limit).collect::<Vec<_>>();
            let total_size_bytes = dashboard_agent_task_total_size(&rows);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.agent.task.history".to_string()],
                payload: Some(json!({
                    "ok": true,
                    "type": "dashboard_agent_task_history",
                    "rows": rows,
                    "total_count": total_count,
                    "total_size_bytes": total_size_bytes
                })),
            }
        }
        "dashboard.agent.task.totalSize" => {
            let mut tasks = crate::dashboard_compat_api_comms_store::read_tasks(root);
            if crate::dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                crate::dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            }
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.agent.task.totalSize".to_string()],
                payload: Some(json!({
                    "ok": true,
                    "type": "dashboard_agent_task_total_size",
                    "task_count": tasks.len() as i64,
                    "total_size_bytes": dashboard_agent_task_total_size(&tasks),
                    "status_counts": dashboard_agent_task_status_counts(&tasks)
                })),
            }
        }
        "dashboard.agent.task.export" => {
            let task_id = payload
                .get("task_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("id").and_then(Value::as_str))
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            if task_id.is_empty() {
                return LaneResult {
                    ok: false,
                    status: 2,
                    argv: vec!["dashboard.agent.task.export".to_string()],
                    payload: Some(json!({
                        "ok": false,
                        "error": "task_id_required",
                        "type": "dashboard_agent_task_error"
                    })),
                };
            }
            let format = payload
                .get("format")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 20).to_ascii_lowercase())
                .unwrap_or_else(|| "json".to_string());
            let mut tasks = crate::dashboard_compat_api_comms_store::read_tasks(root);
            if crate::dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                crate::dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            }
            let task = tasks.into_iter().find(|row| {
                clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80) == task_id
            });
            let Some(task) = task else {
                return LaneResult {
                    ok: false,
                    status: 2,
                    argv: vec!["dashboard.agent.task.export".to_string()],
                    payload: Some(json!({
                        "ok": false,
                        "error": "task_not_found",
                        "task_id": task_id,
                        "type": "dashboard_agent_task_error"
                    })),
                };
            };
            let export = if format == "markdown" || format == "md" {
                format!(
                    "# Task {}\n\n- id: `{}`\n- status: `{}`\n- assigned_to: `{}`\n- completion_percent: `{}`\n- updated_at: `{}`\n\n## Description\n{}\n",
                    clean_text(task.get("title").and_then(Value::as_str).unwrap_or("Task"), 200),
                    clean_text(task.get("id").and_then(Value::as_str).unwrap_or(""), 80),
                    clean_text(task.get("status").and_then(Value::as_str).unwrap_or("queued"), 40),
                    clean_text(task.get("assigned_to").and_then(Value::as_str).unwrap_or(""), 140),
                    task.get("completion_percent").and_then(Value::as_i64).unwrap_or(0),
                    clean_text(task.get("updated_at").and_then(Value::as_str).unwrap_or(""), 80),
                    clean_text(task.get("description").and_then(Value::as_str).unwrap_or(""), 4000)
                )
            } else {
                serde_json::to_string_pretty(&task).unwrap_or_else(|_| "{}".to_string())
            };
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.agent.task.export".to_string()],
                payload: Some(json!({
                    "ok": true,
                    "type": "dashboard_agent_task_export",
                    "task_id": task_id,
                    "format": format,
                    "task": task,
                    "export": export
                })),
            }
        }
        "dashboard.agent.task.explainChangesShared" => {
            let mut tasks = crate::dashboard_compat_api_comms_store::read_tasks(root);
            if crate::dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                crate::dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            }
            let before = payload
                .get("before")
                .or_else(|| payload.get("old_task"))
                .cloned()
                .or_else(|| {
                    let id = clean_text(
                        payload
                            .get("before_task_id")
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                        80,
                    );
                    if id.is_empty() {
                        return None;
                    }
                    tasks.iter().find(|row| {
                        clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80) == id
                    }).cloned()
                })
                .unwrap_or_else(|| json!({}));
            let after = payload
                .get("after")
                .or_else(|| payload.get("new_task"))
                .cloned()
                .or_else(|| {
                    let id = clean_text(
                        payload
                            .get("after_task_id")
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                        80,
                    );
                    if id.is_empty() {
                        return None;
                    }
                    tasks.iter().find(|row| {
                        clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80) == id
                    }).cloned()
                })
                .unwrap_or_else(|| json!({}));
