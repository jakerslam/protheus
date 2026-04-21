
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
        "dashboard.agent.task.new" => {
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

            let mut tasks = dashboard_compat_api_comms_store::read_tasks(root);
            if dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                dashboard_compat_api_comms_store::write_tasks(root, &tasks);
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
            let mut tasks = dashboard_compat_api_comms_store::read_tasks(root);
            if dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                dashboard_compat_api_comms_store::write_tasks(root, &tasks);
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
            let mut tasks = dashboard_compat_api_comms_store::read_tasks(root);
            if dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                dashboard_compat_api_comms_store::write_tasks(root, &tasks);
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
            let (shared_fields, changed_fields) = dashboard_agent_task_shared_and_changed(&before, &after);
            let changed_count = changed_fields
                .as_array()
                .map(|rows| rows.len() as i64)
                .unwrap_or(0);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.agent.task.explainChangesShared".to_string()],
                payload: Some(json!({
                    "ok": true,
                    "type": "dashboard_agent_task_explain_changes_shared",
                    "shared_fields": shared_fields,
                    "changed_fields": changed_fields,
                    "changed_count": changed_count
                })),
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
