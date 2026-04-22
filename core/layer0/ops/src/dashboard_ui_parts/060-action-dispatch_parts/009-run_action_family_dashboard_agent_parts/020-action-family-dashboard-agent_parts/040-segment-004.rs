            let favorites_only = payload
                .get("favorites_only")
                .and_then(Value::as_bool)
                .or_else(|| payload.get("favoritesOnly").and_then(Value::as_bool))
                .unwrap_or(false);
            let current_workspace_only = payload
                .get("current_workspace_only")
                .and_then(Value::as_bool)
                .or_else(|| payload.get("currentWorkspaceOnly").and_then(Value::as_bool))
                .unwrap_or(false);
            let workspace_path = payload
                .get("workspace_path")
                .and_then(Value::as_str)
                .or_else(|| payload.get("workspacePath").and_then(Value::as_str))
                .map(|v| clean_text(v, 400))
                .unwrap_or_default();
            let search_query = payload
                .get("search_query")
                .and_then(Value::as_str)
                .or_else(|| payload.get("searchQuery").and_then(Value::as_str))
                .map(|v| clean_text(v, 240).to_ascii_lowercase())
                .unwrap_or_default();
            let sort_by = payload
                .get("sort_by")
                .and_then(Value::as_str)
                .or_else(|| payload.get("sortBy").and_then(Value::as_str))
                .map(|v| clean_text(v, 40).to_ascii_lowercase())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "newest".to_string());

            let mut tasks = crate::dashboard_compat_api_comms_store::read_tasks(root);
            if crate::dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                crate::dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            }
            let mut filtered = tasks
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
                    if favorites_only
                        && !row
                            .get("is_favorited")
                            .and_then(Value::as_bool)
                            .or_else(|| row.get("isFavorited").and_then(Value::as_bool))
                            .unwrap_or(false)
                    {
                        return false;
                    }
                    if assigned_to.is_empty() {
                        if current_workspace_only
                            && !dashboard_agent_task_workspace_match(row, &workspace_path)
                        {
                            return false;
                        }
                    } else if clean_agent_id(
                        row.get("assigned_to")
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                    ) != assigned_to
                    {
                        return false;
                    }
                    if current_workspace_only
                        && !dashboard_agent_task_workspace_match(row, &workspace_path)
                    {
                        return false;
                    }
                    if !search_query.is_empty()
                        && !dashboard_agent_task_search_blob(row).contains(&search_query)
                    {
                        return false;
                    }
                    true
                })
                .collect::<Vec<_>>();
            filtered.sort_by(|a, b| match sort_by.as_str() {
                "oldest" => dashboard_agent_task_timestamp_seconds(a)
                    .cmp(&dashboard_agent_task_timestamp_seconds(b)),
                "mostexpensive" | "most_expensive" => dashboard_agent_task_cost_total(b)
                    .partial_cmp(&dashboard_agent_task_cost_total(a))
                    .unwrap_or(std::cmp::Ordering::Equal),
                "mosttokens" | "most_tokens" => {
                    dashboard_agent_task_token_total(b).cmp(&dashboard_agent_task_token_total(a))
                }
                _ => dashboard_agent_task_timestamp_seconds(b)
                    .cmp(&dashboard_agent_task_timestamp_seconds(a)),
            });
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
                    "total_size_bytes": total_size_bytes,
                    "filters": {
                        "favorites_only": favorites_only,
                        "current_workspace_only": current_workspace_only,
                        "workspace_path": workspace_path,
                        "search_query": search_query,
                        "sort_by": sort_by
                    }
                })),
            }
        }
        "dashboard.agent.task.totalSize" | "dashboard.agent.task.getTotalSize" => {
            let mut tasks = crate::dashboard_compat_api_comms_store::read_tasks(root);
            if crate::dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                crate::dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            }
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(json!({
                    "ok": true,
                    "type": "dashboard_agent_task_total_size",
                    "task_count": tasks.len() as i64,
                    "total_size_bytes": dashboard_agent_task_total_size(&tasks),
                    "status_counts": dashboard_agent_task_status_counts(&tasks)
                })),
            }
        }
        "dashboard.agent.task.export"
        | "dashboard.agent.task.exportWithId"
        | "dashboard.agent.task.get"
        | "dashboard.agent.task.showTaskWithId" => {
            let task_id = payload
                .get("task_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("id").and_then(Value::as_str))
                .or_else(|| payload.get("value").and_then(Value::as_str))
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
                argv: vec![normalized.to_string()],
                payload: Some(json!({
                    "ok": true,
                    "type": if normalized == "dashboard.agent.task.showTaskWithId" {
                        "dashboard_agent_task_show_with_id"
                    } else {
                        "dashboard_agent_task_export"
                    },
                    "task_id": task_id,
                    "format": format,
                    "source_action": normalized,
                    "task": task,
                    "export": export
                })),
            }
        }
        "dashboard.agent.task.explainChangesShared"
        | "dashboard.agent.task.taskCompletionViewChanges" => {
            let mut tasks = crate::dashboard_compat_api_comms_store::read_tasks(root);
            if crate::dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                crate::dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            }
            let before = payload
                .get("before")
                .or_else(|| payload.get("before_task"))
                .or_else(|| payload.get("beforeTask"))
                .or_else(|| payload.get("old_task"))
                .cloned()
                .or_else(|| {
                    let id = clean_text(
                        payload
                            .get("before_task_id")
                            .and_then(Value::as_str)
                            .or_else(|| payload.get("beforeTaskId").and_then(Value::as_str))
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
                .or_else(|| {
                    let id = clean_text(
                        payload
                            .get("task_id")
                            .and_then(Value::as_str)
                            .or_else(|| payload.get("taskId").and_then(Value::as_str))
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
                .or_else(|| payload.get("after_task"))
                .or_else(|| payload.get("afterTask"))
                .or_else(|| payload.get("new_task"))
                .cloned()
                .or_else(|| {
                    let id = clean_text(
                        payload
                            .get("after_task_id")
                            .and_then(Value::as_str)
                            .or_else(|| payload.get("afterTaskId").and_then(Value::as_str))
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
