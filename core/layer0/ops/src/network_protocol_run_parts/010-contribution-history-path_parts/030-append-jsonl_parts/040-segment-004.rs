        match commit_ledger(
            root,
            ledger,
            "tokenomics_update",
            json!({
                "action": action,
                "agent": agent,
                "amount": amount,
                "reason": reason,
                "balance_after": next_balance,
                "stake_after": next_stake
            }),
        ) {
            Ok(updated) => emit(
                root,
                json!({
                    "ok": true,
                    "type": "network_protocol_tokenomics_update",
                    "lane": "core/layer0/ops",
                    "action": action,
                    "agent": agent,
                    "amount": amount,
                    "reason": reason,
                    "balances": updated.get("balances").cloned().unwrap_or(Value::Object(Map::new())),
                    "staked": updated.get("staked").cloned().unwrap_or(Value::Object(Map::new())),
                    "network_state_root": updated.get("root_head").cloned().unwrap_or(Value::Null),
                    "layer_map": ["0","1","2","adapter"],
                    "claim_evidence": [
                        {
                            "id": "V8-NETWORK-002.1",
                            "claim": "staking_rewards_and_slashing_emit_identity_bound_receipts",
                            "evidence": {"action": action, "agent": agent}
                        },
                        {
                            "id": "V7-NETWORK-001.1",
                            "claim": "identity_bound_staking_reward_and_slashing_updates_network_tokenomics_ledger",
                            "evidence": {"action": action, "agent": agent, "amount": amount}
                        }
                    ]
                }),
            ),
            Err(err) => emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_tokenomics_update",
                    "lane": "core/layer0/ops",
                    "error": clean(err, 220)
                }),
            ),
        }
    } else if command == "contribution" {
        let strict = parse_bool(parsed.flags.get("strict"), true);
        let agent = clean(
            parsed
                .flags
                .get("agent")
                .cloned()
                .unwrap_or_else(|| "shadow:default".to_string()),
            120,
        );
        let contribution_type = clean(
            parsed
                .flags
                .get("contribution-type")
                .cloned()
                .unwrap_or_else(|| "compute".to_string()),
            24,
        )
        .to_ascii_lowercase();
        let score = parse_f64(parsed.flags.get("score"), 0.6).clamp(0.0, 1.0);
        let mut reward = parse_f64(parsed.flags.get("reward"), -1.0);
        if reward < 0.0 {
            let multiplier = match contribution_type.as_str() {
                "compute" => 1.0,
                "memory" => 1.1,
                "rl" => 1.2,
                "breakthrough" => 1.6,
                _ => 0.9,
            };
            reward = (score * 100.0) * multiplier;
        }
        let stake = parse_f64(parsed.flags.get("stake"), reward * 0.5).max(0.0);
        let slash = parse_f64(parsed.flags.get("slash"), 0.0).max(0.0);
        let gate_ok = gate_action(
            root,
            &format!("tokenomics:contribution:{agent}:{contribution_type}:{score:.4}"),
        );
        if strict && !gate_ok {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_contribution",
                    "lane": "core/layer0/ops",
                    "agent": agent,
                    "contribution_type": contribution_type,
                    "error": "directive_gate_denied",
                    "claim_evidence": [
                        {
                            "id": "V7-NETWORK-001.1",
                            "claim": "proof_of_useful_intelligence_updates_are_policy_gated_and_identity_bound",
                            "evidence": {"allowed": false}
                        }
                    ]
                }),
            );
        }
        let mut ledger = load_ledger(root);
        let balances = ledger
            .get("balances")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let current_balance = balance_of(&balances, &agent);
        put_balance(
            &mut ledger,
            &agent,
            (current_balance + reward - slash).max(0.0),
        );
        let staked = ledger
            .get("staked")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let current_stake = balance_of(&staked, &agent);
        put_stake(
            &mut ledger,
            &agent,
            (current_stake + stake - slash).max(0.0),
        );
        let contribution_event = json!({
            "type": "useful_intelligence_contribution",
            "ts": now_iso(),
            "agent": agent,
            "contribution_type": contribution_type,
            "score": score,
            "reward": reward,
            "stake": stake,
            "slash": slash
        });
        let _ = append_jsonl(&contribution_history_path(root), &contribution_event);
        match commit_ledger(
            root,
            ledger,
            "useful_contribution",
            contribution_event.clone(),
        ) {
            Ok(updated) => emit(
                root,
                json!({
                    "ok": true,
                    "type": "network_protocol_contribution",
                    "lane": "core/layer0/ops",
                    "event": contribution_event,
                    "network_state_root": updated.get("root_head").cloned().unwrap_or(Value::Null),
                    "claim_evidence": [
                        {
                            "id": "V7-NETWORK-001.1",
                            "claim": "proof_of_useful_intelligence_contributions_drive_identity_bound_reward_stake_slash_updates",
                            "evidence": {"agent": agent, "contribution_type": contribution_type, "score": score}
                        }
                    ]
                }),
            ),
            Err(err) => emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_contribution",
                    "lane": "core/layer0/ops",
                    "error": clean(err, 220)
                }),
            ),
        }
    } else if command == "consensus" {
        let op = clean(
            parsed
                .flags
                .get("op")
