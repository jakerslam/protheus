
fn run_action_family_dashboard_agent(root: &Path, normalized: &str, payload: &Value) -> LaneResult {
    match normalized {
        "dashboard.agent.upsertProfile" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::upsert_profile(root, &agent_id, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.upsertProfile".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.archive" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let reason = payload
                .get("reason")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::archive_agent(root, &agent_id, &reason);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.archive".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.unarchive" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::unarchive_agent(root, &agent_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.unarchive".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.upsertContract" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::upsert_contract(root, &agent_id, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.upsertContract".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.enforceContracts" => {
            let result = dashboard_agent_state::enforce_expired_contracts(root);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: 0,
                argv: vec!["dashboard.agent.enforceContracts".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.get" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let result = dashboard_agent_state::load_session(root, &agent_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.get".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.create" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let label = payload
                .get("label")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            let result = dashboard_agent_state::create_session(root, &agent_id, &label);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.create".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.switch" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let session_id = payload
                .get("session_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("sessionId").and_then(Value::as_str))
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::switch_session(root, &agent_id, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.switch".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.delete" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let session_id = payload
                .get("session_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("sessionId").and_then(Value::as_str))
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::delete_session(root, &agent_id, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.delete".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.session.appendTurn" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let user_text = payload
                .get("user")
                .and_then(Value::as_str)
                .or_else(|| payload.get("input").and_then(Value::as_str))
                .map(|v| clean_chat_text_preserve_layout(v, 2000))
                .unwrap_or_default();
            let assistant_text = payload
                .get("assistant")
                .and_then(Value::as_str)
                .or_else(|| payload.get("response").and_then(Value::as_str))
                .map(|v| clean_chat_text_preserve_layout(v, 4000))
                .unwrap_or_default();
            let result =
                dashboard_agent_state::append_turn(root, &agent_id, &user_text, &assistant_text);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.session.appendTurn".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.set" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let value = payload.get("value").cloned().unwrap_or(Value::Null);
            let result = dashboard_agent_state::memory_kv_set(root, &agent_id, &key, &value);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.set".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.get" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::memory_kv_get(root, &agent_id, &key);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.get".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.memoryKv.delete" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let key = payload
                .get("key")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_agent_state::memory_kv_delete(root, &agent_id, &key);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.memoryKv.delete".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.suggestions" => {
            let agent_id = payload
                .get("agent_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agentId").and_then(Value::as_str))
                .map(|v| clean_text(v, 140))
                .unwrap_or_default();
            let user_hint = payload
                .get("user_hint")
                .and_then(Value::as_str)
                .or_else(|| payload.get("hint").and_then(Value::as_str))
                .map(|v| clean_text(v, 220))
                .unwrap_or_default();
            let result = dashboard_agent_state::suggestions(root, &agent_id, &user_hint);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.agent.suggestions".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.task.new" | "dashboard.agent.task.newTask" => {
            let title = payload
                .get("title")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 200))
                .unwrap_or_default();
            if title.is_empty() {
                return LaneResult {
                    ok: false,
                    status: 2,
                    argv: vec!["dashboard.agent.task.new".to_string()],
                    payload: Some(json!({
                        "ok": false,
                        "error": "title_required",
                        "type": "dashboard_agent_task_error"
                    })),
                };
            }
            let description = payload
                .get("description")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 4000))
                .unwrap_or_default();
            let assigned_to = payload
                .get("assigned_to")
                .and_then(Value::as_str)
                .or_else(|| payload.get("agent_id").and_then(Value::as_str))
                .map(clean_agent_id)
                .unwrap_or_default();
            let timeout_secs = payload
                .get("timeout_secs")
                .and_then(Value::as_i64)
                .unwrap_or(300)
                .clamp(15, 86_400);
            let now = chrono::Utc::now();
            let now_iso = now.to_rfc3339();
            let deadline = (now + chrono::Duration::seconds(timeout_secs)).to_rfc3339();
            let seed = json!({
                "kind": "task",
                "title": title,
                "assigned_to": assigned_to,
                "ts": now_iso
            });
            let status = if assigned_to.is_empty() {
                "queued"
            } else {
                "running"
            };
            let mut task = json!({
                "id": dashboard_compat_api_comms_store::make_task_id(&seed),
                "title": title,
                "description": description,
                "assigned_to": assigned_to,
                "status": status,
                "completion_percent": 0,
                "created_at": now_iso,
                "updated_at": now_iso,
                "started_at": now_iso,
                "deadline_at": deadline,
                "timeout_secs": timeout_secs,
                "retry_count": 0,
                "max_retries": payload.get("max_retries").and_then(Value::as_i64).unwrap_or(1).clamp(0, 20),
                "auto_retry_on_timeout": payload.get("auto_retry_on_timeout").and_then(Value::as_bool).unwrap_or(true),
                "swarm_agent_ids": [],
                "completed_agent_ids": [],
                "pending_agent_ids": [],
                "partial_results": {}
            });
            let _ = dashboard_compat_api_comms_store::sync_swarm_progress(&mut task);
            let mut tasks = dashboard_compat_api_comms_store::read_tasks(root);
            tasks.insert(0, task.clone());
            if dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            } else {
                dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            }
            let task_id = clean_text(task.get("id").and_then(Value::as_str).unwrap_or(""), 80);
            dashboard_compat_api_comms_store::append_event(
                root,
                "task_posted",
                "Swarm",
                "",
                &format!("{} (timeout {}s)", title, timeout_secs),
                Some(&task_id),
            );
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.agent.task.new".to_string()],
                payload: Some(json!({
                    "ok": true,
                    "type": "dashboard_agent_task_created",
                    "task": task,
                    "task_count": tasks.len() as i64
                })),
            }
        }
        "dashboard.agent.task.history" => {
            let limit = payload
                .get("limit")
                .and_then(Value::as_i64)
                .unwrap_or(50)
                .clamp(1, 500) as usize;
            let include_completed = payload
                .get("include_completed")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let assigned_to = payload
                .get("assigned_to")
                .and_then(Value::as_str)
                .map(clean_agent_id)
                .unwrap_or_default();

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

            let mut tasks = dashboard_compat_api_comms_store::read_tasks(root);
            if dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                dashboard_compat_api_comms_store::write_tasks(root, &tasks);
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
            let mut tasks = dashboard_compat_api_comms_store::read_tasks(root);
            if dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                dashboard_compat_api_comms_store::write_tasks(root, &tasks);
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
            let mut tasks = dashboard_compat_api_comms_store::read_tasks(root);
            if dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                dashboard_compat_api_comms_store::write_tasks(root, &tasks);
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
            let mut tasks = dashboard_compat_api_comms_store::read_tasks(root);
            if dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                dashboard_compat_api_comms_store::write_tasks(root, &tasks);
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
            let (shared_fields, changed_fields) = dashboard_agent_task_shared_and_changed(&before, &after);
            let changed_count = changed_fields
                .as_array()
                .map(|rows| rows.len() as i64)
                .unwrap_or(0);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(json!({
                    "ok": true,
                    "type": if normalized == "dashboard.agent.task.taskCompletionViewChanges" {
                        "dashboard_agent_task_completion_view_changes"
                    } else {
                        "dashboard_agent_task_explain_changes_shared"
                    },
                    "shared_fields": shared_fields,
                    "changed_fields": changed_fields,
                    "changed_count": changed_count,
                    "view_changes": changed_fields
                })),
            }
        }
        "dashboard.agent.task.favorite" | "dashboard.agent.task.toggleFavorite" => {
            let task_id = payload
                .get("task_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("taskId").and_then(Value::as_str))
                .or_else(|| payload.get("id").and_then(Value::as_str))
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            let is_favorited = payload
                .get("is_favorited")
                .and_then(Value::as_bool)
                .or_else(|| payload.get("isFavorited").and_then(Value::as_bool))
                .or_else(|| payload.get("favorite").and_then(Value::as_bool))
                .unwrap_or(true);
            let result = dashboard_agent_task_apply_favorite(root, &task_id, is_favorited);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.task.feedback" => {
            let task_id = payload
                .get("task_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("taskId").and_then(Value::as_str))
                .or_else(|| payload.get("id").and_then(Value::as_str))
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            let feedback = payload
                .get("feedback")
                .and_then(Value::as_str)
                .or_else(|| payload.get("value").and_then(Value::as_str))
                .or_else(|| payload.get("type").and_then(Value::as_str))
                .map(|v| clean_text(v, 64))
                .unwrap_or_default();
            let result = dashboard_agent_task_apply_feedback(root, &task_id, &feedback);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.initializeWebview" => {
            let result = dashboard_ui_controller_initialize(root, payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.initializeWebview".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.setTerminalExecutionMode" => {
            let result = dashboard_ui_controller_set_terminal_execution_mode(root, payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.setTerminalExecutionMode".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToAddToInput" | "dashboard.ui.event.addToInput" => {
            let result = dashboard_ui_controller_record_subscription(root, "add_to_input", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToChatButtonClicked" | "dashboard.ui.event.chatButtonClicked" => {
            let result =
                dashboard_ui_controller_record_subscription(root, "chat_button_clicked", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToHistoryButtonClicked"
        | "dashboard.ui.event.historyButtonClicked" => {
            let result = dashboard_ui_controller_record_subscription(
                root,
                "history_button_clicked",
                payload,
            );
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.getWebviewHtml" => {
            let result = dashboard_ui_controller_get_webview_html(root);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.getWebviewHtml".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.onDidShowAnnouncement" => {
            let result = dashboard_ui_controller_on_did_show_announcement(root, payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.onDidShowAnnouncement".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.openUrl" => {
            let result = dashboard_ui_controller_open_url(root, payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.openUrl".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.openWalkthrough" => {
            let result = dashboard_ui_controller_open_walkthrough(root);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.openWalkthrough".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.scrollToSettings" => {
            let result = dashboard_ui_controller_scroll_to_settings(root, payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.scrollToSettings".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToAccountButtonClicked"
        | "dashboard.ui.event.accountButtonClicked" => {
            let result =
                dashboard_ui_controller_record_subscription(root, "account_button_clicked", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToMcpButtonClicked" | "dashboard.ui.event.mcpButtonClicked" => {
            let result =
                dashboard_ui_controller_record_subscription(root, "mcp_button_clicked", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToPartialMessage" | "dashboard.ui.event.partialMessage" => {
            let result =
                dashboard_ui_controller_record_subscription(root, "partial_message", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToRelinquishControl"
        | "dashboard.ui.event.relinquishControl" => {
            let result =
                dashboard_ui_controller_record_subscription(root, "relinquish_control", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToSettingsButtonClicked"
        | "dashboard.ui.event.settingsButtonClicked" => {
            let result = dashboard_ui_controller_record_subscription(
                root,
                "settings_button_clicked",
                payload,
            );
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToShowWebview" | "dashboard.ui.event.showWebview" => {
            let result = dashboard_ui_controller_record_subscription(root, "show_webview", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToWorktreesButtonClicked"
        | "dashboard.ui.event.worktreesButtonClicked" => {
            let result = dashboard_ui_controller_record_subscription(
                root,
                "worktrees_button_clicked",
                payload,
            );
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.web.checkIsImageUrl" => {
            let result = dashboard_web_check_is_image_url(payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.web.checkIsImageUrl".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.web.fetchOpenGraphData" => {
            let result = dashboard_web_fetch_open_graph_data(payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.web.fetchOpenGraphData".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.web.openInBrowser" => {
            let result = dashboard_web_open_in_browser(root, payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.web.openInBrowser".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.listWorktrees" => {
            let result = dashboard_worktree_list(root);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.worktree.listWorktrees".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.getAvailableBranches" => {
            let result = dashboard_worktree_get_available_branches(root);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.worktree.getAvailableBranches".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.createWorktree" => {
            let result = dashboard_worktree_create(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.createWorktree".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.deleteWorktree" => {
            let result = dashboard_worktree_delete(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.deleteWorktree".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.switchWorktree" => {
            let result = dashboard_worktree_switch(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.switchWorktree".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.checkoutBranch" => {
            let result = dashboard_worktree_checkout_branch(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.checkoutBranch".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.createWorktreeInclude" => {
            let result = dashboard_worktree_create_include(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.createWorktreeInclude".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.getWorktreeDefaults" => {
            let result = dashboard_worktree_get_defaults(root);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.worktree.getWorktreeDefaults".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.getWorktreeIncludeStatus" => {
            let result = dashboard_worktree_get_include_status(root);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.worktree.getWorktreeIncludeStatus".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.mergeWorktree" => {
            let result = dashboard_worktree_merge(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.mergeWorktree".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.trackWorktreeViewOpened" => {
            let result = dashboard_worktree_track_view_opened(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.trackWorktreeViewOpened".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.hooks.registry.register"
        | "dashboard.hooks.registry.list"
        | "dashboard.hooks.discoveryCache.get"
        | "dashboard.hooks.discoveryCache.refresh"
        | "dashboard.hooks.process.start"
        | "dashboard.hooks.process.complete"
        | "dashboard.hooks.process.registry" => {
            let result = dashboard_hook_route(root, normalized, payload);
            let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
            LaneResult {
                ok,
                status: if ok { 0 } else { 2 },
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.hooks.test.setupFixture"
        | "dashboard.hooks.test.factory.validate"
        | "dashboard.hooks.test.modelContext.build"
        | "dashboard.hooks.test.process.simulate"
        | "dashboard.hooks.test.utils.normalize"
        | "dashboard.hooks.test.notification.emit"
        | "dashboard.hooks.test.shellEscape.inspect"
        | "dashboard.hooks.test.taskCancel.simulate"
        | "dashboard.hooks.test.taskComplete.simulate"
        | "dashboard.hooks.test.taskResume.simulate"
        | "dashboard.hooks.test.taskStart.simulate"
        | "dashboard.hooks.test.userPromptSubmit.simulate"
        | "dashboard.hooks.test.precompact.evaluate"
        | "dashboard.hooks.test.templates.render"
        | "dashboard.hooks.test.templates.placeholders"
        | "dashboard.hooks.test.utils.digest"
        | "dashboard.hooks.test.ignore.evaluate" => {
            let result = dashboard_hook_test_route(root, normalized, payload);
            let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
            LaneResult {
                ok,
                status: if ok { 0 } else { 2 },
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        _ if dashboard_prompt_route_supported(normalized) => {
            let result = dashboard_lock_permission_prompt_route(root, normalized, payload);
            let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
            LaneResult {
                ok,
                status: if ok { 0 } else { 2 },
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        _ => LaneResult {
            ok: false,
            status: 2,
            argv: Vec::new(),
            payload: Some(json!({
                "ok": false,
                "type": "infring_dashboard_action_error",
                "error": format!("unsupported_action:{normalized}")
            })),
        },
    }
}
