
#[test]
fn persistent_mode_supports_tick_wake_terminate_and_metrics() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let spawn_args = vec![
        "spawn".to_string(),
        "--task=swarm-test-5-persistent-health".to_string(),
        "--execution-mode=persistent".to_string(),
        "--lifespan-sec=30".to_string(),
        "--check-in-interval-sec=5".to_string(),
        "--report-mode=always".to_string(),
        "--token-budget=2000".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &spawn_args), 0);

    let mut state = read_state(&state_path);
    let sessions = state
        .get("sessions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    assert_eq!(sessions.len(), 1, "expected one persistent session");
    let session_id = sessions.keys().next().cloned().expect("session id");
    let initial_check_ins = sessions
        .get(&session_id)
        .and_then(|row| row.get("check_ins"))
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert!(
        initial_check_ins >= 1,
        "expected initial check-in at spawn time"
    );

    let tick_args = vec![
        "tick".to_string(),
        "--advance-ms=7000".to_string(),
        "--max-check-ins=8".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &tick_args), 0);

    state = read_state(&state_path);
    let post_tick_check_ins = state
        .get("sessions")
        .and_then(|rows| rows.get(&session_id))
        .and_then(|row| row.get("check_ins"))
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert!(
        post_tick_check_ins >= 2,
        "expected additional check-in after tick"
    );

    let wake_args = vec![
        "sessions".to_string(),
        "wake".to_string(),
        format!("--session-id={session_id}"),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &wake_args), 0);

    state = read_state(&state_path);
    let post_wake_check_ins = state
        .get("sessions")
        .and_then(|rows| rows.get(&session_id))
        .and_then(|row| row.get("check_ins"))
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert!(
        post_wake_check_ins >= 3,
        "expected manual wake to record check-in"
    );

    let metrics_args = vec![
        "sessions".to_string(),
        "metrics".to_string(),
        format!("--session-id={session_id}"),
        "--timeline=1".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &metrics_args), 0);

    let anomalies_args = vec![
        "sessions".to_string(),
        "anomalies".to_string(),
        format!("--session-id={session_id}"),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &anomalies_args), 0);

    let terminate_args = vec![
        "sessions".to_string(),
        "terminate".to_string(),
        format!("--session-id={session_id}"),
        "--graceful=1".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &terminate_args), 0);

    state = read_state(&state_path);
    let session = state
        .get("sessions")
        .and_then(|rows| rows.get(&session_id))
        .cloned()
        .unwrap_or(Value::Null);
    assert_eq!(
        session.get("status").and_then(Value::as_str),
        Some("terminated_graceful")
    );
    assert!(
        session
            .get("persistent")
            .and_then(|value| value.get("terminated_at_ms"))
            .and_then(Value::as_u64)
            .is_some(),
        "expected terminated_at_ms in persistent runtime"
    );
}

#[test]
fn sessions_state_command_returns_session_introspection() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let spawn_args = vec![
        "spawn".to_string(),
        "--task=swarm-test-state-introspection".to_string(),
        "--role=calculator".to_string(),
        "--capabilities=calculate,verify".to_string(),
        "--token-budget=2000".to_string(),
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

    let state_args = vec![
        "sessions".to_string(),
        "state".to_string(),
        format!("--session-id={session_id}"),
        "--timeline=1".to_string(),
        "--tool-history-limit=10".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &state_args), 0);
}

#[test]
fn queue_metrics_command_supports_prometheus_format() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    for idx in 0..3 {
        let spawn_args = vec![
            "spawn".to_string(),
            format!("--task=swarm-test-metrics-{idx}"),
            format!("--state-path={}", state_path.display()),
        ];
        assert_eq!(run_swarm(root.path(), &spawn_args), 0);
    }

    let metrics_args = vec![
        "metrics".to_string(),
        "queue".to_string(),
        "--format=prometheus".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &metrics_args), 0);
}

