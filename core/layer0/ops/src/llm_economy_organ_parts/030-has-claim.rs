
#[cfg(test)]
mod tests {
    use super::*;

    fn has_claim(payload: &Value, claim_id: &str) -> bool {
        payload
            .get("claim_evidence")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some(claim_id))
    }

    #[test]
    fn normalize_target_maps_known_aliases() {
        assert_eq!(normalize_target("virtuals"), "virtuals_acp");
        assert_eq!(normalize_target("trade-router"), "trade_router_solana");
        assert_eq!(normalize_target(""), "all");
    }

    #[test]
    fn enable_all_writes_enabled_hands_to_latest_state() {
        let dir = tempfile::tempdir().expect("tempdir");
        let exit = run(
            dir.path(),
            &[
                "enable".to_string(),
                "all".to_string(),
                "--apply=1".to_string(),
            ],
        );
        assert_eq!(exit, 0);
        let latest = read_json(&latest_path(dir.path())).expect("latest");
        assert_eq!(
            latest.get("type").and_then(Value::as_str),
            Some("llm_economy_organ_enable")
        );
        assert_eq!(
            latest.get("enabled_count").and_then(Value::as_u64),
            Some(ECONOMY_HANDS.len() as u64)
        );
        assert!(has_claim(&latest, "V6-ECONOMY-001.8"));
    }

    #[test]
    fn trading_hand_upgrade_emits_phase_contract_receipt() {
        let dir = tempfile::tempdir().expect("tempdir");
        let exit = run(
            dir.path(),
            &[
                "upgrade-trading-hand".to_string(),
                "--mode=paper".to_string(),
                "--symbol=QQQ".to_string(),
                "--apply=1".to_string(),
            ],
        );
        assert_eq!(exit, 0);
        let latest = read_json(&latest_path(dir.path())).expect("latest");
        assert_eq!(
            latest.get("type").and_then(Value::as_str),
            Some("llm_economy_organ_trading_hand_upgrade")
        );
        assert_eq!(latest.get("mode").and_then(Value::as_str), Some("paper"));
        assert!(has_claim(&latest, "V6-ECONOMY-002.1"));
        assert!(has_claim(&latest, "V6-ECONOMY-002.4"));
    }

    #[test]
    fn bullbear_debate_and_alpaca_execute_emit_receipts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let debate_exit = run(
            dir.path(),
            &[
                "debate-bullbear".to_string(),
                "--symbol=BTCUSD".to_string(),
                "--bull-score=0.62".to_string(),
                "--bear-score=0.38".to_string(),
            ],
        );
        assert_eq!(debate_exit, 0);
        let latest = read_json(&latest_path(dir.path())).expect("latest");
        assert_eq!(
            latest.get("type").and_then(Value::as_str),
            Some("llm_economy_organ_bullbear_debate")
        );
        assert!(has_claim(&latest, "V6-ECONOMY-002.2"));

        let exec_exit = run(
            dir.path(),
            &[
                "alpaca-execute".to_string(),
                "--mode=analysis".to_string(),
                "--symbol=BTCUSD".to_string(),
                "--side=buy".to_string(),
                "--qty=2".to_string(),
                "--max-qty=5".to_string(),
            ],
        );
        assert_eq!(exec_exit, 0);
        let latest = read_json(&latest_path(dir.path())).expect("latest");
        assert_eq!(
            latest.get("type").and_then(Value::as_str),
            Some("llm_economy_organ_alpaca_execute")
        );
        assert_eq!(
            latest
                .get("risk_gate")
                .and_then(|v| v.get("passed"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(has_claim(&latest, "V6-ECONOMY-002.3"));
    }

    #[test]
    fn economy_connector_commands_emit_deterministic_receipts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cmds: Vec<(Vec<String>, &str)> = vec![
            (vec!["virtuals-acp", "--action=earn"], "V6-ECONOMY-001.1"),
            (
                vec!["bankrbot-defi", "--strategy=stable"],
                "V6-ECONOMY-001.2",
            ),
            (
                vec!["jobs-marketplace", "--source=nookplot"],
                "V6-ECONOMY-001.3",
            ),
            (
                vec!["skills-marketplace", "--source=heurist"],
                "V6-ECONOMY-001.4",
            ),
            (vec!["fairscale-credit", "--delta=2.5"], "V6-ECONOMY-001.5"),
            (
                vec!["mining-hand", "--network=litcoin", "--hours=4"],
                "V6-ECONOMY-001.6",
            ),
            (
                vec!["trade-router", "--chain=solana", "--symbol=SOL/USDC"],
                "V6-ECONOMY-001.7",
            ),
            (
                vec!["model-support-refresh", "--apply=1"],
                "V6-ECONOMY-002.5",
            ),
        ]
        .into_iter()
        .map(|(row, claim_id)| {
            (
                row.into_iter().map(|v| v.to_string()).collect::<Vec<_>>(),
                claim_id,
            )
        })
        .collect();
        for (cmd, claim_id) in cmds {
            let exit = run(dir.path(), &cmd);
            assert_eq!(exit, 0, "failed command {:?}", cmd);
            let latest = read_json(&latest_path(dir.path())).expect("latest");
            assert!(latest.get("receipt_hash").is_some());
            assert!(has_claim(&latest, claim_id));
        }
    }

    #[test]
    fn fairscale_credit_updates_identity_bound_trust_scores() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run_cmd = |args: Vec<String>| -> i32 { run(dir.path(), args.as_slice()) };

        assert_eq!(
            run_cmd(vec![
                "fairscale-credit".to_string(),
                "--identity=alice".to_string(),
                "--delta=2".to_string(),
            ]),
            0
        );
        assert_eq!(
            run_cmd(vec![
                "fairscale-credit".to_string(),
                "--identity=bob".to_string(),
                "--delta=1".to_string(),
            ]),
            0
        );
        assert_eq!(
            run_cmd(vec![
                "fairscale-credit".to_string(),
                "--identity=alice".to_string(),
                "--delta=3".to_string(),
            ]),
            0
        );

        let ledger = read_json(&trust_ledger_path(dir.path())).expect("trust ledger");
        assert_eq!(
            ledger
                .pointer("/scores/alice")
                .and_then(Value::as_f64)
                .unwrap_or(-1.0),
            5.0
        );
        assert_eq!(
            ledger
                .pointer("/scores/bob")
                .and_then(Value::as_f64)
                .unwrap_or(-1.0),
            1.0
        );
    }

    #[test]
    fn trade_router_and_mining_reject_invalid_inputs_in_strict_mode() {
        let dir = tempfile::tempdir().expect("tempdir");

        let bad_trade = run(
            dir.path(),
            &[
                "trade-router".to_string(),
                "--strict=1".to_string(),
                "--chain=ethereum".to_string(),
                "--symbol=SOL/USDC".to_string(),
                "--side=buy".to_string(),
                "--qty=1".to_string(),
            ],
        );
        assert_eq!(bad_trade, 2);
        let latest_trade = read_json(&latest_path(dir.path())).expect("latest");
        assert_eq!(
            latest_trade.get("type").and_then(Value::as_str),
            Some("llm_economy_trade_router_error")
        );

        let bad_mining = run(
            dir.path(),
            &[
                "mining-hand".to_string(),
                "--strict=1".to_string(),
                "--network=unknown".to_string(),
            ],
        );
        assert_eq!(bad_mining, 2);
        let latest_mining = read_json(&latest_path(dir.path())).expect("latest");
        assert_eq!(
            latest_mining.get("type").and_then(Value::as_str),
            Some("llm_economy_mining_hand_error")
        );
    }
}
