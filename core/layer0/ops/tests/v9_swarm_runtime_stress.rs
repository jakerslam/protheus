// SPDX-License-Identifier: Apache-2.0
use infring_ops_core::swarm_runtime;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;

fn run_swarm(root: &std::path::Path, args: &[String]) -> i32 {
    swarm_runtime::run(root, args)
}

fn read_state(path: &std::path::Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("read state")).expect("parse state")
}

fn sessions_map<'a>(state: &'a Value) -> &'a serde_json::Map<String, Value> {
    state
        .get("sessions")
        .and_then(Value::as_object)
        .expect("sessions object")
}

fn session_id_by_task(state: &Value, task: &str) -> String {
    sessions_map(state)
        .iter()
        .find_map(|(session_id, row)| {
            (row.get("report")
                .and_then(|value| value.get("task"))
                .and_then(Value::as_str)
                == Some(task))
            .then(|| session_id.clone())
        })
        .expect("session id by task")
}

fn run_hierarchy_scenario(
    root: &std::path::Path,
    state_path: &std::path::Path,
    agents: u64,
    fanout: u64,
    task_prefix: &str,
) -> Value {
    let args = vec![
        "test".to_string(),
        "hierarchy".to_string(),
        format!("--agents={agents}"),
        format!("--fanout={fanout}"),
        "--metrics=detailed".to_string(),
        format!("--task-prefix={task_prefix}"),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root, &args), 0);
    read_state(state_path)
}

fn assert_hierarchy_invariants(state: &Value, task_prefix: &str, fanout_limit: usize) {
    let sessions = sessions_map(state);
    let root_task = format!("{task_prefix}-root");
    let root_session_id = sessions
        .iter()
        .find_map(|(session_id, row)| {
            (row.get("parent_id").is_some() && row.get("parent_id") == Some(&Value::Null))
                .then(|| session_id.clone())
        })
        .or_else(|| {
            sessions.iter().find_map(|(session_id, row)| {
                (row.get("task").and_then(Value::as_str) == Some(root_task.as_str()))
                    .then(|| session_id.clone())
            })
        })
        .expect("hierarchy root session id");

    let mut child_counts = std::collections::BTreeMap::new();
    let mut orphan_count = 0usize;
    let mut missing_parent_count = 0usize;
    for (session_id, row) in sessions {
        if session_id == &root_session_id {
            continue;
        }
        let parent_id = row.get("parent_id").and_then(Value::as_str);
        match parent_id {
            Some(parent) => {
                if !sessions.contains_key(parent) {
                    missing_parent_count += 1;
                } else {
                    *child_counts.entry(parent.to_string()).or_insert(0usize) += 1;
                }
            }
            None => orphan_count += 1,
        }
    }

    assert_eq!(
        orphan_count, 0,
        "hierarchy should not contain orphan children"
    );
    assert_eq!(
        missing_parent_count, 0,
        "hierarchy should not contain missing-parent links"
    );
    let fanout_violations = child_counts
        .values()
        .filter(|count| **count > fanout_limit)
        .count();
    assert_eq!(
        fanout_violations, 0,
        "hierarchy should respect configured fanout"
    );
}

#[test]
fn stress_concurrency_48_agents_preserves_session_invariants() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let args = vec![
        "test".to_string(),
        "concurrency".to_string(),
        "--agents=48".to_string(),
        "--metrics=detailed".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &args), 0);

    let state = read_state(&state_path);
    let sessions = sessions_map(&state);
    assert!(
        sessions.len() >= 48,
        "expected at least 48 sessions, got {}",
        sessions.len()
    );

    let unique_ids = sessions.keys().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        unique_ids.len(),
        sessions.len(),
        "session identifiers must be unique"
    );

    let metrics_complete = sessions.values().all(|session| {
        let Some(metrics) = session.get("metrics") else {
            return false;
        };
        metrics.get("queue_wait_ms").is_some()
            && metrics.get("execution_end_ms").is_some()
            && metrics.get("report_back_latency_ms").is_some()
    });
    assert!(
        metrics_complete,
        "detailed concurrency run must persist timing metrics for all sessions"
    );

    let dead_letters = state
        .get("dead_letters")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert!(
        dead_letters <= 2,
        "unexpected dead-letter spike during spawn stress: {dead_letters}"
    );
}

