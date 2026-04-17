
fn status_cmd(root: &Path, date_arg: &str, cli: &CliArgs) -> Value {
    let policy_path = cli
        .flags
        .get("policy")
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join(DEFAULT_POLICY_REL));
    let paths = runtime_paths(root, &policy_path);

    let key = date_arg.trim().to_ascii_lowercase();
    let payload = if key == "latest" {
        read_json(&paths.latest_path)
    } else {
        read_json(
            &paths
                .runs_dir
                .join(format!("{}.json", clean_text(&key, 16))),
        )
    };

    let mut state = load_doctor_state(&paths);
    prune_history(&mut state, 24, 200_000);
    let count_24h = |kind: &str| count_history(&state, kind, None);
    let latest_path_rel = rel_path(root, &paths.latest_path);
    let state_path_rel = rel_path(root, &paths.state_path);
    let recent_repair_attempts_24h = count_24h("repair_attempt");
    let recent_rollbacks_24h = count_24h("repair_rollback");
    let recent_unknown_signatures_24h = count_24h("unknown_signature");
    let recent_suspicious_signatures_24h = count_24h("suspicious_signature");
    let kill_switch = serde_json::to_value(&state.kill_switch).unwrap_or(Value::Null);
    let kill_switch_engaged = state.kill_switch.engaged;

    if !payload.is_object() {
        let mut out = json!({
            "ok": false,
            "type": "autotest_doctor_status",
            "error": "autotest_doctor_snapshot_missing",
            "kill_switch": kill_switch,
            "state_path": state_path_rel,
            "claim_evidence": [
                {
                    "id": "status_snapshot_missing",
                    "claim": "doctor_status_fails_closed_when_snapshot_missing",
                    "evidence": {
                        "date_arg": key,
                        "latest_path": latest_path_rel
                    }
                }
            ],
            "persona_lenses": {
                "auditor": {
                    "kill_switch": kill_switch_engaged
                }
            }
        });
        out["receipt_hash"] = Value::String(receipt_hash(&out));
        return out;
    }

    let mut out = json!({
        "ok": true,
        "type": "autotest_doctor_status",
        "ts": payload.get("ts").cloned().unwrap_or(Value::Null),
        "run_id": payload.get("run_id").cloned().unwrap_or(Value::Null),
        "date": payload.get("date").cloned().unwrap_or(Value::Null),
        "apply": payload.get("apply").and_then(Value::as_bool).unwrap_or(false),
        "skipped": payload.get("skipped").and_then(Value::as_bool).unwrap_or(false),
        "failures_observed": payload.get("failures_observed").and_then(Value::as_u64).unwrap_or(0),
        "actions_planned": payload.get("actions_planned").and_then(Value::as_u64).unwrap_or(0),
        "actions_applied": payload.get("actions_applied").and_then(Value::as_u64).unwrap_or(0),
        "unknown_signature_count": payload.get("unknown_signature_count").and_then(Value::as_u64).unwrap_or(0),
        "unknown_signature_routes": payload.get("unknown_signature_routes").and_then(Value::as_u64).unwrap_or(0),
        "known_signature_candidates": payload.get("known_signature_candidates").and_then(Value::as_u64).unwrap_or(0),
        "known_signature_auto_handled": payload.get("known_signature_auto_handled").and_then(Value::as_u64).unwrap_or(0),
        "known_signature_auto_handle_rate": payload.get("known_signature_auto_handle_rate").and_then(Value::as_f64).unwrap_or(0.0),
        "rollbacks": payload.get("rollbacks").and_then(Value::as_u64).unwrap_or(0),
        "destructive_repair_blocks": payload.get("destructive_repair_blocks").and_then(Value::as_u64).unwrap_or(0),
        "broken_pieces_stored": payload.get("broken_pieces_stored").and_then(Value::as_u64).unwrap_or(0),
        "research_items_stored": payload.get("research_items_stored").and_then(Value::as_u64).unwrap_or(0),
        "kill_switch": kill_switch,
        "recent_repair_attempts_24h": recent_repair_attempts_24h,
        "recent_rollbacks_24h": recent_rollbacks_24h,
        "recent_unknown_signatures_24h": recent_unknown_signatures_24h,
        "recent_suspicious_signatures_24h": recent_suspicious_signatures_24h,
        "run_path": payload.get("run_path").cloned().unwrap_or(Value::Null),
        "latest_path": latest_path_rel,
        "state_path": state_path_rel,
        "claim_evidence": [
            {
                "id": "status_snapshot",
                "claim": "doctor_status_reflects_latest_state",
                "evidence": {
                    "recent_repair_attempts_24h": recent_repair_attempts_24h,
                    "kill_switch_engaged": kill_switch_engaged
                }
            }
        ],
        "persona_lenses": {
            "auditor": {
                "kill_switch": kill_switch_engaged
            }
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn cli_failure_receipt(cmd: &str, error: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "autotest_doctor_cli_error",
        "ts": now_iso(),
        "command": cmd,
        "error": error,
        "exit_code": code,
        "claim_evidence": [
            {
                "id": "fail_closed_cli",
                "claim": "doctor_cli_failures_emit_deterministic_receipts",
                "evidence": {
                    "command": cmd,
                    "error": error
                }
            }
        ],
        "persona_lenses": {
            "operator": {
                "mode": "cli",
                "exit_code": code
            },
            "auditor": {
                "deterministic_receipt": true
            }
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops autotest-doctor run [YYYY-MM-DD|latest] [--policy=path] [--apply=1|0] [--max-actions=N] [--force=1|0] [--reset-kill-switch=1]");
    println!("  protheus-ops autotest-doctor status [latest|YYYY-MM-DD] [--policy=path]");
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cli = parse_cli(argv);
    let cmd = cli
        .positional
        .first()
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_default();

    if cmd.is_empty() || matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let out = if cmd == "run" {
        run_doctor(
            root,
            cli.positional
                .get(1)
                .map(String::as_str)
                .unwrap_or("latest"),
            &cli,
        )
    } else if cmd == "status" {
        status_cmd(
            root,
            cli.positional
                .get(1)
                .map(String::as_str)
                .unwrap_or("latest"),
            &cli,
        )
    } else {
        usage();
        print_json_line(&cli_failure_receipt(&cmd, "unknown_command", 2));
        return 2;
    };

    let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
    print_json_line(&out);
    if ok {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parity_fixture_classify_failure_kind() {
        let a = json!({"guard_ok": false});
        assert_eq!(classify_failure_kind(&a), "guard_blocked");

        let b = json!({"flaky": true});
        assert_eq!(classify_failure_kind(&b), "flaky");

        let c = json!({"stderr_excerpt": "timed out"});
        assert_eq!(classify_failure_kind(&c), "timeout");

        let d = json!({"exit_code": 2});
        assert_eq!(classify_failure_kind(&d), "exit_nonzero");
    }

    #[test]
    fn parity_fixture_extract_trusted_test_path() {
        let ok = extract_trusted_test_path("node tests/client-memory-tools/a.test.ts");
        assert!(ok.trusted);
        assert_eq!(
            ok.path.as_deref(),
            Some("tests/client-memory-tools/a.test.ts")
        );

        let bad = extract_trusted_test_path("node client/runtime/systems/ops/a.test.ts");
        assert!(!bad.trusted);
        assert_eq!(bad.reason.as_deref(), Some("path_outside_allowlist"));
    }

    #[test]
    fn parity_fixture_collect_failures_signature_stable() {
        let fixture = json!({
            "results": [
                {
                    "id": "tst_a",
                    "ok": false,
                    "command": "node tests/client-memory-tools/a.test.ts",
                    "exit_code": 1,
                    "guard_ok": true,
                    "guard_reason": null,
                    "stderr_excerpt": "assertion failed",
                    "stdout_excerpt": ""
                }
            ]
        });
        let failures = collect_failures(&fixture);
        assert_eq!(failures.len(), 1);
        let first = &failures[0];
        assert!(first.signature_id.starts_with("sig_"));
        assert_eq!(first.kind, "exit_nonzero");
        assert!(first.trusted_test_command);
    }

    #[test]
    fn deterministic_receipt_hash_for_fixture() {
        let payload = json!({
            "ok": true,
            "type": "autotest_doctor_status",
            "actions_applied": 2,
            "claim_evidence": [{"id":"c1","claim":"x","evidence":{"a":1}}]
        });
        let h1 = receipt_hash(&payload);
        let h2 = receipt_hash(&payload);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn cli_failure_receipt_includes_hash_and_invariants() {
        let out = cli_failure_receipt("runx", "unknown_command", 2);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("autotest_doctor_cli_error")
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
    }

    #[test]
    fn status_missing_snapshot_receipt_is_hashed() {
        let root = tempfile::tempdir().expect("tempdir");
        let cli = CliArgs {
            positional: vec!["status".to_string()],
            flags: HashMap::new(),
        };
        let out = status_cmd(root.path(), "latest", &cli);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("autotest_doctor_snapshot_missing")
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
    }

    #[test]
    fn parse_cli_honors_double_dash_passthrough() {
        let cli = parse_cli(&[
            "run".to_string(),
            "--strict=1".to_string(),
            "--".to_string(),
            "--grep".to_string(),
            "web fetch".to_string(),
        ]);
        assert_eq!(cli.flags.get("strict").map(String::as_str), Some("1"));
        assert_eq!(
            cli.positional,
            vec![
                "run".to_string(),
                "--grep".to_string(),
                "web fetch".to_string(),
            ]
        );
    }
}
