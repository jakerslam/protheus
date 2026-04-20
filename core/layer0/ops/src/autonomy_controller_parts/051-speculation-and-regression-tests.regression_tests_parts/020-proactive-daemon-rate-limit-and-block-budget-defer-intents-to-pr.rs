
#[test]
fn proactive_daemon_rate_limit_and_block_budget_defer_intents() {
    let root = tempdir().expect("tmp");
    assert_eq!(
        run_hand_new(
            root.path(),
            &["hand-new".to_string(), "--hand-id=h-limited".to_string()],
        ),
        0
    );
    let hand_path = hand_path(root.path(), "h-limited");
    let mut hand = read_json(&hand_path).expect("hand");
    hand["memory"]["core"] = Value::Array(
        (0..120)
            .map(|idx| json!({"text": format!("core-{idx}")}))
            .collect(),
    );
    write_json(&hand_path, &hand).expect("write hand");

    let swarm_path = root
        .path()
        .join("local/state/ops/swarm_runtime/latest.json");
    fs::create_dir_all(swarm_path.parent().expect("parent")).expect("mkdir");
    write_json(
        &swarm_path,
        &json!({
            "dead_letters": [json!({"id":"d1"})],
            "sessions": {
                "s1": {},
                "s2": {}
            }
        }),
    )
    .expect("swarm");

    assert_eq!(
        run_proactive_daemon_daemon(
            root.path(),
            &[
                "proactive_daemon".to_string(),
                "cycle".to_string(),
                "--auto=1".to_string(),
                "--force=1".to_string(),
                "--max-proactive=1".to_string(),
                "--block-budget-ms=100".to_string(),
            ],
        ),
        0
    );
    let state = read_json(&proactive_daemon_state_path(root.path())).expect("state");
    let deferred = state
        .pointer("/last_deferred_intents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        deferred.iter().any(|row| {
            matches!(
                row.get("reason").and_then(Value::as_str),
                Some("blocking_budget" | "rate_limit")
            )
        }),
        "expected at least one deferred reason from budget/rate limiting"
    );
}

#[test]
fn proactive_daemon_auto_compaction_uses_reactive_threshold_near_ninety_five_percent() {
    let root = tempdir().expect("tmp");
    assert_eq!(
        run_hand_new(
            root.path(),
            &["hand-new".to_string(), "--hand-id=h-reactive".to_string()],
        ),
        0
    );
    let hand_path = hand_path(root.path(), "h-reactive");
    let mut hand = read_json(&hand_path).expect("hand");
    hand["memory"]["core"] = Value::Array(
        (0..120)
            .map(|idx| json!({"text": format!("core-{idx}")}))
            .collect(),
    );
    write_json(&hand_path, &hand).expect("write hand");
    assert_eq!(
        run_proactive_daemon_daemon(
            root.path(),
            &[
                "proactive_daemon".to_string(),
                "cycle".to_string(),
                "--auto=1".to_string(),
                "--force=1".to_string(),
                "--max-proactive=8".to_string(),
                "--block-budget-ms=5000".to_string(),
            ],
        ),
        0
    );
    let state = read_json(&proactive_daemon_state_path(root.path())).expect("state");
    let executed = state
        .pointer("/last_executed_intents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let compact_row = executed
        .iter()
        .find(|row| {
            row.pointer("/intent/task").and_then(Value::as_str) == Some("compact_hand_memory")
        })
        .cloned()
        .expect("compact intent executed");
    assert_eq!(
        compact_row.get("pressure_ratio").and_then(Value::as_f64),
        Some(0.95)
    );
}

#[test]
fn proactive_daemon_heartbeat_tick_gate_prevents_early_cycle_reentry() {
    let root = tempdir().expect("tmp");
    assert_eq!(
        run_proactive_daemon_daemon(
            root.path(),
            &[
                "proactive_daemon".to_string(),
                "cycle".to_string(),
                "--tick-ms=60000".to_string(),
                "--force=1".to_string(),
            ],
        ),
        0
    );
    let first = read_json(&proactive_daemon_state_path(root.path())).expect("state first");
    assert_eq!(first.get("cycles").and_then(Value::as_u64), Some(1));
    assert_eq!(
        run_proactive_daemon_daemon(
            root.path(),
            &[
                "proactive_daemon".to_string(),
                "cycle".to_string(),
                "--tick-ms=60000".to_string(),
            ],
        ),
        0
    );
    let second = read_json(&proactive_daemon_state_path(root.path())).expect("state second");
    assert_eq!(second.get("cycles").and_then(Value::as_u64), Some(1));
    assert_eq!(
        second.get("last_decision").and_then(Value::as_str),
        Some("tick_deferred")
    );
}

#[test]
fn proactive_daemon_daily_log_is_append_only_across_cycles() {
    let root = tempdir().expect("tmp");
    let args = &[
        "proactive_daemon".to_string(),
        "cycle".to_string(),
        "--force=1".to_string(),
        "--auto=1".to_string(),
    ];
    assert_eq!(run_proactive_daemon_daemon(root.path(), args), 0);
    assert_eq!(run_proactive_daemon_daemon(root.path(), args), 0);
    let ymd: String = now_iso().chars().take(10).collect();
    let rows = read_jsonl(&proactive_daemon_daily_log_path(root.path(), &ymd));
    assert!(
        rows.len() >= 2,
        "expected append-only proactive_daemon log to retain multiple cycle rows"
    );
}

#[test]
fn proactive_daemon_pattern_logger_records_bounded_hints() {
    let root = tempdir().expect("tmp");
    let swarm_path = root
        .path()
        .join("local/state/ops/swarm_runtime/latest.json");
    fs::create_dir_all(swarm_path.parent().expect("parent")).expect("mkdir");
    write_json(
        &swarm_path,
        &json!({
            "dead_letters": [json!({"id":"d1"}), json!({"id":"d2"})],
            "sessions": {
                "s1": {}, "s2": {}, "s3": {}
            }
        }),
    )
    .expect("swarm");
    assert_eq!(
        run_proactive_daemon_daemon(
            root.path(),
            &[
                "proactive_daemon".to_string(),
                "cycle".to_string(),
                "--auto=1".to_string(),
                "--force=1".to_string(),
                "--max-proactive=8".to_string(),
            ],
        ),
        0
    );
    let state = read_json(&proactive_daemon_state_path(root.path())).expect("state");
    let suggestions = state
        .pointer("/pattern_log/suggestions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!suggestions.is_empty(), "pattern suggestions should be recorded");
    assert!(suggestions.iter().any(|row| row
        .get("hint")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("dead-letter")));
}