#[test]
fn background_worker_start_status_stop_lifecycle() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let start_args = vec![
        "background".to_string(),
        "start".to_string(),
        "--task=background-worker-health".to_string(),
        "--execution-mode=background".to_string(),
        "--lifespan-sec=60".to_string(),
        "--check-in-interval-sec=10".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &start_args), 0);

    let status_args = vec![
        "background".to_string(),
        "status".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &status_args), 0);

    let mut state = read_state(&state_path);
    let sessions = state
        .get("sessions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let (session_id, session_row) = sessions
        .iter()
        .find(|(_, row)| {
            row.get("background_worker")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .expect("background worker session");
    assert_eq!(
        session_row.get("status").and_then(Value::as_str),
        Some("background_running")
    );

    let stop_args = vec![
        "background".to_string(),
        "stop".to_string(),
        format!("--session-id={session_id}"),
        "--graceful=1".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &stop_args), 0);

    state = read_state(&state_path);
    assert_eq!(
        state
            .get("sessions")
            .and_then(|rows| rows.get(session_id))
            .and_then(|row| row.get("status"))
            .and_then(Value::as_str),
        Some("terminated_graceful")
    );
}

#[test]
fn scheduled_tasks_add_and_run_due_generate_sessions() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let add_args = vec![
        "scheduled".to_string(),
        "add".to_string(),
        "--task=scheduled-health-check".to_string(),
        "--interval-sec=1".to_string(),
        "--runs=1".to_string(),
        "--max-runtime-sec=2".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &add_args), 0);

    let status_args = vec![
        "scheduled".to_string(),
        "status".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &status_args), 0);

    let run_due_args = vec![
        "scheduled".to_string(),
        "run-due".to_string(),
        "--advance-ms=2000".to_string(),
        "--max-runs=1".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &run_due_args), 0);

    let state = read_state(&state_path);
    let tasks = state
        .get("scheduled_tasks")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    assert_eq!(tasks.len(), 1, "expected one scheduled task");
    let task = tasks.values().next().cloned().unwrap_or(Value::Null);
    assert_eq!(
        task.get("remaining_runs").and_then(Value::as_u64),
        Some(0),
        "expected scheduled task run budget exhausted"
    );
    assert_eq!(task.get("active").and_then(Value::as_bool), Some(false));
    assert!(
        task.get("last_session_id")
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false),
        "expected scheduled task to record a spawned session"
    );
    let session_count = state
        .get("sessions")
        .and_then(Value::as_object)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert!(session_count >= 1, "expected spawned session from run-due");
}

#[test]
fn persistent_test_suite_creates_check_in_timeline() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let args = vec![
        "test".to_string(),
        "persistent".to_string(),
        "--lifespan-sec=20".to_string(),
        "--check-in-interval-sec=5".to_string(),
        "--advance-ms=10000".to_string(),
        format!("--state-path={}", state_path.display()),
    ];
    assert_eq!(run_swarm(root.path(), &args), 0);

    let state = read_state(&state_path);
    let sessions = state
        .get("sessions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    assert!(!sessions.is_empty(), "expected at least one session");
    let check_in_counts = sessions
        .values()
        .filter_map(|row| row.get("check_ins").and_then(Value::as_array))
        .map(|rows| rows.len())
        .collect::<Vec<_>>();
    assert!(
        check_in_counts.iter().any(|count| *count >= 2),
        "expected persistent test lane to produce timeline check-ins"
    );
}

#[test]
fn spawn_payload_exposes_authoritative_tool_manifest() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let spawn_args = vec![
        "spawn".to_string(),
        "--task=tool-manifest-proof".to_string(),
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
    let session = state
        .get("sessions")
        .and_then(|rows| rows.get(&session_id))
        .cloned()
        .unwrap_or(Value::Null);
    let tool_access = session
        .get("tool_access")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        tool_access
            .iter()
            .any(|row| row.as_str() == Some("sessions_send")),
        "expected sessions_send in authoritative tool access"
    );
}

#[test]
fn sessions_bootstrap_exposes_generic_agent_contract() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let spawn_args = vec![
        "spawn".to_string(),
        "--task=bootstrap-contract-proof".to_string(),
        "--token-budget=240".to_string(),
        "--on-budget-exhausted=fail".to_string(),
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

    let bootstrap_args = vec![
        "sessions_bootstrap".to_string(),
        format!("--sessionKey={session_id}"),
        format!("--state-path={}", state_path.display()),
    ];
    let payload = run_bridge_cli(&bootstrap_args);
    let bootstrap = payload.get("bootstrap").cloned().unwrap_or(Value::Null);
    assert_eq!(
        payload.get("session_id").and_then(Value::as_str),
        Some(session_id.as_str())
    );
    assert_eq!(
        bootstrap.get("version").and_then(Value::as_str),
        Some("swarm-agent-bootstrap/v1")
    );
    assert!(
        bootstrap
            .get("commands")
            .and_then(|row| row.get("sessions_send"))
            .and_then(Value::as_str)
            .map(|row| row.contains("sessions_send"))
            .unwrap_or(false),
        "expected bootstrap command surface to include sessions_send"
    );
    assert_eq!(
        bootstrap
            .get("budget")
            .and_then(|row| row.get("on_budget_exhausted"))
            .and_then(Value::as_str),
        Some("fail")
    );
    assert!(
        bootstrap
            .get("prompt")
            .and_then(Value::as_str)
            .map(|row| row.contains("Use direct swarm bridge commands"))
            .unwrap_or(false),
        "expected bootstrap prompt to direct generic agents to bridge commands"
    );
}

