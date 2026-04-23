        let ledger = load_ledger(root);
        return emit(
            root,
            json!({
                "ok": true,
                "type": "network_protocol_ignite_bitcoin",
                "lane": "core/layer0/ops",
                "apply": apply,
                "profile": "bitcoin",
                "seed": seed,
                "activation": {
                    "command": "infring network ignite bitcoin",
                    "surface": "core://network-protocol"
                },
                "network_state_root": ledger.get("root_head").cloned().unwrap_or(Value::String("genesis".to_string())),
                "gates": {
                    "conduit_required": true,
                    "prime_directive_gate": true,
                    "sovereign_identity_required": true,
                    "fail_closed": true
                },
                "gate_action": "tokenomics:ignite-bitcoin",
                "layer_map": ["0","1","2","client","app"],
                "claim_evidence": [
                    {
                        "id": "V8-NETWORK-002.5",
                        "claim": "bitcoin_profile_ignition_is_core_authoritative_and_receipted",
                        "evidence": {"profile": "bitcoin", "state_root_present": true}
                    }
                ]
            }),
        );
    }

    if command == "stake" || command == "reward" || command == "slash" {
        let action = parsed
            .flags
            .get("action")
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_else(|| command.clone());
        let agent = clean(
            parsed
                .flags
                .get("agent")
                .cloned()
                .unwrap_or_else(|| "shadow:default".to_string()),
            120,
        );
        let amount = parse_f64(parsed.flags.get("amount"), 10.0).max(0.0);
        let reason = clean(
            parsed
                .flags
                .get("reason")
                .cloned()
                .unwrap_or_else(|| "proof_of_useful_intelligence".to_string()),
            220,
        );

        let mut ledger = load_ledger(root);
        let balances = ledger
            .get("balances")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let current_balance = balance_of(&balances, &agent);

        let gate_action = format!("tokenomics:{}:{}:{}", action, agent, reason);
        let gate_ok = directive_kernel::action_allowed(root, &gate_action);
        if !gate_ok && action != "slash" {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_tokenomics_update",
                    "lane": "core/layer0/ops",
                    "action": action,
                    "agent": agent,
                    "amount": amount,
                    "reason": reason,
                    "error": "directive_gate_denied",
                    "layer_map": ["0","1","2","adapter"],
                    "claim_evidence": [
                        {
                            "id": "V8-NETWORK-002.1",
                            "claim": "staking_rewards_and_slashing_emit_identity_bound_receipts",
                            "evidence": {"allowed": false, "reason": "directive_gate_denied"}
                        }
                    ]
                }),
            );
        }

        let next_balance = match action.as_str() {
            "slash" => (current_balance - amount).max(0.0),
            _ => current_balance + amount,
        };
        put_balance(&mut ledger, &agent, next_balance);

        let staked = ledger
            .get("staked")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let current_stake = balance_of(&staked, &agent);
        let next_stake = match action.as_str() {
            "stake" => current_stake + amount,
            "slash" => (current_stake - amount).max(0.0),
            _ => current_stake,
        };
        put_stake(&mut ledger, &agent, next_stake);