#[test]
fn stress_bulk_ttl_expiry_retry_restores_delivery() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let parent_args = vec![
        "spawn".to_string(),
        "--task=stress-parent".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &parent_args), 0);

    let state = read_state(&state_path);
    let parent_id = sessions_map(&state)
        .keys()
        .next()
        .cloned()
        .expect("parent id");

    for task in ["stress-sender", "stress-receiver"] {
        let args = vec![
            "spawn".to_string(),
            format!("--task={task}"),
            format!("--session-id={parent_id}"),
            format!("--state-path={}", state_path.display()),
        ];
        assert_eq!(run_swarm(root.path(), &args), 0);
    }

    let state = read_state(&state_path);
    let sender = session_id_by_task(&state, "stress-sender");
    let receiver = session_id_by_task(&state, "stress-receiver");

    for idx in 0..40 {
        let send_args = vec![
            "sessions".to_string(),
            "send".to_string(),
            format!("--sender-id={sender}"),
            format!("--session-id={receiver}"),
            format!("--message=expire-burst-{idx}"),
            "--delivery=at_least_once".to_string(),
            "--ttl-ms=1".to_string(),
            format!("--state-path={}", state_path.display()),
        ];
        assert_eq!(run_swarm(root.path(), &send_args), 0);
    }

    std::thread::sleep(std::time::Duration::from_millis(20));
    let status_args = vec![
        "status".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &status_args), 0);

    let state = read_state(&state_path);
    let dead_letters = state
        .get("dead_letters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        dead_letters.len() >= 10,
        "expected TTL expiry to generate dead letters; got {}",
        dead_letters.len()
    );

    let retry_ids = dead_letters
        .iter()
        .take(10)
        .filter_map(|row| {
            row.get("message")
                .and_then(|msg| msg.get("message_id"))
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .collect::<Vec<_>>();
    assert_eq!(retry_ids.len(), 10, "expected retry ids");

    for message_id in &retry_ids {
        let retry_args = vec![
            "sessions".to_string(),
            "retry-dead-letter".to_string(),
            format!("--message-id={message_id}"),
            format!("--state-path={}", state_path.display()),
        ];
        assert_eq!(run_swarm(root.path(), &retry_args), 0);
    }

    let receive_args = vec![
        "sessions".to_string(),
        "receive".to_string(),
        format!("--session-id={receiver}"),
        "--limit=200".to_string(),
        "--mark-read=0".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &receive_args), 0);

    let state_after = read_state(&state_path);
    let unread_after_retry = state_after
        .get("mailboxes")
        .and_then(|rows| rows.get(&receiver))
        .and_then(|row| row.get("unread"))
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert!(
        unread_after_retry >= retry_ids.len(),
        "expected retried dead letters to be delivered back into inbox"
    );
}

#[test]
fn stress_role_fanout_burst_keeps_mailboxes_consistent() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");
    let roles = [
        "generator",
        "filter",
        "filter",
        "summarizer",
        "filter",
        "validator",
    ];

    for idx in 0..18 {
        let role = roles[idx % roles.len()];
        let args = vec![
            "spawn".to_string(),
            format!("--task=fanout-{idx}"),
            format!("--role={role}"),
            format!("--state-path={}", state_path.display()),
        ];
        assert_eq!(run_swarm(root.path(), &args), 0);
    }

    let state = read_state(&state_path);
    let filter_targets = state
        .get("service_registry")
        .and_then(|rows| rows.get("filter"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        !filter_targets.is_empty(),
        "expected filter targets in service registry"
    );

    for idx in 0..25 {
        let send_role_args = vec![
            "sessions".to_string(),
            "send-role".to_string(),
            "--sender-id=coordinator".to_string(),
            "--role=filter".to_string(),
            format!("--message=fanout-burst-{idx}"),
            "--delivery=at_least_once".to_string(),
            format!("--state-path={}", state_path.display()),
        ];
        assert_eq!(run_swarm(root.path(), &send_role_args), 0);
    }

    let state_after = read_state(&state_path);
    let expected_min = 25usize;
    let delivered_total = filter_targets
        .iter()
        .filter_map(|row| row.get("session_id").and_then(Value::as_str))
        .map(|session_id| {
            state_after
                .get("mailboxes")
                .and_then(|rows| rows.get(session_id))
                .and_then(|row| row.get("unread"))
                .and_then(Value::as_array)
                .map(|rows| rows.len())
                .unwrap_or(0)
        })
        .sum::<usize>();
    assert!(
        delivered_total >= expected_min,
        "fanout delivery shortfall: delivered={delivered_total}, expected_min={expected_min}"
    );

    let dead_letters = state_after
        .get("dead_letters")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert_eq!(
        dead_letters, 0,
        "fanout burst should not dead-letter reachable sessions"
    );
}

