
#[test]
fn proactive_daemon_policy_tiered_tool_surfaces_emit_conduit_receipts() {
    let root = tempdir().expect("tmp");
    assert_eq!(
        run_proactive_daemon_daemon(
            root.path(),
            &[
                "proactive_daemon".to_string(),
                "cycle".to_string(),
                "--auto=1".to_string(),
                "--force=1".to_string(),
                "--policy-tier=execute".to_string(),
                "--tool-surfaces=subscribe_pr,push_notification,send_user_file".to_string(),
                "--max-proactive=8".to_string(),
                "--block-budget-ms=5000".to_string(),
            ],
        ),
        0
    );
    let state = read_json(&proactive_daemon_state_path(root.path())).expect("state");
    assert!(
        state
            .pointer("/tool_surfaces/receipts_written")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 1
    );
    let receipt_rows = read_jsonl(&proactive_daemon_tool_receipts_path(root.path()));
    assert!(!receipt_rows.is_empty(), "expected proactive tool receipts");
    assert!(receipt_rows.iter().all(|row| {
        row.get("transport").and_then(Value::as_str) == Some("conduit")
    }));
}

#[test]
fn proactive_daemon_failure_isolation_quarantines_failed_intent_without_poisoning_other_tasks() {
    let root = tempdir().expect("tmp");
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
                "--dream-max-without-ms=60000".to_string(),
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
    assert!(deferred.iter().any(|row| {
        row.get("reason").and_then(Value::as_str) == Some("dream_failed")
            && row.pointer("/failure_isolation/failure_class").and_then(Value::as_str)
                == Some("runtime_fault")
    }));
    assert!(state
        .pointer("/failure_isolation/quarantine")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().any(|row| row.get("task").and_then(Value::as_str) == Some("dream_consolidation")))
        .unwrap_or(false));
    let executed = state
        .pointer("/last_executed_intents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(executed.iter().any(|row| {
        row.pointer("/intent/task").and_then(Value::as_str) == Some("pattern_log")
    }));
}

#[test]
fn proactive_daemon_recovery_matrix_records_strategy_ladder() {
    let root = tempdir().expect("tmp");
    let args = &[
        "proactive_daemon".to_string(),
        "cycle".to_string(),
        "--auto=1".to_string(),
        "--force=1".to_string(),
        "--max-proactive=8".to_string(),
        "--block-budget-ms=5000".to_string(),
        "--dream-max-without-ms=60000".to_string(),
    ];
    assert_eq!(run_proactive_daemon_daemon(root.path(), args), 0);
    assert_eq!(run_proactive_daemon_daemon(root.path(), args), 0);
    let state = read_json(&proactive_daemon_state_path(root.path())).expect("state");
    let history = state
        .pointer("/recovery_matrix/history")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!history.is_empty(), "recovery history should be recorded");
    assert!(history.iter().any(|row| {
        row.get("strategy").and_then(Value::as_str) == Some("retry")
    }));
    assert!(history.iter().any(|row| {
        row.get("strategy").and_then(Value::as_str) == Some("resync")
    }));
}

#[test]
fn proactive_daemon_triggers_dream_and_cleanup_when_inactive() {
    let root = tempdir().expect("tmp");
    assert_eq!(
        run_hand_new(
            root.path(),
            &["hand-new".to_string(), "--hand-id=h-dream".to_string()],
        ),
        0
    );
    let hand_path = hand_path(root.path(), "h-dream");
    let mut hand = read_json(&hand_path).expect("hand");
    hand["updated_at"] = json!("2000-01-01T00:00:00Z");
    write_json(&hand_path, &hand).expect("write hand");

    let target_file = root.path().join("target/debug/stale.bin");
    fs::create_dir_all(target_file.parent().expect("target parent")).expect("target dir");
    fs::write(&target_file, "stale").expect("target write");

    std::env::set_var("SPINE_SLEEP_CLEANUP_MIN_INTERVAL_MINUTES", "0");
    std::env::set_var("SPINE_SLEEP_CLEANUP_TARGET_MAX_AGE_HOURS", "0");

    assert_eq!(
        run_proactive_daemon_daemon(
            root.path(),
            &[
                "proactive_daemon".to_string(),
                "cycle".to_string(),
                "--auto=1".to_string(),
                "--force=1".to_string(),
                "--max-proactive=8".to_string(),
                "--block-budget-ms=40000".to_string(),
                "--dream-idle-ms=60000".to_string(),
                "--dream-max-without-ms=60000".to_string(),
            ],
        ),
        0
    );

    let state = read_json(&proactive_daemon_state_path(root.path())).expect("state");
    assert!(
        state
            .pointer("/dream/last_dream_at_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
    );
    assert_eq!(
        state
            .pointer("/dream/last_dream_reason")
            .and_then(Value::as_str),
        Some("inactivity")
    );
    let executed = state
        .pointer("/last_executed_intents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(executed.iter().any(|row| {
        row.pointer("/intent/task").and_then(Value::as_str) == Some("dream_consolidation")
    }));
    assert!(
        !root.path().join("target").exists(),
        "sleep cleanup should run as part of dream execution"
    );

    std::env::remove_var("SPINE_SLEEP_CLEANUP_MIN_INTERVAL_MINUTES");
    std::env::remove_var("SPINE_SLEEP_CLEANUP_TARGET_MAX_AGE_HOURS");
}

#[test]
fn autoreason_run_persists_state_and_iterations() {
    let root = tempdir().expect("tmp");
    assert_eq!(
        run_autoreason(
            root.path(),
            &[
                "autoreason".to_string(),
                "run".to_string(),
                "--task=improve launch strategy".to_string(),
                "--convergence=2".to_string(),
                "--max-iters=6".to_string(),
                "--judges=3".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    let state = read_json(&autoreason_state_path(root.path())).expect("autoreason state");
    assert_eq!(state.get("total_runs").and_then(Value::as_u64), Some(1));
    let run_id = state
        .pointer("/last_run/run_id")
        .and_then(Value::as_str)
        .expect("run id");
    let rows = read_jsonl(&autoreason_run_log_path(root.path(), run_id));
    assert!(
        !rows.is_empty(),
        "autoreason run should persist iteration rows"
    );
}
