use crate::task_graph::{ReadinessStatus, Task, TaskGraph};
use serde_json::{json, Value};

fn as_str(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn as_u64(args: &Value, key: &str, fallback: u64) -> u64 {
    args.get(key).and_then(Value::as_u64).unwrap_or(fallback)
}

fn matches_scope(task: &Task, scope_id: &str) -> bool {
    task.scope_id == scope_id
}

pub fn next_runnable(
    graph: &TaskGraph,
    scope_id: &str,
    assignee: Option<&str>,
    now_ms: u64,
    stale_after_ms: u64,
) -> Option<Task> {
    let mut rows = graph
        .tasks
        .values()
        .filter(|task| matches_scope(task, scope_id))
        .filter(|task| {
            assignee
                .map(|v| task.assignee.as_deref() == Some(v))
                .unwrap_or(true)
        })
        .filter(|task| {
            graph.derive_readiness(&task.id, now_ms, stale_after_ms)
                == Some(ReadinessStatus::Runnable)
        })
        .cloned()
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then(a.updated_at.cmp(&b.updated_at))
            .then(a.id.cmp(&b.id))
    });
    rows.into_iter().next()
}

pub fn subtree(graph: &TaskGraph, task_id: &str, depth: usize) -> Vec<Task> {
    let mut out = Vec::<Task>::new();
    let mut frontier = vec![(task_id.to_string(), 0usize)];
    while let Some((current, layer)) = frontier.pop() {
        if let Some(task) = graph.task(&current).cloned() {
            out.push(task);
            if layer < depth {
                let mut children = graph.children_of(&current);
                children.sort();
                for child in children.into_iter().rev() {
                    frontier.push((child, layer + 1));
                }
            }
        }
    }
    out
}

pub fn blocked_by(graph: &TaskGraph, task_id: &str, now_ms: u64, stale_after_ms: u64) -> Value {
    let readiness = graph
        .derive_readiness(task_id, now_ms, stale_after_ms)
        .map(|v| format!("{v:?}").to_ascii_lowercase())
        .unwrap_or_else(|| "unknown".to_string());
    let dependency_blockers = graph
        .dependencies_of(task_id)
        .into_iter()
        .filter(|dep| {
            graph
                .task(dep)
                .map(|row| row.lifecycle_status)
                .map(|v| v != crate::task_graph::LifecycleStatus::Completed)
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    let blocker_rows = graph
        .task(task_id)
        .map(|task| {
            task.blockers
                .iter()
                .filter(|row| !row.resolved)
                .map(|row| {
                    json!({
                        "blocker_id": row.blocker_id,
                        "kind": format!("{:?}", row.kind).to_ascii_lowercase(),
                        "reference_id": row.reference_id,
                        "reason": row.reason
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "task_id": task_id,
        "readiness": readiness,
        "dependencies": dependency_blockers,
        "blockers": blocker_rows
    })
}

pub fn claimable_tasks(
    graph: &TaskGraph,
    scope_id: &str,
    assignee: Option<&str>,
    now_ms: u64,
    stale_after_ms: u64,
) -> Vec<Task> {
    graph
        .tasks
        .values()
        .filter(|task| matches_scope(task, scope_id))
        .filter(|task| {
            assignee
                .map(|v| task.assignee.as_deref() == Some(v) || task.assignee.is_none())
                .unwrap_or(true)
        })
        .filter(|task| {
            matches!(
                graph.derive_readiness(&task.id, now_ms, stale_after_ms),
                Some(ReadinessStatus::Runnable | ReadinessStatus::Stale)
            )
        })
        .cloned()
        .collect::<Vec<_>>()
}

pub fn stale_tasks(graph: &TaskGraph, scope_id: &str, age_ms: u64, now_ms: u64) -> Vec<Task> {
    graph
        .tasks
        .values()
        .filter(|task| matches_scope(task, scope_id))
        .filter(|task| {
            task.last_heartbeat_at
                .map(|ts| now_ms.saturating_sub(ts) > age_ms)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>()
}

pub fn summary(graph: &TaskGraph, scope_id: &str, now_ms: u64, stale_after_ms: u64) -> Value {
    let mut lifecycle = json!({
        "pending": 0u64,
        "in_progress": 0u64,
        "review": 0u64,
        "completed": 0u64,
        "failed": 0u64,
        "cancelled": 0u64
    });
    let mut readiness = json!({
        "runnable": 0u64,
        "blocked": 0u64,
        "leased": 0u64,
        "stale": 0u64
    });
    let mut total = 0u64;
    for task in graph
        .tasks
        .values()
        .filter(|row| matches_scope(row, scope_id))
    {
        total = total.saturating_add(1);
        let key = format!("{:?}", task.lifecycle_status).to_ascii_lowercase();
        if let Some(current) = lifecycle.get(&key).and_then(Value::as_u64) {
            lifecycle[&key] = json!(current.saturating_add(1));
        }
        if let Some(state) = graph.derive_readiness(&task.id, now_ms, stale_after_ms) {
            let key = format!("{state:?}").to_ascii_lowercase();
            if let Some(current) = readiness.get(&key).and_then(Value::as_u64) {
                readiness[&key] = json!(current.saturating_add(1));
            }
        }
    }
    json!({
        "scope_id": scope_id,
        "total": total,
        "lifecycle": lifecycle,
        "readiness": readiness
    })
}

pub fn dispatch_named_query(
    graph: &TaskGraph,
    operation: &str,
    args: &Value,
    now_ms: u64,
    stale_after_ms: u64,
) -> Result<Value, String> {
    let op = operation.trim().to_ascii_lowercase();
    match op.as_str() {
        "next_runnable" => {
            let scope = as_str(args, "scope").ok_or_else(|| "scope_required".to_string())?;
            let assignee = as_str(args, "assignee");
            Ok(json!({
                "task": next_runnable(graph, &scope, assignee.as_deref(), now_ms, stale_after_ms)
            }))
        }
        "subtree" => {
            let task_id = as_str(args, "task_id").ok_or_else(|| "task_id_required".to_string())?;
            let depth = as_u64(args, "depth", 2).min(64) as usize;
            Ok(json!({
                "tasks": subtree(graph, &task_id, depth)
            }))
        }
        "blocked_by" => {
            let task_id = as_str(args, "task_id").ok_or_else(|| "task_id_required".to_string())?;
            Ok(blocked_by(graph, &task_id, now_ms, stale_after_ms))
        }
        "claimable_tasks" => {
            let scope = as_str(args, "scope").ok_or_else(|| "scope_required".to_string())?;
            let assignee = as_str(args, "assignee");
            Ok(json!({
                "tasks": claimable_tasks(graph, &scope, assignee.as_deref(), now_ms, stale_after_ms)
            }))
        }
        "stale_tasks" => {
            let scope = as_str(args, "scope").ok_or_else(|| "scope_required".to_string())?;
            let age_ms = as_u64(args, "age_ms", stale_after_ms);
            Ok(json!({
                "tasks": stale_tasks(graph, &scope, age_ms, now_ms)
            }))
        }
        "summary" => {
            let scope = as_str(args, "scope").ok_or_else(|| "scope_required".to_string())?;
            Ok(summary(graph, &scope, now_ms, stale_after_ms))
        }
        _ => Err("unsupported_named_query".to_string()),
    }
}