#[test]
fn stress_role_fanout_round_robin_prevents_single_mailbox_hotspot() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    for idx in 0..24 {
        let args = vec![
            "spawn".to_string(),
            format!("--task=round-robin-filter-{idx}"),
            "--role=filter".to_string(),
            format!("--state-path={}", state_path.display()),
        ];
        assert_eq!(run_swarm(root.path(), &args), 0);
    }

    for idx in 0..480 {
        let send_role_args = vec![
            "sessions".to_string(),
            "send-role".to_string(),
            "--sender-id=coordinator".to_string(),
            "--role=filter".to_string(),
            format!("--message=round-robin-burst-{idx}"),
            "--delivery=at_least_once".to_string(),
            "--ttl-ms=3600000".to_string(),
            format!("--state-path={}", state_path.display()),
        ];
        assert_eq!(run_swarm(root.path(), &send_role_args), 0);
    }

    let state = read_state(&state_path);
    let filter_targets = state
        .get("service_registry")
        .and_then(|rows| rows.get("filter"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert_eq!(filter_targets.len(), 24, "expected 24 filter workers");

    let unread_counts = filter_targets
        .iter()
        .filter_map(|row| row.get("session_id").and_then(Value::as_str))
        .map(|session_id| {
            state
                .get("mailboxes")
                .and_then(|rows| rows.get(session_id))
                .and_then(|row| row.get("unread"))
                .and_then(Value::as_array)
                .map(|rows| rows.len())
                .unwrap_or(0)
        })
        .collect::<Vec<_>>();

    let active_recipients = unread_counts.iter().filter(|count| **count > 0).count();
    let max_unread = unread_counts.into_iter().max().unwrap_or(0);
    assert!(
        active_recipients >= 12,
        "role dispatch hotspot detected: active_recipients={active_recipients}"
    );
    assert!(
        max_unread <= 24,
        "single mailbox overload detected: max_unread={max_unread}"
    );

    let dead_letters = state
        .get("dead_letters")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert_eq!(
        dead_letters, 0,
        "round-robin role dispatch should avoid dead letters for this load"
    );
}

#[test]
fn stress_hierarchy_256_agents_preserves_parent_lineage_and_fanout_bounds() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");
    let state = run_hierarchy_scenario(root.path(), &state_path, 256, 8, "hierarchy-regression");
    let sessions = sessions_map(&state);
    assert!(
        sessions.len() >= 256,
        "expected at least 256 sessions, got {}",
        sessions.len()
    );
    assert_hierarchy_invariants(&state, "hierarchy-regression", 8);
}

#[test]
fn stress_hierarchy_4096_agents_preserves_parent_lineage_and_fanout_bounds() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");
    let state = run_hierarchy_scenario(
        root.path(),
        &state_path,
        4096,
        16,
        "hierarchy-brutal-regression",
    );
    let sessions = sessions_map(&state);
    assert!(
        sessions.len() >= 4096,
        "expected at least 4096 sessions, got {}",
        sessions.len()
    );
    assert_hierarchy_invariants(&state, "hierarchy-brutal-regression", 16);
}

