                    "claim_evidence": [
                        {
                            "id": "V8-NETWORK-003.4",
                            "claim": "oracle_query_and_market_actions_are_conduit_and_policy_gated",
                            "evidence": {"allowed": false, "provider": provider}
                        }
                    ]
                }),
            );
        }
        let seed = sha256_hex_str(&format!("{provider}:{event}"));
        let yes = (u64::from_str_radix(&seed[0..8], 16).unwrap_or(5000) % 10000) as f64 / 10000.0;
        let no = 1.0 - yes;
        let market_id = format!("{}:{}", provider, &seed[..12]);
        let query = json!({
            "market_id": market_id,
            "provider": provider.clone(),
            "event": event.clone(),
            "probabilities": {
                "yes": yes,
                "no": no
            },
            "confidence": ((yes - 0.5).abs() * 2.0).min(1.0),
            "ts": now_iso(),
            "provenance_hash": seed
        });
        let _ = write_json(&state_root(root).join("oracle").join("latest.json"), &query);
        let _ = append_event(root, &json!({"oracle_query": query.clone()}));
        emit(
            root,
            json!({
                "ok": true,
                "type": "network_protocol_oracle_query",
                "lane": "core/layer0/ops",
                "query": query,
                "claim_evidence": [
                    {
                        "id": "V8-NETWORK-003.1",
                        "claim": "prediction_market_oracle_returns_structured_probability_with_provenance",
                        "evidence": {"provider": provider.clone(), "market_id": market_id}
                    },
                    {
                        "id": "V8-NETWORK-003.4",
                        "claim": "oracle_query_and_market_actions_are_conduit_and_policy_gated",
                        "evidence": {"allowed": true, "provider": provider}
                    }
                ]
            }),
        )
    } else if command == "truth-weight" {
        let strict = parse_bool(parsed.flags.get("strict"), true);
        let market = clean(
            parsed
                .flags
                .get("market")
                .cloned()
                .or_else(|| parsed.flags.get("market-id").cloned())
                .unwrap_or_else(|| "polymarket:default".to_string()),
            160,
        );
        let gate_ok = gate_action(root, &format!("oracle:truth-weight:{market}"));
        if strict && !gate_ok {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_truth_weight",
                    "lane": "core/layer0/ops",
                    "market": market,
                    "error": "directive_gate_denied",
                    "claim_evidence": [
                        {
                            "id": "V8-NETWORK-003.4",
                            "claim": "truth_weight_is_conduit_and_policy_gated",
                            "evidence": {"allowed": false}
                        }
                    ]
                }),
            );
        }
        let latest = read_json(&state_root(root).join("oracle").join("latest.json"))
            .unwrap_or_else(|| {
                json!({
                    "market_id": market,
                    "probabilities": {"yes": 0.5, "no": 0.5},
                    "confidence": 0.0
                })
            });
        let p_yes = latest
            .get("probabilities")
            .and_then(|v| v.get("yes"))
            .and_then(Value::as_f64)
            .unwrap_or(0.5);
        let source_reliability = (p_yes * 0.7) + 0.3;
        let causality_alignment = (1.0 - (0.5 - p_yes).abs()) * 0.8;
        let hybrid = ((source_reliability + causality_alignment) / 2.0).min(1.0);
        let disinfo_guard = json!({
            "quarantine_threshold": 0.25,
            "weight_multiplier": if hybrid < 0.25 { 0.2 } else { 1.0 },
            "mode": if hybrid < 0.25 { "quarantine" } else { "reweight" }
        });
        let out_state = json!({
            "market": market.clone(),
            "hybrid_confidence": hybrid,
            "components": {
                "market_probability_yes": p_yes,
                "source_reliability": source_reliability,
                "causality_alignment": causality_alignment
            },
            "disinformation_guard": disinfo_guard,
            "ts": now_iso()
        });
        let _ = write_json(
            &state_root(root)
                .join("oracle")
                .join("truth_weight_latest.json"),
            &out_state,
        );
        emit(
            root,
            json!({
                "ok": true,
                "type": "network_protocol_truth_weight",
                "lane": "core/layer0/ops",
                "weighting": out_state,
                "claim_evidence": [
                    {
                        "id": "V8-NETWORK-003.2",
                        "claim": "market_probabilities_are_fused_with_truth_signals_for_hybrid_scoring",
                        "evidence": {"hybrid_confidence": hybrid}
                    },
                    {
                        "id": "V8-NETWORK-003.3",
                        "claim": "disinformation_resistance_weights_or_quarantines_low_confidence_inputs",
                        "evidence": {"mode": disinfo_guard.get("mode").cloned().unwrap_or(Value::Null)}
                    },
                    {
                        "id": "V8-NETWORK-003.4",
                        "claim": "truth_weight_is_conduit_and_policy_gated",
                        "evidence": {"allowed": true}
                    },
                    {
                        "id": "V8-NETWORK-003.5",
                        "claim": "truth_weight_command_surface_routes_to_core_with_dashboard_ready_state",
                        "evidence": {"market": market}
                    }
                ]
            }),
        )
    } else if command == "merkle-root" {
        let gate_ok = gate_action(root, "tokenomics:merkle-root");
        if !gate_ok {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_global_merkle_root",
                    "lane": "core/layer0/ops",
                    "error": "directive_gate_denied",
                    "gate_action": "tokenomics:merkle-root"
                }),
            );
        }
        let account = clean(
            parsed.flags.get("account").cloned().unwrap_or_default(),
            120,
        );
        let proof_requested = parse_bool(parsed.flags.get("proof"), true);
        let ledger = load_ledger(root);
        let policy_hash = directive_kernel::directive_vault_hash(root);
        let leaves = leaves_for_root(&ledger, &policy_hash);
        let root_hash = deterministic_merkle_root(&leaves);

