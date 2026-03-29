
#[test]
fn child_budget_reservation_settles_into_parent_budget() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let parent_args = vec![
        "spawn".to_string(),
        "--task=parent-budget".to_string(),
        "--token-budget=500".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &parent_args), 0);

    let state = read_state(&state_path);
    let parent_id = state
        .get("sessions")
        .and_then(Value::as_object)
        .and_then(|rows| rows.keys().next())
        .cloned()
        .expect("parent id");

    let child_args = vec![
        "spawn".to_string(),
        "--task=child-budget".to_string(),
        format!("--session-id={parent_id}"),
        "--token-budget=200".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &child_args), 0);

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

    let parent_args = vec![
        "spawn".to_string(),
        "--task=dlq-parent".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &parent_args), 0);
    let state = read_state(&state_path);
    let parent_id = state
        .get("sessions")
        .and_then(Value::as_object)
        .and_then(|rows| rows.keys().next())
        .cloned()
        .expect("parent id");

    for task in ["dlq-sender", "dlq-receiver"] {
        let args = vec![
            "spawn".to_string(),
            format!("--task={task}"),
            format!("--session-id={parent_id}"),
            format!("--state-path={}", state_path.display()),
        ];
        assert_eq!(run_swarm(root.path(), &args), 0);
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

    let spawn_args = vec![
        "spawn".to_string(),
        "--task=resume-persistent".to_string(),
        "--execution-mode=persistent".to_string(),
        "--lifespan-sec=60".to_string(),
        "--check-in-interval-sec=5".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &spawn_args), 0);

    let state = read_state(&state_path);
    let session_id = state
        .get("sessions")
        .and_then(Value::as_object)
        .and_then(|rows| rows.keys().next())
        .cloned()
        .expect("session id");

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