#[test]
fn stress_role_fanout_1024_messages_stays_balanced_and_deadletter_free() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    for idx in 0..64 {
        let args = vec![
            "spawn".to_string(),
            format!("--task=storm-filter-{idx}"),
            "--role=filter".to_string(),
            format!("--state-path={}", state_path.display()),
        ];
        assert_eq!(run_swarm(root.path(), &args), 0);
    }

    for idx in 0..1024 {
        let send_role_args = vec![
            "sessions".to_string(),
            "send-role".to_string(),
            "--sender-id=coordinator".to_string(),
            "--role=filter".to_string(),
            format!("--message=storm-burst-{idx}"),
            "--delivery=at_least_once".to_string(),
            "--ttl-ms=3600000".to_string(),
            format!("--state-path={}", state_path.display()),
        ];
        assert_eq!(run_swarm(root.path(), &send_role_args), 0);
    }

    let state = read_state(&state_path);
    let filter_targets = state
        .get("service_registry")
        .and_then(|rows| rows.get("filter"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert_eq!(filter_targets.len(), 64, "expected 64 filter workers");

    let unread_counts = filter_targets
        .iter()
        .filter_map(|row| row.get("session_id").and_then(Value::as_str))
        .map(|session_id| {
            state
                .get("mailboxes")
                .and_then(|rows| rows.get(session_id))
                .and_then(|row| row.get("unread"))
                .and_then(Value::as_array)
                .map(|rows| rows.len())
                .unwrap_or(0)
        })
        .collect::<Vec<_>>();

    let active_recipients = unread_counts.iter().filter(|count| **count > 0).count();
    let max_unread = unread_counts.iter().copied().max().unwrap_or(0);
    let min_unread = unread_counts.iter().copied().min().unwrap_or(0);
    assert!(
        active_recipients >= 48,
        "distribution too narrow under storm load: active_recipients={active_recipients}"
    );
    assert!(
        max_unread <= 24,
        "mailbox hotspot under storm load: max_unread={max_unread}"
    );
    assert!(
        min_unread >= 8,
        "distribution skew under storm load: min_unread={min_unread}"
    );

    let dead_letters = state
        .get("dead_letters")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert_eq!(
        dead_letters, 0,
        "storm load should remain dead-letter free with balanced dispatch"
    );
}

#[test]
fn stress_scale_policy_plan_and_set_support_100k_readiness() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let plan_args = vec![
        "scale".to_string(),
        "plan".to_string(),
        "--agents=100000".to_string(),
        "--fanout=32".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &plan_args), 0);

    let set_args = vec![
        "scale".to_string(),
        "set".to_string(),
        "--max-sessions=250000".to_string(),
        "--max-children-per-parent=64".to_string(),
        "--max-depth-hard=96".to_string(),
        "--target-ready-agents=100000".to_string(),
        "--enforce-session-cap=1".to_string(),
        "--enforce-parent-capacity=1".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &set_args), 0);

    let status_args = vec![
        "scale".to_string(),
        "status".to_string(),
        "--agents=100000".to_string(),
        "--fanout=32".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &status_args), 0);

    let state = read_state(&state_path);
    let policy = state
        .get("scale_policy")
        .and_then(Value::as_object)
        .expect("scale policy");
    assert_eq!(
        policy.get("max_sessions_hard").and_then(Value::as_u64),
        Some(250_000)
    );
    assert_eq!(
        policy
            .get("max_children_per_parent")
            .and_then(Value::as_u64),
        Some(64)
    );
    assert_eq!(
        policy.get("target_ready_agents").and_then(Value::as_u64),
        Some(100_000)
    );
}

#[test]
fn stress_scale_parent_capacity_guard_blocks_overloaded_manager() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let set_args = vec![
        "scale".to_string(),
        "set".to_string(),
        "--max-children-per-parent=2".to_string(),
        "--enforce-parent-capacity=1".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &set_args), 0);

    let root_spawn = vec![
        "spawn".to_string(),
        "--task=root".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &root_spawn), 0);

    let state = read_state(&state_path);
    let root_id = sessions_map(&state)
        .keys()
        .next()
        .cloned()
        .expect("root id");

    for idx in 0..2 {
        let child_spawn = vec![
            "spawn".to_string(),
            format!("--task=child-{idx}"),
            format!("--session-id={root_id}"),
            format!("--state-path={}", state_path.display()),
        ];
        assert_eq!(run_swarm(root.path(), &child_spawn), 0);
    }

    let third_child = vec![
        "spawn".to_string(),
        "--task=child-2".to_string(),
        format!("--session-id={root_id}"),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(
        run_swarm(root.path(), &third_child),
        2,
        "expected parent-capacity guard to fail the third child spawn"
    );
}
