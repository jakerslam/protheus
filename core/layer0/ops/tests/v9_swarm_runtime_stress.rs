// SPDX-License-Identifier: Apache-2.0
use protheus_ops_core::swarm_runtime;
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
