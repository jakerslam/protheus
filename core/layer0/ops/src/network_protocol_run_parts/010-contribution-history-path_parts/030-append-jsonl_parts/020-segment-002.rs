    if command == "dashboard" {
        let ledger = load_ledger(root);
        let policy_hash = directive_kernel::directive_vault_hash(root);
        let leaves = leaves_for_root(&ledger, &policy_hash);
        let global_merkle_root = deterministic_merkle_root(&leaves);
        let balances = ledger
            .get("balances")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let staked = ledger
            .get("staked")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let zk_claims = ledger
            .get("zk_claims")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let verified_claims = zk_claims
            .values()
            .filter(|row| {
                row.get("verified")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .count();
        let total_balance = balances
            .values()
            .filter_map(Value::as_f64)
            .fold(0.0f64, |acc, amount| acc + amount);
        let total_staked = staked
            .values()
            .filter_map(Value::as_f64)
            .fold(0.0f64, |acc, amount| acc + amount);
        let membership = read_json(&membership_path(root)).unwrap_or_else(|| json!({"nodes": []}));
        let governance_votes = read_jsonl(&governance_votes_path(root));
        let consensus_rows = read_jsonl(&consensus_ledger_path(root));
        let emission = ledger.get("emission").cloned().unwrap_or(Value::Null);
        let oracle_latest =
            read_json(&state_root(root).join("oracle").join("latest.json")).unwrap_or(Value::Null);

        return emit(
            root,
            json!({
                "ok": true,
                "type": "network_protocol_dashboard",
                "lane": "core/layer0/ops",
                "activation_command": "protheus network ignite bitcoin",
                "token_flow": {
                    "accounts": balances.len(),
                    "total_balance": total_balance,
                    "total_staked": total_staked
                },
                "ledger_health": {
                    "global_merkle_root": global_merkle_root,
                    "root_head": ledger.get("root_head").cloned().unwrap_or(Value::Null),
                    "leaf_count": leaves.len(),
                    "height": ledger.get("height").cloned().unwrap_or(Value::from(0))
                },
                "emission_curve": emission,
                "zk_claims": {
                    "total": zk_claims.len(),
                    "verified": verified_claims
                },
                "network_organism_view": {
                    "tokenomics": true,
                    "merkle_state": true,
                    "emission": true,
                    "zk_claims": true,
                    "oracle": oracle_latest != Value::Null,
                    "consensus": !consensus_rows.is_empty(),
                    "membership": membership
                        .get("nodes")
                        .and_then(Value::as_array)
                        .map(|rows| !rows.is_empty())
                        .unwrap_or(false)
                },
                "governance": {
                    "consensus_event_count": consensus_rows.len(),
                    "vote_count": governance_votes.len(),
                    "member_count": membership
                        .get("nodes")
                        .and_then(Value::as_array)
                        .map(|rows| rows.len())
                        .unwrap_or(0)
                },
                "web_tooling": {
                    "runtime": read_json(&web_tooling_runtime_path(root)).unwrap_or(Value::Null),
                    "auth": detect_web_tooling_auth_presence()
                },
                "claim_evidence": [
                    {
                        "id": "V8-NETWORK-002.5",
                        "claim": "network_organism_dashboard_surfaces_token_ledger_emission_and_claim_health",
                        "evidence": {
                            "global_merkle_root": global_merkle_root,
                            "verified_claims": verified_claims,
                            "total_accounts": balances.len()
                        }
                    },
                    {
                        "id": "V8-NETWORK-003.5",
                        "claim": "dashboard_surfaces_oracle_and_truth_weight_state",
                        "evidence": {
                            "oracle_available": oracle_latest != Value::Null
                        }
                    },
                    {
                        "id": "V7-NETWORK-001.4",
                        "claim": "network_dashboard_surfaces_membership_and_reputation_weighted_governance_activity",
                        "evidence": {
                            "member_count": membership
                                .get("nodes")
                                .and_then(Value::as_array)
                                .map(|rows| rows.len())
                                .unwrap_or(0),
                            "vote_count": governance_votes.len()
                        }
                    }
                ]
            }),
        );
    }

    if command == "ignite-bitcoin"
        || (command == "ignite"
            && parsed
                .positional
                .get(1)
                .map(|v| v.trim().eq_ignore_ascii_case("bitcoin"))
                .unwrap_or(false))
    {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let seed = clean(
            parsed
                .flags
                .get("seed")
                .cloned()
                .unwrap_or_else(|| "genesis".to_string()),
            96,
        );
        let gate_ok = gate_action(root, "tokenomics:ignite-bitcoin");
        if apply && !gate_ok {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_ignite_bitcoin",
                    "lane": "core/layer0/ops",
                    "apply": apply,
                    "profile": "bitcoin",
                    "seed": seed,
                    "error": "directive_gate_denied",
                    "gate_action": "tokenomics:ignite-bitcoin",
                    "layer_map": ["0","1","2","client","app"],
                    "claim_evidence": [
                        {
                            "id": "V8-NETWORK-002.5",
                            "claim": "bitcoin_profile_ignition_is_core_authoritative_and_receipted",
                            "evidence": {"allowed": false, "reason": "directive_gate_denied"}
                        }
                    ]
                }),
            );
        }

        if apply && !ledger_path(root).exists() {
            let mut ledger = default_ledger();
            put_balance(&mut ledger, "organism:treasury", 1_000_000.0);
            put_stake(&mut ledger, "organism:treasury", 0.0);
            let _ = commit_ledger(root, ledger, "ignite_bitcoin", json!({"seed": seed}));
        }

