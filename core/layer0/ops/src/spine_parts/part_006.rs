
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parity_fixture_evidence_plan_matches_ts_rules() {
        let a = compute_evidence_run_plan(Some(2), Some("none"), Some("none"));
        assert_eq!(a.get("evidence_runs").and_then(Value::as_i64), Some(2));

        let b = compute_evidence_run_plan(Some(2), Some("soft"), Some("none"));
        assert_eq!(
            b.get("pressure_throttle").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(b.get("evidence_runs").and_then(Value::as_i64), Some(1));

        let c = compute_evidence_run_plan(Some(4), Some("none"), Some("hard"));
        assert_eq!(
            c.get("pressure_throttle").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(c.get("evidence_runs").and_then(Value::as_i64), Some(1));
    }

    #[test]
    fn deterministic_receipt_hash_for_fixture() {
        let payload = json!({
            "ok": true,
            "type": "spine_run_complete",
            "mode": "eyes",
            "date": "2026-03-04",
            "claim_evidence": [{"id":"c1","claim":"x","evidence":{"a":1}}]
        });
        let h1 = receipt_hash(&payload);
        let h2 = receipt_hash(&payload);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn terminal_failure_receipt_is_emitted_with_claim_evidence_and_hash() {
        let root = tempdir().expect("tempdir");
        let cli = CliArgs {
            command: "run".to_string(),
            mode: "eyes".to_string(),
            date: "2026-03-04".to_string(),
            max_eyes: None,
        };
        let run_id = "spine_test_1";
        let mut ledger = LedgerWriter::new(root.path(), &cli.date, run_id);
        let evidence_plan = default_evidence_plan();
        let constitution_hash = Some("abc123".to_string());
        let policy = MechSuitPolicy {
            enabled: true,
            heartbeat_hours: 4,
            manual_triggers_allowed: false,
            quiet_non_critical: false,
            silent_subprocess_output: true,
            push_attention_queue: true,
            attention_queue_path: "local/state/attention/queue.jsonl".to_string(),
            attention_receipts_path: "local/state/attention/receipts.jsonl".to_string(),
            attention_latest_path: "local/state/attention/latest.json".to_string(),
            attention_max_queue_depth: 2048,
            attention_ttl_hours: 48,
            attention_dedupe_window_hours: 24,
            attention_backpressure_drop_below: "critical".to_string(),
            attention_escalate_levels: vec!["critical".to_string()],
            ambient_stance: true,
            dopamine_threshold_breach_only: true,
            status_path: root
                .path()
                .join("local/state/ops/mech_suit_mode/latest.json"),
            history_path: root
                .path()
                .join("local/state/ops/mech_suit_mode/history.jsonl"),
            policy_path: root
                .path()
                .join("client/runtime/config/mech_suit_mode_policy.json"),
        };
        let context = TerminalReceiptContext {
            run_id,
            cli: &cli,
            policy: &policy,
            constitution_hash: &constitution_hash,
            constitution_ok: true,
            evidence_plan: &evidence_plan,
            evidence_ok: 0,
            started_ms: 0,
        };

        let code = emit_terminal_receipt(&mut ledger, &context, false, Some("guard_failed"));
        assert_eq!(code, 1);

        let latest_path = root
            .path()
            .join("client/runtime/local/state/spine/runs/latest.json");
        let latest_raw = std::fs::read_to_string(latest_path).expect("latest json");
        let latest = serde_json::from_str::<Value>(&latest_raw).expect("valid json");

        assert_eq!(
            latest.get("type").and_then(Value::as_str),
            Some("spine_run_failed")
        );
        assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(false));
        assert!(latest.get("claim_evidence").is_some());
        assert!(latest.get("persona_lenses").is_some());
        assert_eq!(
            latest.get("failure_reason").and_then(Value::as_str),
            Some("guard_failed")
        );

        let expected_hash = latest
            .get("receipt_hash")
            .and_then(Value::as_str)
            .expect("hash")
            .to_string();
        let mut unhashed = latest.clone();
        let unhashed_obj = unhashed.as_object_mut().expect("object");
        unhashed_obj.remove("receipt_hash");
        // Ledger metadata is added after hash calculation for the terminal payload.
        unhashed_obj.remove("ledger_seq");
        assert_eq!(receipt_hash(&unhashed), expected_hash);
    }

    #[test]
    fn parse_cli_supports_run_alias() {
        let args = vec![
            "run".to_string(),
            "daily".to_string(),
            "2026-03-04".to_string(),
            "--max-eyes=7".to_string(),
        ];
        let parsed = parse_cli(&args).expect("parsed");
        assert_eq!(parsed.mode, "daily");
        assert_eq!(parsed.date, "2026-03-04");
        assert_eq!(parsed.max_eyes, Some(7));
    }

    #[test]
    fn parse_cli_supports_split_max_eyes_flag() {
        let args = vec![
            "eyes".to_string(),
            "2026-03-04".to_string(),
            "--max-eyes".to_string(),
            "12".to_string(),
        ];
        let parsed = parse_cli(&args).expect("parsed");
        assert_eq!(parsed.mode, "eyes");
        assert_eq!(parsed.date, "2026-03-04");
        assert_eq!(parsed.max_eyes, Some(12));
    }

    #[test]
    fn parse_cli_supports_status_overrides() {
        let args = vec![
            "status".to_string(),
            "--mode=eyes".to_string(),
            "--date=2026-03-05".to_string(),
        ];
        let parsed = parse_cli(&args).expect("parsed");
        assert_eq!(parsed.command, "status");
        assert_eq!(parsed.mode, "eyes");
        assert_eq!(parsed.date, "2026-03-05");
    }

    #[test]
    fn ambient_gate_receipt_is_hashed_and_fail_closed() {
        let policy = MechSuitPolicy {
            enabled: true,
            heartbeat_hours: 4,
            manual_triggers_allowed: false,
            quiet_non_critical: true,
            silent_subprocess_output: true,
            push_attention_queue: true,
            attention_queue_path: "local/state/attention/queue.jsonl".to_string(),
            attention_receipts_path: "local/state/attention/receipts.jsonl".to_string(),
            attention_latest_path: "local/state/attention/latest.json".to_string(),
            attention_max_queue_depth: 2048,
            attention_ttl_hours: 48,
            attention_dedupe_window_hours: 24,
            attention_backpressure_drop_below: "critical".to_string(),
            attention_escalate_levels: vec!["critical".to_string()],
            ambient_stance: true,
            dopamine_threshold_breach_only: true,
            status_path: PathBuf::from("local/state/ops/mech_suit_mode/latest.json"),
            history_path: PathBuf::from("local/state/ops/mech_suit_mode/history.jsonl"),
            policy_path: PathBuf::from("client/runtime/config/mech_suit_mode_policy.json"),
        };
        let cli = CliArgs {
            command: "run".to_string(),
            mode: "eyes".to_string(),
            date: "2026-03-05".to_string(),
            max_eyes: None,
        };
        let receipt = ambient_gate_blocked_receipt(&cli, &policy, "manual");
        assert_eq!(receipt.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(receipt.get("blocked").and_then(Value::as_bool), Some(true));
        assert_eq!(
            receipt.get("reason").and_then(Value::as_str),
            Some("manual_trigger_blocked_mech_suit_mode")
        );

        let expected_hash = receipt
            .get("receipt_hash")
            .and_then(Value::as_str)
            .expect("hash")
            .to_string();
        let mut unhashed = receipt.clone();
        unhashed
            .as_object_mut()
            .expect("object")
            .remove("receipt_hash");
        assert_eq!(receipt_hash(&unhashed), expected_hash);
    }

    #[test]
    fn cli_error_receipt_is_deterministic_and_hashed() {
        let argv = vec!["bad".to_string(), "--x=1".to_string()];
        let out = cli_error_receipt(&argv, "invalid_args", 2);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("spine_cli_error")
        );
        assert!(out.get("claim_evidence").is_some());
        assert!(out.get("persona_lenses").is_some());

        let expected_hash = out
            .get("receipt_hash")
            .and_then(Value::as_str)
            .expect("hash")
            .to_string();
        let mut unhashed = out.clone();
        unhashed
            .as_object_mut()
            .expect("object")
            .remove("receipt_hash");
        assert_eq!(receipt_hash(&unhashed), expected_hash);

        let ts = out.get("ts").and_then(Value::as_str).expect("ts");
        let date = out.get("date").and_then(Value::as_str).expect("date");
        assert!(ts.starts_with(date));
    }

    #[test]
    fn sleep_cleanup_run_removes_old_archive_and_target() {
        let root = tempdir().expect("tempdir");
        let archive_dir = root.path().join("local/workspace/archive/churn-a");
        let target_file = root.path().join("target/debug/stale.bin");
        fs::create_dir_all(&archive_dir).expect("archive");
        fs::create_dir_all(target_file.parent().expect("target parent")).expect("target dir");
        fs::write(archive_dir.join("receipt.json"), "{}").expect("archive write");
        fs::write(&target_file, "stale").expect("target write");

        std::env::set_var("SPINE_SLEEP_CLEANUP_ARCHIVE_KEEP_LATEST", "0");
        std::env::set_var("SPINE_SLEEP_CLEANUP_ARCHIVE_MAX_AGE_HOURS", "0");
        std::env::set_var("SPINE_SLEEP_CLEANUP_TARGET_MAX_AGE_HOURS", "0");
        std::env::set_var("SPINE_SLEEP_CLEANUP_MIN_INTERVAL_MINUTES", "0");

        let (code, out) = execute_sleep_cleanup(root.path(), true, true, "test");
        assert_eq!(code, 0);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));

        assert!(!root.path().join("local/workspace/archive/churn-a").exists());
        assert!(!root.path().join("target").exists());
        assert!(root
            .path()
            .join("client/runtime/local/state/ops/sleep_cleanup/latest.json")
            .exists());

        std::env::remove_var("SPINE_SLEEP_CLEANUP_ARCHIVE_KEEP_LATEST");
        std::env::remove_var("SPINE_SLEEP_CLEANUP_ARCHIVE_MAX_AGE_HOURS");
        std::env::remove_var("SPINE_SLEEP_CLEANUP_TARGET_MAX_AGE_HOURS");
        std::env::remove_var("SPINE_SLEEP_CLEANUP_MIN_INTERVAL_MINUTES");
    }

    #[test]
    fn sleep_cleanup_pressure_mode_prunes_old_state_first() {
        let root = tempdir().expect("tempdir");
        let state_history = root
            .path()
            .join("core/local/state/ops/pressure_lane/history.jsonl");
        fs::create_dir_all(state_history.parent().expect("state parent")).expect("state dir");
        let payload = "x".repeat(16_000);
        fs::write(&state_history, payload).expect("state write");

        std::env::set_var("SPINE_SLEEP_CLEANUP_MIN_INTERVAL_MINUTES", "0");
        std::env::set_var("SPINE_SLEEP_CLEANUP_FREE_SPACE_FLOOR_PERCENT", "100");
        std::env::set_var("SPINE_SLEEP_CLEANUP_PRESSURE_TARGET_FREE_PERCENT", "100");
        std::env::set_var("SPINE_SLEEP_CLEANUP_PRESSURE_JSONL_CAP_BYTES", "1024");
        std::env::set_var("SPINE_SLEEP_CLEANUP_PRESSURE_MIN_AGE_HOURS", "0");

        let (code, out) = execute_sleep_cleanup(root.path(), true, true, "pressure_test");
        assert_eq!(code, 0);
        assert_eq!(
            out.pointer("/pressure_mode/active")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            out.pointer("/removed/pressure_paths")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1
        );

        let capped_size = fs::metadata(&state_history).expect("metadata").len();
        assert!(capped_size <= 1024);

        std::env::remove_var("SPINE_SLEEP_CLEANUP_MIN_INTERVAL_MINUTES");
        std::env::remove_var("SPINE_SLEEP_CLEANUP_FREE_SPACE_FLOOR_PERCENT");
        std::env::remove_var("SPINE_SLEEP_CLEANUP_PRESSURE_TARGET_FREE_PERCENT");
        std::env::remove_var("SPINE_SLEEP_CLEANUP_PRESSURE_JSONL_CAP_BYTES");
        std::env::remove_var("SPINE_SLEEP_CLEANUP_PRESSURE_MIN_AGE_HOURS");
    }

    #[test]
    fn sleep_cleanup_purge_removes_fresh_target_and_trims_history() {
        let root = tempdir().expect("tempdir");
        let target_file = root.path().join("target/debug/fresh.bin");
        let history_path = root
            .path()
            .join("core/local/state/ops/purge_lane/history.jsonl");
        fs::create_dir_all(target_file.parent().expect("target parent")).expect("target dir");
        fs::create_dir_all(history_path.parent().expect("history parent")).expect("history dir");
        fs::write(&target_file, "fresh_target").expect("target write");
        fs::write(&history_path, "x".repeat(24_000)).expect("history write");

        let (code, out) = execute_sleep_cleanup_purge(root.path(), true, true, "purge_test");
        assert_eq!(code, 0);
        assert_eq!(out.get("mode").and_then(Value::as_str), Some("purge"));
        assert_eq!(
            out.pointer("/pressure_mode/active")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(!root.path().join("target").exists());
        assert!(fs::metadata(&history_path).expect("history metadata").len() <= 128 * 1024);
    }

    #[test]
    fn daily_closeout_triggers_sleep_cleanup_automatically() {
        let root = tempdir().expect("tempdir");
        let archive_dir = root.path().join("local/workspace/archive/churn-b");
        let target_file = root.path().join("target/debug/stale.bin");
        fs::create_dir_all(&archive_dir).expect("archive");
        fs::create_dir_all(target_file.parent().expect("target parent")).expect("target dir");
        fs::write(archive_dir.join("receipt.json"), "{}").expect("archive write");
        fs::write(&target_file, "stale").expect("target write");

        std::env::set_var("SPINE_SLEEP_CLEANUP_ARCHIVE_KEEP_LATEST", "0");
        std::env::set_var("SPINE_SLEEP_CLEANUP_ARCHIVE_MAX_AGE_HOURS", "0");
        std::env::set_var("SPINE_SLEEP_CLEANUP_TARGET_MAX_AGE_HOURS", "0");
        std::env::set_var("SPINE_SLEEP_CLEANUP_MIN_INTERVAL_MINUTES", "0");

        let cli = CliArgs {
            command: "run".to_string(),
            mode: "daily".to_string(),
            date: "2026-03-19".to_string(),
            max_eyes: None,
        };
        let run_id = "spine_daily_auto_cleanup";
        let policy = MechSuitPolicy {
            enabled: true,
            heartbeat_hours: 4,
            manual_triggers_allowed: true,
            quiet_non_critical: false,
            silent_subprocess_output: true,
            push_attention_queue: true,
            attention_queue_path: "local/state/attention/queue.jsonl".to_string(),
            attention_receipts_path: "local/state/attention/receipts.jsonl".to_string(),
            attention_latest_path: "local/state/attention/latest.json".to_string(),
            attention_max_queue_depth: 2048,
            attention_ttl_hours: 48,
            attention_dedupe_window_hours: 24,
            attention_backpressure_drop_below: "critical".to_string(),
            attention_escalate_levels: vec!["critical".to_string()],
            ambient_stance: true,
            dopamine_threshold_breach_only: true,
            status_path: root
                .path()
                .join("local/state/ops/mech_suit_mode/latest.json"),
            history_path: root
                .path()
                .join("local/state/ops/mech_suit_mode/history.jsonl"),
            policy_path: root
                .path()
                .join("client/runtime/config/mech_suit_mode_policy.json"),
        };
        let constitution_hash = Some("abc123".to_string());
        let evidence_plan = default_evidence_plan();
        let mut ledger = LedgerWriter::new(root.path(), &cli.date, run_id);
        let ctx = TerminalReceiptContext {
            run_id,
            cli: &cli,
            policy: &policy,
            constitution_hash: &constitution_hash,
            constitution_ok: true,
            evidence_plan: &evidence_plan,
            evidence_ok: 0,
            started_ms: 0,
        };

        let code = emit_terminal_with_closeout(root.path(), &mut ledger, &ctx, true, None);
        assert_eq!(code, 0);

        assert!(!root.path().join("local/workspace/archive/churn-b").exists());
        assert!(!root.path().join("target").exists());
        assert!(root
            .path()
            .join("client/runtime/local/state/ops/sleep_cleanup/latest.json")
            .exists());

        std::env::remove_var("SPINE_SLEEP_CLEANUP_ARCHIVE_KEEP_LATEST");
        std::env::remove_var("SPINE_SLEEP_CLEANUP_ARCHIVE_MAX_AGE_HOURS");
        std::env::remove_var("SPINE_SLEEP_CLEANUP_TARGET_MAX_AGE_HOURS");
        std::env::remove_var("SPINE_SLEEP_CLEANUP_MIN_INTERVAL_MINUTES");
    }
}


