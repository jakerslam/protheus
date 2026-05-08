use super::*;

fn unique_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "kernel-sentinel-scheduler-{name}-{}",
        crate::deterministic_receipt_hash(&json!({
            "test": name,
            "nonce": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        }))
    ))
}

#[test]
fn schedule_alias_coordinates_tick_and_preserves_schedule_artifact_type() {
    let root = unique_root("schedule-alias");
    let out = root.join("schedule.json");
    let auto = root.join("auto.json");
    let args = vec![
        "--strict=0".to_string(),
        "--force=1".to_string(),
        format!("--schedule-artifact={}", out.display()),
        format!("--auto-artifact={}", auto.display()),
    ];
    let exit = run_schedule(&root, &args);
    assert_eq!(exit, 0);
    let artifact: Value = serde_json::from_str(&fs::read_to_string(out).unwrap()).unwrap();
    assert_eq!(artifact["type"], "kernel_sentinel_schedule_run");
    assert_eq!(artifact["mode"], "schedule");
    assert_eq!(artifact["coordinator_mode"], "tick");
    assert_eq!(artifact["tick"], true);
}

#[test]
fn heartbeat_cascades_to_dream_when_dream_window_is_due() {
    let root = unique_root("heartbeat-dream");
    let out = root.join("heartbeat.json");
    let auto = root.join("heartbeat-auto.json");
    let args = vec![
        "--strict=0".to_string(),
        "--force=1".to_string(),
        "--dream-max-without-seconds=1".to_string(),
        format!("--schedule-artifact={}", out.display()),
        format!("--auto-artifact={}", auto.display()),
    ];
    let exit = run_heartbeat(&root, &args);
    assert_eq!(exit, 0);
    let artifact: Value = serde_json::from_str(&fs::read_to_string(out).unwrap()).unwrap();
    let state_dir = state_dir_from_args(&root, &args);
    let dream_state_path = state_path(&state_dir, SchedulerMode::Dream);
    assert_eq!(artifact["type"], "kernel_sentinel_heartbeat_run");
    assert_eq!(artifact["cascade"]["target"], "dream");
    assert_eq!(artifact["cascade"]["invoked"], true);
    assert_eq!(artifact["dream_gate"]["due"], true);
    assert!(dream_state_path.exists());
    assert!(auto.exists());
}

#[test]
fn dream_skips_auto_when_activity_is_recent_and_prior_dream_is_fresh() {
    let root = unique_root("dream-recent");
    let state_dir = root.join("local/state/kernel_sentinel");
    fs::create_dir_all(&state_dir).unwrap();
    let now = now_epoch_seconds();
    write_json(
        &state_path(&state_dir, SchedulerMode::Dream),
        &json!({
            "type": "kernel_sentinel_schedule_state",
            "last_attempt_epoch_secs": now,
            "last_success_epoch_secs": now,
            "last_exit_code": 0
        }),
    )
    .unwrap();
    let hands_dir = root.join("local/state/hands");
    fs::create_dir_all(&hands_dir).unwrap();
    fs::write(hands_dir.join("hand-a.json"), "{}\n").unwrap();
    let out = root.join("dream.json");
    let auto = root.join("dream-auto.json");
    let args = vec![
        "--strict=0".to_string(),
        "--dream-idle-seconds=3600".to_string(),
        "--dream-max-without-seconds=86400".to_string(),
        format!("--schedule-artifact={}", out.display()),
        format!("--auto-artifact={}", auto.display()),
    ];
    let exit = run_dream(&root, &args);
    assert_eq!(exit, 0);
    let artifact: Value = serde_json::from_str(&fs::read_to_string(out).unwrap()).unwrap();
    assert_eq!(artifact["type"], "kernel_sentinel_dream_run");
    assert_eq!(artifact["due"], false);
    assert_eq!(artifact["auto_run_invoked"], false);
    assert_eq!(artifact["dream_gate"]["due"], false);
    assert!(artifact.get("dream_system_cleanup").is_none());
    assert!(!auto.exists());
}

