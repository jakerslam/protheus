
fn first_session_id(state: &Value) -> String {
    state
        .get("sessions")
        .and_then(Value::as_object)
        .and_then(|rows| rows.keys().next())
        .cloned()
        .expect("session id")
}

fn spawn_session(
    root: &std::path::Path,
    state_path: &std::path::Path,
    task: &str,
    parent_session: Option<&str>,
    extra_flags: &[&str],
) {
    let mut args = vec![
        "spawn".to_string(),
        format!("--task={task}"),
        format!("--state-path={}", state_path.display()),
    ];
    if let Some(parent) = parent_session {
        args.push(format!("--session-id={parent}"));
    }
    for flag in extra_flags {
        args.push((*flag).to_string());
    }
    assert_eq!(run_swarm(root, &args), 0);
}

#[test]
fn child_budget_reservation_settles_into_parent_budget() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    spawn_session(
        root.path(),
        &state_path,
        "parent-budget",
        None,
        &["--token-budget=500"],
    );

    let state = read_state(&state_path);
    let parent_id = first_session_id(&state);
    spawn_session(
        root.path(),
        &state_path,
        "child-budget",
        Some(&parent_id),
        &["--token-budget=200"],
    );

    let state = read_state(&state_path);
    let parent_budget = state
        .get("sessions")
        .and_then(|rows| rows.get(&parent_id))
        .and_then(|row| row.get("budget_telemetry"))
        .cloned()
        .unwrap_or(Value::Null);
    assert!(
        parent_budget
            .get("settled_child_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0,
        "expected settled child token usage in parent telemetry"
    );
}

#[test]
fn dead_letter_messages_can_be_retried() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    spawn_session(root.path(), &state_path, "dlq-parent", None, &[]);
    let state = read_state(&state_path);
    let parent_id = first_session_id(&state);

    for task in ["dlq-sender", "dlq-receiver"] {
        spawn_session(root.path(), &state_path, task, Some(&parent_id), &[]);
    }

    let state = read_state(&state_path);
    let sender = find_session_id_by_task(&state, "dlq-sender").expect("sender");
    let receiver = find_session_id_by_task(&state, "dlq-receiver").expect("receiver");

    let send_args = vec![
        "sessions".to_string(),
        "send".to_string(),
        format!("--sender-id={sender}"),
        format!("--session-id={receiver}"),
        "--message=expire-me".to_string(),
        "--delivery=at_least_once".to_string(),
        "--ttl-ms=1".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &send_args), 0);
    let status_args = vec![
        "status".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert_eq!(run_swarm(root.path(), &status_args), 0);

    let dead_letter_args = vec![
        "sessions".to_string(),
        "dead-letter".to_string(),
        format!("--session-id={receiver}"),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &dead_letter_args), 0);

    let state = read_state(&state_path);
    let message_id = state
        .get("dead_letters")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("message"))
        .and_then(|row| row.get("message_id"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .expect("dead letter message id");

    let retry_args = vec![
        "sessions".to_string(),
        "retry-dead-letter".to_string(),
        format!("--message-id={message_id}"),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &retry_args), 0);
}

#[test]
fn persistent_session_resume_restores_running_status() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    spawn_session(
        root.path(),
        &state_path,
        "resume-persistent",
        None,
        &[
            "--execution-mode=persistent",
            "--lifespan-sec=60",
            "--check-in-interval-sec=5",
        ],
    );

    let state = read_state(&state_path);
    let session_id = first_session_id(&state);

    let resume_args = vec![
        "sessions".to_string(),
        "resume".to_string(),
        format!("--session-id={session_id}"),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &resume_args), 0);

    let state = read_state(&state_path);
    assert_eq!(
        state
            .get("sessions")
            .and_then(|rows| rows.get(&session_id))
            .and_then(|row| row.get("status"))
            .and_then(Value::as_str),
        Some("persistent_running")
    );
}

