// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

use super::CompatApiResponse;

#[path = "dashboard_compat_api_comms_store.rs"]
mod dashboard_compat_api_comms_store;

fn agent_name_map(root: &Path, snapshot: &Value) -> HashMap<String, String> {
    let mut out = HashMap::<String, String>::new();
    for row in super::build_agent_roster(root, snapshot, true) {
        let id = super::clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
        if id.is_empty() {
            continue;
        }
        let name = super::clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 120);
        if name.is_empty() {
            out.insert(id.clone(), super::humanize_agent_name(&id));
        } else {
            out.insert(id.clone(), name);
        }
    }
    out
}

fn agent_name_from_map(map: &HashMap<String, String>, agent_id: &str, fallback: &str) -> String {
    let id = super::clean_agent_id(agent_id);
    if id.is_empty() {
        return fallback.to_string();
    }
    map.get(&id)
        .cloned()
        .unwrap_or_else(|| super::humanize_agent_name(&id))
}

fn request_swarm_agent_ids(request: &Value) -> Vec<String> {
    for key in ["swarm_agent_ids", "agent_ids", "agents"] {
        let parsed = dashboard_compat_api_comms_store::parse_agent_ids(request.get(key));
        if !parsed.is_empty() {
            return parsed;
        }
    }
    Vec::new()
}

