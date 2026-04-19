pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    match cmd.as_str() {
        "status" | "runtime-stability-soak" | "self-documentation-closeout" => {
            print_json_line(&native_receipt(root, &cmd, argv));
            0
        }
        "run" => {
            let strict = parse_bool(parse_flag(argv, "strict").as_deref(), false);
            let mut receipt = native_receipt(root, &cmd, argv);
            let run_id = receipt
                .get("receipt_hash")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| {
                    clean_id(
                        Some(format!(
                            "run-{}",
                            now_iso()
                                .chars()
                                .filter(|c| c.is_ascii_digit())
                                .take(16)
                                .collect::<String>()
                        )),
                        "run-duality",
                    )
                });
            let duality = autonomy_duality_bundle(
                root,
                "weaver_arbitration",
                "autonomy_controller_run",
                &run_id,
                &json!({
                    "objective": parse_flag(argv, "objective").unwrap_or_else(|| "default".to_string()),
                    "max_actions": parse_flag(argv, "max-actions")
                        .and_then(|v| v.parse::<i64>().ok())
                        .unwrap_or(1)
                }),
                true,
            );
            if strict && autonomy_duality_hard_block(&duality) {
                let mut denied = cli_error_receipt(argv, "duality_toll_hard_block", 2);
                denied["type"] = Value::String("autonomy_controller_duality_gate".to_string());
                denied["duality"] = duality;
                print_json_line(&denied);
                return 2;
            }
            receipt["duality"] = duality;
            match persist_autonomy_run_row(root, argv, &receipt) {
                Ok(row) => {
                    receipt["run_telemetry"] = json!({
                        "ok": true,
                        "path": lane_utils::rel_path(root, &autonomy_runs_path(root, &today_ymd(
                            receipt.get("ts").and_then(Value::as_str).unwrap_or_default()
                        ))),
                        "row": row
                    });
                    print_json_line(&receipt);
                    0
                }
                Err(err) => {
                    if strict {
                        print_json_line(&cli_error_receipt(
                            argv,
                            &format!("autonomy_run_persist_failed:{err}"),
                            2,
                        ));
                        2
                    } else {
                        receipt["run_telemetry"] = json!({
                            "ok": false,
                            "error": err
                        });
                        print_json_line(&receipt);
                        0
                    }
                }
            }
        }
        "hand-new" => run_hand_new(root, argv),
        "hand-cycle" => run_hand_cycle(root, argv),
        "hand-status" => run_hand_status(root, argv),
        "hand-memory-page" => run_hand_memory_page(root, argv),
        "hand-wasm-task" => run_hand_wasm_task(root, argv),
        "compact" => run_tiered_compaction(root, argv),
        "dream" => run_dream_consolidation(root, argv),
        "proactive_daemon" | "kairos" => run_proactive_daemon_daemon(root, argv),
        "speculate" | "speculation" => run_speculation_overlay(root, argv),
        "autoreason" => run_autoreason(root, argv),
        "ephemeral-run" => run_ephemeral(root, argv),
        "trunk-status" => run_trunk_status(root, argv),
        "pain-signal" => {
            print_json_line(&native_pain_signal_receipt(root, argv));
            0
        }
        "multi-agent-debate" => run_multi_agent_debate(root, argv),
        "ethical-reasoning" => run_ethical_reasoning(root, argv),
        "autonomy-simulation-harness" => run_simulation_harness(root, argv),
        "non-yield-cycle" => {
            run_extended_autonomy_lane(root, argv, "non-yield-cycle", "autonomy_non_yield_cycle")
        }
        "non-yield-harvest" => run_extended_autonomy_lane(
            root,
            argv,
            "non-yield-harvest",
            "autonomy_non_yield_harvest",
        ),
        "non-yield-enqueue" => run_extended_autonomy_lane(
            root,
            argv,
            "non-yield-enqueue",
            "autonomy_non_yield_enqueue",
        ),
        "non-yield-replay" => {
            run_extended_autonomy_lane(root, argv, "non-yield-replay", "autonomy_non_yield_replay")
        }
        "non-yield-ledger-backfill" => run_extended_autonomy_lane(
            root,
            argv,
            "non-yield-ledger-backfill",
            "autonomy_non_yield_ledger_backfill",
        ),
        "autophagy-baseline-guard" => run_extended_autonomy_lane(
            root,
            argv,
            "autophagy-baseline-guard",
            "autophagy_baseline_guard",
        ),
        "doctor-forge-micro-debug-lane" => run_extended_autonomy_lane(
            root,
            argv,
            "doctor-forge-micro-debug-lane",
            "doctor_forge_micro_debug_lane",
        ),
        "physiology-opportunity-map" => run_extended_autonomy_lane(
            root,
            argv,
            "physiology-opportunity-map",
            "autonomy_physiology_opportunity_map",
        ),
        _ => {
            usage();
            print_json_line(&cli_error_receipt(argv, "unknown_command", 2));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn native_receipt_is_deterministic() {
        let root = tempdir().expect("tempdir");
        let args = vec!["run".to_string(), "--objective=t1".to_string()];
        let payload = native_receipt(root.path(), "run", &args);
        let hash = payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .expect("hash")
            .to_string();
        let mut unhashed = payload.clone();
        unhashed
            .as_object_mut()
            .expect("obj")
            .remove("receipt_hash");
        assert_eq!(receipt_hash(&unhashed), hash);
    }

    #[test]
    fn run_persists_autonomy_run_row_for_harness() {
        let root = tempdir().expect("tmp");
        let code = run(
            root.path(),
            &[
                "run".to_string(),
                "--objective=t1_harness_seed".to_string(),
                "--max-actions=2".to_string(),
            ],
        );
        assert_eq!(code, 0);

        let today: String = now_iso().chars().take(10).collect();
        let path = root
            .path()
            .join("client")
            .join("runtime")
            .join("local")
            .join("state")
            .join("autonomy")
            .join("runs")
            .join(format!("{today}.jsonl"));
        let rows = read_jsonl(&path);
        assert!(!rows.is_empty());
        let last = rows.last().expect("row");
        assert_eq!(
            last.get("type").and_then(Value::as_str),
            Some("autonomy_run")
        );
        assert_eq!(
            last.get("objective_id").and_then(Value::as_str),
            Some("t1_harness_seed")
        );
        assert!(last.get("duality").is_some());
        assert!(last.pointer("/duality/toll").is_some());
        assert!(last.pointer("/duality/dual_voice").is_some());
    }

    #[test]
    fn multi_agent_debate_command_emits_payload() {
        let root = tempdir().expect("tmp");
        let args = vec![
            "multi-agent-debate".to_string(),
            "run".to_string(),
            format!(
                "--input-base64={}",
                base64::engine::general_purpose::STANDARD
                    .encode("{\"objective_id\":\"t1\",\"candidates\":[{\"candidate_id\":\"a\",\"score\":0.8,\"confidence\":0.8,\"risk\":\"low\"}]}")
            ),
            "--persist=0".to_string(),
        ];
        let code = run(root.path(), &args);
        assert_eq!(code, 0);
    }

    #[test]
    fn autonomy_hand_and_ephemeral_lanes_emit_claim_receipts() {
        let root = tempdir().expect("tmp");
        assert_eq!(
            run(
                root.path(),
                &[
                    "hand-new".to_string(),
                    "--hand-id=alpha".to_string(),
                    "--template=research".to_string(),
                    "--strict=1".to_string(),
                ],
            ),
            0
        );
        assert_eq!(
            run(
                root.path(),
                &[
                    "hand-cycle".to_string(),
                    "--hand-id=alpha".to_string(),
                    "--goal=collect".to_string(),
                    "--strict=1".to_string(),
                ],
            ),
            0
        );
        assert_eq!(
            run(
                root.path(),
                &[
                    "hand-memory-page".to_string(),
                    "--hand-id=alpha".to_string(),
                    "--op=page-in".to_string(),
                    "--tier=core".to_string(),
                    "--key=k1".to_string(),
                    "--strict=1".to_string(),
                ],
            ),
            0
        );
        assert_eq!(
            run(
                root.path(),
                &[
                    "hand-wasm-task".to_string(),
                    "--hand-id=alpha".to_string(),
                    "--task=t1".to_string(),
                    "--fuel=500".to_string(),
                    "--epoch-ms=100".to_string(),
                    "--strict=1".to_string(),
                ],
            ),
            0
        );
        assert_eq!(
            run(
                root.path(),
                &[
                    "ephemeral-run".to_string(),
                    "--goal=build feature".to_string(),
                    "--domain=general".to_string(),
                    "--ui-leaf=1".to_string(),
                    "--strict=1".to_string(),
                ],
            ),
            0
        );
        assert_eq!(
            run(
                root.path(),
                &["trunk-status".to_string(), "--strict=1".to_string()],
            ),
            0
        );
    }

    #[test]
    fn conduit_bypass_is_rejected_for_ephemeral_and_hands() {
        let root = tempdir().expect("tmp");
        assert_eq!(
            run(
                root.path(),
                &[
                    "ephemeral-run".to_string(),
                    "--goal=t".to_string(),
                    "--bypass=1".to_string(),
                    "--strict=1".to_string(),
                ],
            ),
            1
        );
        assert_eq!(
            run(
                root.path(),
                &[
                    "hand-new".to_string(),
                    "--hand-id=beta".to_string(),
                    "--bypass=1".to_string(),
                    "--strict=1".to_string(),
                ],
            ),
            1
        );
    }

    #[test]
    fn run_strict_fails_closed_when_duality_toll_hard_blocks() {
        let root = tempdir().expect("tmp");
        let config_dir = root.path().join("client/runtime/config");
        let state_dir = root.path().join("local/state/autonomy/duality");
        fs::create_dir_all(&config_dir).expect("config dir");
        fs::create_dir_all(&state_dir).expect("state dir");
        fs::write(
            config_dir.join("duality_codex.txt"),
            "order/chaos harmonization\nzero point\n",
        )
        .expect("codex");
        fs::write(
            config_dir.join("duality_seed_policy.json"),
            serde_json::to_string_pretty(&json!({
                "enabled": true,
                "shadow_only": true,
                "advisory_only": true,
                "toll_enabled": true,
                "toll_hard_block_threshold": 0.5,
                "codex_path": "client/runtime/config/duality_codex.txt",
                "state": {
                    "latest_path": "local/state/autonomy/duality/latest.json",
                    "history_path": "local/state/autonomy/duality/history.jsonl"
                },
                "outputs": {"persist_shadow_receipts": true, "persist_observations": true}
            }))
            .expect("policy encode"),
        )
        .expect("policy");
        fs::write(
            state_dir.join("latest.json"),
            serde_json::to_string_pretty(&json!({
                "version": "v1",
                "seed_confidence": 1.0,
                "toll_debt": 5.0
            }))
            .expect("state encode"),
        )
        .expect("state");

        let code = run(
            root.path(),
            &[
                "run".to_string(),
                "--objective=hard_block_probe".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(code, 2);
    }

    #[test]
    fn new_memory_autonomy_and_speculation_commands_dispatch() {
        let root = tempdir().expect("tmp");
        assert_eq!(
            run(
                root.path(),
                &[
                    "compact".to_string(),
                    "--hand-id=alpha".to_string(),
                    "--mode=micro".to_string()
                ],
            ),
            0
        );
        assert_eq!(
            run(
                root.path(),
                &["dream".to_string(), "--hand-id=alpha".to_string()]
            ),
            0
        );
        assert_eq!(
            run(root.path(), &["proactive_daemon".to_string(), "status".to_string()]),
            0
        );
        assert_eq!(
            run(
                root.path(),
                &[
                    "speculate".to_string(),
                    "run".to_string(),
                    "--spec-id=alpha-spec".to_string()
                ],
            ),
            0
        );
        assert_eq!(
            run(
                root.path(),
                &[
                    "autoreason".to_string(),
                    "status".to_string(),
                    "--strict=1".to_string(),
                ],
            ),
            0
        );
    }
}
