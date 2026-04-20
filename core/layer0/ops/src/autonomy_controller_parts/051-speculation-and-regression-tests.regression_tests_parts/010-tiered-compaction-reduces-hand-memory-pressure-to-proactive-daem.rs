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