fn topology_payload(root: &Path, snapshot: &Value) -> Value {
    let roster = super::build_agent_roster(root, snapshot, false);
    let mut nodes = Vec::<Value>::new();
    let mut edges = Vec::<Value>::new();
    for row in roster {
        let id = super::clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
        if id.is_empty() {
            continue;
        }
        let provider = super::clean_text(
            row.get("model_provider")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        let model = super::clean_text(
            row.get("model_name").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        nodes.push(json!({
            "id": id,
            "name": super::clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 120),
            "state": super::clean_text(row.get("state").and_then(Value::as_str).unwrap_or("Idle"), 40),
            "model": if provider.is_empty() || model.is_empty() { model.clone() } else { format!("{provider}/{model}") }
        }));
        let parent = super::clean_agent_id(
            row.get("parent_agent_id")
                .and_then(Value::as_str)
                .or_else(|| {
                    row.pointer("/contract/parent_agent_id")
                        .and_then(Value::as_str)
                })
                .unwrap_or(""),
        );
        if !parent.is_empty() {
            edges.push(json!({"kind": "parent_child", "from": parent, "to": id}));
        }
    }
    json!({
        "ok": true,
        "nodes": nodes,
        "edges": edges,
        "connected": true
    })
}

pub fn handle(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    body: &[u8],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    if method == "GET" && path_only == "/api/comms/topology" {
        return Some(CompatApiResponse {
            status: 200,
            payload: topology_payload(root, snapshot),
        });
    }

    if method == "GET" && path_only == "/api/comms/events" {
        let mut events = dashboard_compat_api_comms_store::read_events(root);
        let limit = super::query_value(path, "limit")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(200)
            .clamp(1, 1000);
        if events.len() > limit {
            events.truncate(limit);
        }
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "events": events}),
        });
    }

    if method == "GET" && path_only == "/api/comms/tasks" {
        let mut tasks = dashboard_compat_api_comms_store::read_tasks(root);
        if dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks) {
            dashboard_compat_api_comms_store::write_tasks(root, &tasks);
        }
        let limit = super::query_value(path, "limit")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(200)
            .clamp(1, 1000);
        if tasks.len() > limit {
            tasks.truncate(limit);
        }
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "tasks": tasks}),
        });
    }

    if method == "POST" && path_only == "/api/comms/send" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let from_id = super::clean_agent_id(
            request
                .get("from_agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
        );
        let to_id = super::clean_agent_id(
            request
                .get("to_agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
        );
        let message = super::clean_text(
            request.get("message").and_then(Value::as_str).unwrap_or(""),
            2_000,
        );
        if from_id.is_empty() || to_id.is_empty() || message.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "invalid_message"}),
            });
        }
        let names = agent_name_map(root, snapshot);
        dashboard_compat_api_comms_store::append_event(
            root,
            "agent_message",
            &agent_name_from_map(&names, &from_id, "Agent"),
            &agent_name_from_map(&names, &to_id, "Agent"),
            &message,
            None,
        );
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true}),
        });
    }

    if method == "POST" && path_only == "/api/comms/task" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let title = super::clean_text(
            request.get("title").and_then(Value::as_str).unwrap_or(""),
            200,
        );
        if title.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "title_required"}),
            });
        }
        let description = super::clean_text(
            request
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or(""),
            4_000,
        );
        let assigned_to = super::clean_agent_id(
            request
                .get("assigned_to")
                .and_then(Value::as_str)
                .unwrap_or(""),
        );
        let mut swarm_agent_ids = request_swarm_agent_ids(&request);
        if !assigned_to.is_empty() && !swarm_agent_ids.iter().any(|row| row == &assigned_to) {
            swarm_agent_ids.push(assigned_to.clone());
        }
        let timeout_secs = request
            .get("timeout_secs")
            .and_then(Value::as_i64)
            .unwrap_or(300)
            .clamp(15, 86_400);
        let max_retries = request
            .get("max_retries")
            .and_then(Value::as_i64)
            .unwrap_or(1)
            .clamp(0, 20);
        let auto_retry_on_timeout = request
            .get("auto_retry_on_timeout")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let now = Utc::now();
        let now_iso = now.to_rfc3339();
        let deadline = (now + Duration::seconds(timeout_secs)).to_rfc3339();
        let seed = json!({"kind":"task","title":title,"assigned_to":assigned_to,"ts":now_iso});
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
            "max_retries": max_retries,
            "auto_retry_on_timeout": auto_retry_on_timeout,
            "swarm_agent_ids": swarm_agent_ids,
            "completed_agent_ids": [],
            "pending_agent_ids": [],
            "partial_results": {}
        });
        let _ = dashboard_compat_api_comms_store::sync_swarm_progress(&mut task);
        let mut tasks = dashboard_compat_api_comms_store::read_tasks(root);
        tasks.insert(0, task.clone());
        dashboard_compat_api_comms_store::write_tasks(root, &tasks);
        let task_id = super::clean_text(task.get("id").and_then(Value::as_str).unwrap_or(""), 80);
        dashboard_compat_api_comms_store::append_event(
            root,
            "task_posted",
            "Swarm",
            "",
            &format!(
                "{} (timeout {}s)",
                task.get("title").and_then(Value::as_str).unwrap_or("Task"),
                timeout_secs
            ),
            Some(&task_id),
        );
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "task": task}),
        });
    }

    if method == "POST" && path_only.starts_with("/api/comms/task/") {
        let tail = path_only
            .trim_start_matches("/api/comms/task/")
            .trim_matches('/');
        let mut parts = tail.split('/').filter(|part| !part.trim().is_empty());
        let task_id = super::clean_text(parts.next().unwrap_or(""), 80);
        let action = super::clean_text(parts.next().unwrap_or(""), 40).to_ascii_lowercase();
        if task_id.is_empty() || action.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "invalid_task_route"}),
            });
        }
        let mut tasks = dashboard_compat_api_comms_store::read_tasks(root);
        let Some(idx) = tasks.iter().position(|row| {
            super::clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80) == task_id
        }) else {
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "task_not_found"}),
            });
        };
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let now_iso = crate::now_iso();

        if action == "rerun" {
            let retry_count = dashboard_compat_api_comms_store::parse_task_retry(&tasks[idx]) + 1;
            let retry_task = dashboard_compat_api_comms_store::build_retry_task(
                &tasks[idx],
                Utc::now(),
                retry_count,
                false,
            );
            let retry_id = super::clean_text(
                retry_task.get("id").and_then(Value::as_str).unwrap_or(""),
                80,
            );
            tasks[idx]["next_task_id"] = Value::String(retry_id.clone());
            tasks[idx]["updated_at"] = Value::String(now_iso.clone());
            tasks.insert(0, retry_task.clone());
            dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            dashboard_compat_api_comms_store::append_event(
                root,
                "task_rerun",
                "Swarm",
                "",
                "Manual rerun started",
                Some(&retry_id),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "task": retry_task}),
            });
        }

        if action == "progress" {
            let completed_agents = dashboard_compat_api_comms_store::parse_agent_ids(
                request
                    .get("completed_agent_ids")
                    .or_else(|| request.get("completed_agents")),
            );
            dashboard_compat_api_comms_store::merge_completed_agent_ids(
                &mut tasks[idx],
                &completed_agents,
            );
            let pending_agents =
                dashboard_compat_api_comms_store::parse_agent_ids(request.get("pending_agent_ids"));
            if !pending_agents.is_empty() {
                dashboard_compat_api_comms_store::override_pending_agent_ids(
                    &mut tasks[idx],
                    &pending_agents,
                );
            }
            dashboard_compat_api_comms_store::merge_partial_results(
                &mut tasks[idx],
                request
                    .get("partial_results")
                    .or_else(|| request.get("results")),
            );
            let explicit_percent = request
                .get("completion_percent")
                .and_then(Value::as_i64)
                .map(|value| value.clamp(0, 100));
            let (swarm_percent, _swarm_changed) =
                dashboard_compat_api_comms_store::sync_swarm_progress(&mut tasks[idx]);
            let has_swarm = tasks[idx]
                .get("swarm_agent_ids")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false);
            let percent = if has_swarm {
                swarm_percent.clamp(0, 100)
            } else {
                explicit_percent
                    .unwrap_or(dashboard_compat_api_comms_store::parse_task_progress(
                        &tasks[idx],
                    ))
                    .clamp(0, 100)
            };
            if dashboard_compat_api_comms_store::parse_task_progress(&tasks[idx]) != percent {
                tasks[idx]["completion_percent"] = Value::from(percent);
            }
            tasks[idx]["updated_at"] = Value::String(now_iso.clone());
            tasks[idx]["status"] = Value::String(
                if percent >= 100 {
                    "completed"
                } else {
                    "running"
                }
                .to_string(),
            );
            if percent >= 100 {
                tasks[idx]["completed_at"] = Value::String(now_iso.clone());
            }
            if let Some(summary) = request.get("result_summary").and_then(Value::as_str) {
                tasks[idx]["result_summary"] = Value::String(super::clean_text(summary, 2_000));
            }
            dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            dashboard_compat_api_comms_store::append_event(
                root,
                if percent >= 100 {
                    "task_completed"
                } else {
                    "task_progress"
                },
                "Swarm",
                "",
                &format!("Progress updated to {}%", percent),
                Some(&task_id),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "task": tasks[idx].clone()}),
            });
        }

        if action == "complete" {
            if let Some(swarm_ids) = tasks[idx].get("swarm_agent_ids").cloned() {
                tasks[idx]["completed_agent_ids"] = swarm_ids;
                tasks[idx]["pending_agent_ids"] = Value::Array(Vec::new());
                let _ = dashboard_compat_api_comms_store::sync_swarm_progress(&mut tasks[idx]);
            }
            tasks[idx]["completion_percent"] = Value::from(100);
            tasks[idx]["status"] = Value::String("completed".to_string());
            tasks[idx]["updated_at"] = Value::String(now_iso);
            tasks[idx]["completed_at"] = tasks[idx]
                .get("updated_at")
                .cloned()
                .unwrap_or_else(|| json!(crate::now_iso()));
            if let Some(summary) = request.get("result_summary").and_then(Value::as_str) {
                tasks[idx]["result_summary"] = Value::String(super::clean_text(summary, 2_000));
            }
            dashboard_compat_api_comms_store::write_tasks(root, &tasks);
            dashboard_compat_api_comms_store::append_event(
                root,
                "task_completed",
                "Swarm",
                "",
                "Task marked completed",
                Some(&task_id),
            );
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "task": tasks[idx].clone()}),
            });
        }

        return Some(CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "error": "task_action_not_found"}),
        });
    }

    None
}
