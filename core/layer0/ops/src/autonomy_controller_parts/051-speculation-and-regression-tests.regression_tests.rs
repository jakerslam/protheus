#[test]
fn tiered_compaction_reduces_hand_memory_pressure() {
    let root = tempdir().expect("tmp");
    assert_eq!(
        run_hand_new(
            root.path(),
            &["hand-new".to_string(), "--hand-id=h1".to_string()]
        ),
        0
    );
    let path = hand_path(root.path(), "h1");
    let mut hand = read_json(&path).expect("hand");
    hand["memory"] = json!({
        "core": (0..40).map(|i| json!({"text": format!("core-{i}")})).collect::<Vec<_>>(),
        "archival": (0..80).map(|i| json!({"text": format!("arch-{i}")})).collect::<Vec<_>>(),
        "external": (0..64).map(|i| json!({"text": format!("ext-{i}")})).collect::<Vec<_>>()
    });
    write_json(&path, &hand).expect("write");
    assert_eq!(
        run_tiered_compaction(
            root.path(),
            &[
                "compact".to_string(),
                "--hand-id=h1".to_string(),
                "--mode=snip".to_string()
            ]
        ),
        0
    );
    let next = read_json(&path).expect("next");
    let core_len = next
        .pointer("/memory/core")
        .and_then(Value::as_array)
        .map(|v| v.len())
        .unwrap_or(0);
    assert!(core_len < 40);
}

#[test]
fn speculation_overlay_run_and_merge_updates_trunk_state() {
    let root = tempdir().expect("tmp");
    assert_eq!(
        run_speculation_overlay(
            root.path(),
            &[
                "speculate".to_string(),
                "run".to_string(),
                "--spec-id=s1".to_string(),
                "--input-json={\"plan\":\"test\"}".to_string()
            ]
        ),
        0
    );
    assert_eq!(
        run_speculation_overlay(
            root.path(),
            &[
                "speculate".to_string(),
                "merge".to_string(),
                "--spec-id=s1".to_string(),
                "--verify=1".to_string()
            ]
        ),
        0
    );
    let trunk = read_json(&trunk_state_path(root.path())).expect("trunk");
    let merged = trunk
        .pointer("/speculation_merges")
        .and_then(Value::as_array)
        .map(|v| v.len())
        .unwrap_or(0);
    assert_eq!(merged, 1);
}

#[test]
fn dream_consolidation_writes_four_phase_receipts() {
    let root = tempdir().expect("tmp");
    assert_eq!(
        run_hand_new(
            root.path(),
            &["hand-new".to_string(), "--hand-id=h2".to_string()]
        ),
        0
    );
    assert_eq!(
        run_dream_consolidation(
            root.path(),
            &["dream".to_string(), "--hand-id=h2".to_string()]
        ),
        0
    );
    let rows = read_jsonl(&dream_events_path(root.path()));
    assert!(!rows.is_empty());
    let phases = rows
        .last()
        .and_then(|row| row.pointer("/phase_receipts"))
        .and_then(Value::as_array)
        .map(|v| v.len())
        .unwrap_or(0);
    assert_eq!(phases, 4);
    let artifact_path = rows
        .last()
        .and_then(|row| row.get("semantic_artifact_path"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    assert!(!artifact_path.is_empty(), "semantic artifact path should be present");
    assert!(
        std::path::Path::new(&artifact_path).exists(),
        "semantic artifact should exist on disk"
    );
}

#[test]
fn proactive_daemon_pause_blocks_cycle_increment() {
    let root = tempdir().expect("tmp");
    assert_eq!(
        run_proactive_daemon_daemon(
            root.path(),
            &["proactive_daemon".to_string(), "pause".to_string()]
        ),
        0
    );
    assert_eq!(
        run_proactive_daemon_daemon(
            root.path(),
            &["proactive_daemon".to_string(), "cycle".to_string()]
        ),
        0
    );
    let state = read_json(&proactive_daemon_state_path(root.path())).expect("state");
    assert_eq!(state.get("paused").and_then(Value::as_bool), Some(true));
    assert_eq!(state.get("cycles").and_then(Value::as_u64), Some(0));
}

#[test]
fn proactive_daemon_cycle_emits_append_only_daily_log_and_state_write_confirmation() {
    let root = tempdir().expect("tmp");
    assert_eq!(
        run_proactive_daemon_daemon(
            root.path(),
            &[
                "proactive_daemon".to_string(),
                "cycle".to_string(),
                "--auto=1".to_string(),
                "--force=1".to_string(),
                "--brief=1".to_string(),
            ],
        ),
        0
    );
    let ymd: String = now_iso().chars().take(10).collect();
    let log_path = proactive_daemon_daily_log_path(root.path(), &ymd);
    let rows = read_jsonl(&log_path);
    assert!(
        !rows.is_empty(),
        "proactive_daemon daily log should append at least one row"
    );
    let state = read_json(&proactive_daemon_state_path(root.path())).expect("state");
    assert_eq!(
        state
            .pointer("/write_discipline/state_write_confirmed")
            .and_then(Value::as_bool),
        Some(true)
    );
}

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

#[test]
fn autoreason_blind_eval_hides_candidate_ids_from_blinded_surface() {
    let eval = autoreason_blind_evaluate(
        "ar-test",
        1,
        &[
            ("a_revised".to_string(), "candidate a body".to_string()),
            ("b_revised".to_string(), "candidate b body".to_string()),
            ("ab_synth".to_string(), "candidate ab body".to_string()),
        ],
        3,
    );
    let blinded = eval
        .get("blinded_candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!blinded.is_empty());
    assert!(blinded.iter().all(|row| row.get("candidate_id").is_none()));
    let winner = eval.get("winner_id").and_then(Value::as_str).unwrap_or("");
    assert!(matches!(winner, "a_revised" | "b_revised" | "ab_synth"));
}

#[test]
fn autoreason_conduit_bypass_is_rejected() {
    let root = tempdir().expect("tmp");
    assert_eq!(
        run_autoreason(
            root.path(),
            &[
                "autoreason".to_string(),
                "run".to_string(),
                "--task=t".to_string(),
                "--bypass=1".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        1
    );
}
