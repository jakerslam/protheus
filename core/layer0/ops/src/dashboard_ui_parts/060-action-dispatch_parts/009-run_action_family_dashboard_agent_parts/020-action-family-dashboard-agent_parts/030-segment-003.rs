                "queued"
            } else {
                "running"
            };
            let mut task = json!({
                "id": crate::dashboard_compat_api_comms_store::make_task_id(&seed),
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
            let _ = crate::dashboard_compat_api_comms_store::sync_swarm_progress(&mut task);
            let mut tasks = crate::dashboard_compat_api_comms_store::read_tasks(root);
            tasks.insert(0, task.clone());
            if crate::dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
                crate::dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            } else {
                crate::dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            }
            let task_id = clean_text(task.get("id").and_then(Value::as_str).unwrap_or(""), 80);
            crate::dashboard_compat_api_comms_store::append_event(
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