#[test]
fn dream_runs_system_cleanup_when_due() {
    let root = unique_root("dream-system-cleanup");
    let stale_target = root.join("target/debug/stale.bin");
    fs::create_dir_all(stale_target.parent().unwrap()).unwrap();
    fs::write(&stale_target, "stale").unwrap();
    std::env::set_var("SPINE_SLEEP_CLEANUP_MIN_INTERVAL_MINUTES", "0");
    std::env::set_var("SPINE_SLEEP_CLEANUP_TARGET_MAX_AGE_HOURS", "0");

    let out = root.join("dream-cleanup.json");
    let auto = root.join("dream-cleanup-auto.json");
    let args = vec![
        "--strict=0".to_string(),
        "--force=1".to_string(),
        "--dream-system-cleanup=1".to_string(),
        "--max-runtime-ms=60000".to_string(),
        format!("--schedule-artifact={}", out.display()),
        format!("--auto-artifact={}", auto.display()),
    ];
    let exit = run_dream(&root, &args);

    std::env::remove_var("SPINE_SLEEP_CLEANUP_MIN_INTERVAL_MINUTES");
    std::env::remove_var("SPINE_SLEEP_CLEANUP_TARGET_MAX_AGE_HOURS");

    assert_eq!(exit, 0);
    let artifact: Value = serde_json::from_str(&fs::read_to_string(out).unwrap()).unwrap();
    assert_eq!(artifact["type"], "kernel_sentinel_dream_run");
    assert_eq!(artifact["due"], true);
    assert_eq!(
        artifact["dream_system_cleanup"]["origin"],
        "kernel_sentinel_dream"
    );
    assert_eq!(artifact["dream_system_cleanup"]["exit_code"], 0);
    assert_eq!(artifact["dream_system_cleanup"]["ok"], true);
    assert!(
        !root.join("target").exists(),
        "dream system cleanup should prune stale build output"
    );
    assert_eq!(
        artifact["dream_maintenance_manifest"]["type"],
        "kernel_sentinel_dream_maintenance_manifest"
    );
    assert!(artifact["dream_maintenance_manifest"]["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|job| job["id"] == "spine_sleep_cleanup" && job["ran"] == true));
}

#[test]
fn dream_runs_memory_compression_when_due() {
    let root = unique_root("dream-memory-compress");
    let db_path = root.join("core/local/state/memory/runtime_memory.sqlite");
    fs::create_dir_all(db_path.parent().unwrap()).unwrap();
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE memories (
              id TEXT PRIMARY KEY,
              content TEXT NOT NULL,
              tags_json TEXT NOT NULL DEFAULT '[]',
              updated_at INTEGER NOT NULL,
              repetitions INTEGER NOT NULL DEFAULT 1,
              retention_score REAL NOT NULL DEFAULT 1.0
            );
            CREATE TABLE memory_cache (
              key TEXT PRIMARY KEY,
              payload TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            "#,
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memories (id, content, tags_json, updated_at, repetitions, retention_score) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params!["stale", "old low retention", "[]", 1i64, 1i64, 0.01f64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memories (id, content, tags_json, updated_at, repetitions, retention_score) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params!["kept", "fresh high retention", "[]", now_epoch_seconds() as i64, 1i64, 1.0f64],
        )
        .unwrap();
    }

    let out = root.join("dream-memory.json");
    let auto = root.join("dream-memory-auto.json");
    let args = vec![
        "--strict=0".to_string(),
        "--force=1".to_string(),
        "--dream-system-cleanup=0".to_string(),
        "--dream-memory-compress=1".to_string(),
        "--max-runtime-ms=60000".to_string(),
        format!("--schedule-artifact={}", out.display()),
        format!("--auto-artifact={}", auto.display()),
    ];
    let exit = run_dream(&root, &args);

    assert_eq!(exit, 0);
    let artifact: Value = serde_json::from_str(&fs::read_to_string(out).unwrap()).unwrap();
    assert_eq!(artifact["type"], "kernel_sentinel_dream_run");
    assert_eq!(
        artifact["dream_memory_compress"]["id"],
        "memory_sqlite_compress"
    );
    assert_eq!(artifact["dream_memory_compress"]["ok"], true);
    assert_eq!(artifact["dream_memory_compress"]["rows_removed"], 1);
    assert!(artifact["dream_maintenance_manifest"]["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|job| job["id"] == "memory_sqlite_compress" && job["ran"] == true));
}

#[test]
fn strict_dream_fails_when_previous_success_is_stale() {
    let root = unique_root("dream-stale");
    let state_dir = root.join("local/state/kernel_sentinel");
    fs::create_dir_all(&state_dir).unwrap();
    write_json(
        &state_path(&state_dir, SchedulerMode::Dream),
        &json!({
            "type": "kernel_sentinel_schedule_state",
            "last_success_epoch_secs": 1
        }),
    )
    .unwrap();
    let out = root.join("stale-dream.json");
    let args = vec![
        "--strict=1".to_string(),
        "--dream-idle-seconds=999999".to_string(),
        "--dream-max-without-seconds=999999".to_string(),
        "--stale-window-seconds=1".to_string(),
        format!("--schedule-artifact={}", out.display()),
    ];
    let exit = run_dream(&root, &args);
    assert_eq!(exit, 2);
    let artifact: Value = serde_json::from_str(&fs::read_to_string(out).unwrap()).unwrap();
    assert_eq!(artifact["stale"], true);
    assert_eq!(
        artifact["stale_escalation"]["reason"],
        "kernel_sentinel_auto_run_stale"
    );
    assert_eq!(artifact["dream_gate"]["due"], true);
    assert_eq!(artifact["auto_run_invoked"], true);
}

#[test]
fn health_summary_reports_unconfigured_when_no_state_exists() {
    let root = unique_root("health-unconfigured");
    let summary = build_scheduler_health_summary(&root, &[]);
    assert_eq!(summary["configured"], false);
    assert_eq!(summary["lifecycle_status"], "unconfigured");
    assert_eq!(summary["tick"]["lifecycle_status"], "unconfigured");
    assert_eq!(summary["heartbeat"]["lifecycle_status"], "unconfigured");
    assert_eq!(summary["dream"]["lifecycle_status"], "unconfigured");
}

#[test]
fn health_summary_reports_stale_and_degraded_modes_independently() {
    let root = unique_root("health-modes");
    let state_dir = root.join("local/state/kernel_sentinel");
    fs::create_dir_all(&state_dir).unwrap();
    let now = now_epoch_seconds();
    write_json(
        &state_path(&state_dir, SchedulerMode::Tick),
        &json!({
            "type": "kernel_sentinel_schedule_state",
            "last_attempt_epoch_secs": now,
            "last_success_epoch_secs": now,
            "last_exit_code": 0
        }),
    )
    .unwrap();
    write_json(
        &state_path(&state_dir, SchedulerMode::Heartbeat),
        &json!({
            "type": "kernel_sentinel_schedule_state",
            "last_attempt_epoch_secs": now,
            "last_success_epoch_secs": now,
            "last_exit_code": 9
        }),
    )
    .unwrap();
    write_json(
        &state_path(&state_dir, SchedulerMode::Dream),
        &json!({
            "type": "kernel_sentinel_schedule_state",
            "last_attempt_epoch_secs": 2,
            "last_success_epoch_secs": 1,
            "last_exit_code": 0
        }),
    )
    .unwrap();
    let args = vec!["--stale-window-seconds=1".to_string()];
    let summary = build_scheduler_health_summary(&root, &args);
    assert_eq!(summary["configured"], true);
    assert_eq!(summary["stale"], true);
    assert_eq!(summary["degraded"], true);
    assert_eq!(summary["tick"]["lifecycle_status"], "fresh");
    assert_eq!(summary["heartbeat"]["lifecycle_status"], "degraded");
    assert_eq!(summary["dream"]["lifecycle_status"], "stale");
}

#[test]
fn heartbeat_uses_legacy_schedule_state_as_fallback() {
    let root = unique_root("health-legacy");
    let state_dir = root.join("local/state/kernel_sentinel");
    fs::create_dir_all(&state_dir).unwrap();
    write_json(
        &legacy_state_path(&state_dir),
        &json!({
            "type": "kernel_sentinel_schedule_state",
            "last_attempt_epoch_secs": 5,
            "last_success_epoch_secs": 5,
            "last_exit_code": 0
        }),
    )
    .unwrap();
    let summary = build_scheduler_health_summary(&root, &[]);
    assert_eq!(summary["heartbeat"]["legacy_fallback_used"], true);
    assert_eq!(summary["dream"]["legacy_fallback_used"], false);
}
