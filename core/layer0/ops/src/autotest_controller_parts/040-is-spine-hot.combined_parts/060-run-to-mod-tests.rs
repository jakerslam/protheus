
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

    let mut paths = runtime_paths(root);
    if let Some(p) = cli.flags.get("policy") {
        let pb = PathBuf::from(p);
        paths.policy_path = if pb.is_absolute() { pb } else { root.join(pb) };
    }
    let policy = load_policy(root, &paths.policy_path);

    if let Err(err) = ensure_state_dirs(&paths) {
        print_json_line(&cli_failure_receipt(&cmd, &err, 1));
        return 1;
    }

    if !policy.enabled && cmd != "status" {
        print_json_line(&json!({
            "ok": true,
            "type": "autotest",
            "ts": now_iso(),
            "disabled": true,
            "reason": "policy_disabled"
        }));
        return 0;
    }

    let out = match cmd.as_str() {
        "sync" => sync_state(root, &paths, &policy),
        "run" => cmd_run(root, &cli, &policy, &paths),
        "report" => cmd_report(root, &cli, &policy, &paths),
        "status" => cmd_status(root, &policy, &paths),
        "pulse" => {
            let run_cli = CliArgs {
                positional: vec!["run".to_string()],
                flags: {
                    let mut flags = cli.flags.clone();
                    flags.insert("sleep-only".to_string(), "1".to_string());
                    flags
                },
            };
            let run_out = cmd_run(root, &run_cli, &policy, &paths);
            let report_cli = CliArgs {
                positional: vec!["report".to_string(), "latest".to_string()],
                flags: HashMap::from([("write".to_string(), "1".to_string())]),
            };
            let report_out = cmd_report(root, &report_cli, &policy, &paths);
            let mut payload = json!({
                "ok": run_out.get("ok").and_then(Value::as_bool).unwrap_or(false)
                    && report_out.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "type": "autotest_pulse",
                "ts": now_iso(),
                "run": run_out,
                "report": report_out,
                "claim_evidence": [
                    {
                        "id": "pulse_pair",
                        "claim": "pulse_runs_and_reports_in_sequence",
                        "evidence": {
                            "run_ok": run_out.get("ok").and_then(Value::as_bool).unwrap_or(false),
                            "report_ok": report_out.get("ok").and_then(Value::as_bool).unwrap_or(false)
                        }
                    }
                ],
                "persona_lenses": {
                    "operator": {
                        "mode": "pulse"
                    }
                }
            });
            payload["receipt_hash"] = Value::String(receipt_hash(&payload));
            payload
        }
        "daemon" => cmd_daemon(root, &cli, &policy, &paths),
        _ => {
            usage();
            print_json_line(&cli_failure_receipt(&cmd, "unknown_command", 2));
            return 2;
        }
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
    fn parity_fixture_score_prefers_layer_and_tokens() {
        let policy = default_policy();
        let module = ModuleCandidate {
            id: "mod_1".to_string(),
            path: "client/runtime/systems/ops/autotest_controller.ts".to_string(),
            abs_path: PathBuf::from("client/runtime/systems/ops/autotest_controller.ts"),
            basename: "autotest_controller".to_string(),
        };
        let test = TestCandidate {
            id: "tst_1".to_string(),
            kind: "node_test".to_string(),
            path: "tests/client-memory-tools/autotest_controller.test.ts".to_string(),
            command: "node tests/client-memory-tools/autotest_controller.test.ts".to_string(),
            stem: "autotest_controller.test".to_string(),
        };
        let score = score_module_test_pair(&module, &test, &policy);
        assert!(score >= policy.min_match_score);
    }

    #[test]
    fn deterministic_receipt_hash_for_fixture() {
        let payload = json!({
            "ok": true,
            "type": "autotest_status",
            "modules_total": 2,
            "tests_total": 4,
            "claim_evidence": [{"id":"c1","claim":"x","evidence":{"a":1}}]
        });
        let h1 = receipt_hash(&payload);
        let h2 = receipt_hash(&payload);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn cli_failure_receipt_includes_hash_and_invariants() {
        let out = cli_failure_receipt("daemonx", "unknown_command", 2);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("autotest_cli_error")
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
    fn parse_cli_preserves_run_command() {
        let cli = parse_cli(&["run".to_string()]);
        assert_eq!(cli.positional.first().map(String::as_str), Some("run"));
    }

    #[test]
    fn parse_cli_honors_double_dash_passthrough() {
        let cli = parse_cli(&[
            "run".to_string(),
            "--scope=web".to_string(),
            "--".to_string(),
            "--grep".to_string(),
            "web fetch".to_string(),
        ]);
        assert_eq!(cli.flags.get("scope").map(String::as_str), Some("web"));
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

