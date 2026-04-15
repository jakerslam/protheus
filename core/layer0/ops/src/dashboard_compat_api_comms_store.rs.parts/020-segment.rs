pub fn build_retry_task(parent: &Value, now: DateTime<Utc>, retry_count: i64, auto: bool) -> Value {
    let timeout_secs = parse_task_timeout_secs(parent);
    let progress = parse_task_progress(parent);
    let title = super::super::clean_text(
        parent
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Task"),
        200,
    );
    let description = super::super::clean_text(
        parent
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        4_000,
    );
    let assigned_to = super::super::clean_agent_id(
        parent
            .get("assigned_to")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let status = if assigned_to.is_empty() {
        "queued"
    } else {
        "running"
    };
    let swarm_agent_ids = parse_swarm_agents(parent);
    let completed_agent_ids = parse_completed_agents(parent);
    let pending_agent_ids = if parse_pending_agents(parent).is_empty() {
        swarm_agent_ids
            .iter()
            .filter(|id| !completed_agent_ids.iter().any(|done| done == *id))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        parse_pending_agents(parent)
    };
    let now_iso = now.to_rfc3339();
    let deadline = (now + Duration::seconds(timeout_secs)).to_rfc3339();
    let seed = json!({
        "kind": "task_retry",
        "parent_id": super::super::clean_text(parent.get("id").and_then(Value::as_str).unwrap_or(""), 80),
        "retry_count": retry_count,
        "ts": now_iso
    });
    let mut retry_task = json!({
        "id": make_task_id(&seed),
        "title": title,
        "description": description,
        "assigned_to": assigned_to,
        "status": status,
        "completion_percent": progress,
        "created_at": now_iso,
        "updated_at": now_iso,
        "started_at": now_iso,
        "deadline_at": deadline,
        "timeout_secs": timeout_secs,
        "retry_count": retry_count,
        "max_retries": parse_task_max_retries(parent),
        "auto_retry_on_timeout": parent.get("auto_retry_on_timeout").and_then(Value::as_bool).unwrap_or(true),
        "carryover_from": super::super::clean_text(parent.get("id").and_then(Value::as_str).unwrap_or(""), 80),
        "carryover_detail": if auto { "auto_timeout_retry" } else { "manual_rerun" },
        "result_summary": parent.get("result_summary").cloned().unwrap_or_else(|| json!("")),
        "partial_results": parent.get("partial_results").cloned().unwrap_or_else(|| json!({})),
        "swarm_agent_ids": swarm_agent_ids,
        "completed_agent_ids": completed_agent_ids,
        "pending_agent_ids": pending_agent_ids
    });
    let _ = sync_swarm_progress(&mut retry_task);
    retry_task
}

pub fn apply_task_lifecycle(root: &Path, tasks: &mut Vec<Value>) -> bool {
    let now = Utc::now();
    let now_iso = now.to_rfc3339();
    let mut changed = false;
    let mut spawned = Vec::<Value>::new();

    for row in tasks.iter_mut() {
        let status = parse_task_status(row);
        if matches!(status.as_str(), "completed" | "failed" | "timed_out" | "paused" | "cancelled" | "canceled" | "aborted") {
            continue;
        }
        if row.get("timeout_secs").is_none() {
            row["timeout_secs"] = Value::from(300);
            changed = true;
        }
        if row.get("completion_percent").is_none() {
            row["completion_percent"] = Value::from(0);
            changed = true;
        }
        let timeout_secs = parse_task_timeout_secs(row);
        let Some(started_at) = task_started_at(row) else {
            continue;
        };
        let elapsed = (now - started_at).num_seconds().max(0);
        let mut progress = parse_task_progress(row);
        let (swarm_progress, swarm_changed) = sync_swarm_progress(row);
        if swarm_changed {
            progress = swarm_progress;
            row["updated_at"] = Value::String(now_iso.clone());
            changed = true;
        }
        if parse_swarm_agents(row).is_empty() {
            let auto_progress = ((elapsed * 100) / timeout_secs).clamp(0, 95);
            if auto_progress > progress {
                progress = auto_progress;
                row["completion_percent"] = Value::from(progress);
                row["updated_at"] = Value::String(now_iso.clone());
                changed = true;
            }
        }
        if elapsed < timeout_secs {
            continue;
        }
        row["status"] = Value::String("timed_out".to_string());
        row["updated_at"] = Value::String(now_iso.clone());
        row["timed_out_at"] = Value::String(now_iso.clone());
        changed = true;

        let task_id =
            super::super::clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80);
        append_event(
            root,
            "task_timed_out",
            "Swarm",
            "",
            &format!("Task timed out at {}% progress", progress),
            Some(&task_id),
        );

        let auto_retry = row
            .get("auto_retry_on_timeout")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let retry_count = parse_task_retry(row);
        let max_retries = parse_task_max_retries(row);
        if auto_retry && retry_count < max_retries && progress < 100 {
            let retry_task = build_retry_task(row, now, retry_count + 1, true);
            let retry_id = super::super::clean_text(
                retry_task.get("id").and_then(Value::as_str).unwrap_or(""),
                80,
            );
            let pending_agents = retry_task
                .get("swarm_pending_agents")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            row["next_task_id"] = Value::String(retry_id.clone());
            spawned.push(retry_task);
            append_event(
                root,
                "task_rerun",
                "Swarm",
                "",
                &format!(
                    "Auto rerun started from {}% ({} agents pending)",
                    progress, pending_agents
                ),
                Some(&retry_id),
            );
        }
    }

    if !spawned.is_empty() {
        tasks.extend(spawned);
        changed = true;
    }
    if changed {
        tasks.sort_by_key(|row| {
            std::cmp::Reverse(super::super::clean_text(
                row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
                80,
            ))
        });
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timed_out_swarm_task_spawns_resumable_retry_with_carryover() {
        let root = tempfile::tempdir().expect("tempdir");
        let started = (Utc::now() - Duration::seconds(90)).to_rfc3339();
        let mut tasks = vec![json!({
            "id": "task-parent",
            "title": "Swarm Task",
            "status": "running",
            "completion_percent": 34,
            "created_at": started,
            "updated_at": started,
            "started_at": started,
            "deadline_at": started,
            "timeout_secs": 30,
            "retry_count": 0,
            "max_retries": 2,
            "auto_retry_on_timeout": true,
            "swarm_agent_ids": ["agent-a", "agent-b", "agent-c"],
            "completed_agent_ids": ["agent-a"],
            "pending_agent_ids": ["agent-b", "agent-c"],
            "partial_results": {"agent-a": "done"}
        })];
        let changed = apply_task_lifecycle(root.path(), &mut tasks);
        assert!(changed);
        assert!(tasks.len() >= 2);
        let parent = tasks
            .iter()
            .find(|row| row.get("id").and_then(Value::as_str) == Some("task-parent"))
            .expect("parent");
        assert_eq!(
            parent.get("status").and_then(Value::as_str),
            Some("timed_out")
        );
        let retry = tasks
            .iter()
            .find(|row| row.get("carryover_from").and_then(Value::as_str) == Some("task-parent"))
            .expect("retry task");
        assert_eq!(retry.get("retry_count").and_then(Value::as_i64), Some(1));
        assert_eq!(
            retry
                .get("swarm_pending_agents")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            2
        );
        assert_eq!(
            retry
                .pointer("/partial_results/agent-a")
                .and_then(Value::as_str),
            Some("done")
        );
    }
}

