
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let strict = parse_bool(parse_cli_flag(argv, "strict"), false);

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    if matches!(command.as_str(), "status" | "dashboard") {
        let conduit = conduit_enforcement(argv, &command, strict);
        if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            let mut out = json!({
                "ok": false,
                "type": "p2p_gossip_seed_conduit_gate",
                "lane": "core/layer2/ops",
                "command": command,
                "strict": strict,
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            print_json_line(&out);
            return 1;
        }
        let out = dashboard_receipt(root);
        persist_receipt(root, &out);
        print_json_line(&out);
        return 0;
    }

    let conduit = conduit_enforcement(argv, &command, strict);
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let mut out = json!({
            "ok": false,
            "type": "p2p_gossip_seed_conduit_gate",
            "lane": "core/layer2/ops",
            "command": command,
            "strict": strict,
            "errors": ["conduit_bypass_rejected"],
            "conduit_enforcement": conduit
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        print_json_line(&out);
        return 1;
    }

    if matches!(command.as_str(), "discover" | "join") {
        let profile = parse_cli_flag(argv, "profile").unwrap_or_else(|| "hyperspace".to_string());
        let node = parse_cli_flag(argv, "node").unwrap_or_else(|| "local-node".to_string());
        let apply = parse_bool(parse_cli_flag(argv, "apply"), true);
        let mut rep = reputations(root);
        rep.entry(node.clone()).or_insert(Value::from(1.0));
        let bootstrap_reputation = rep.get(&node).and_then(Value::as_f64).unwrap_or(0.0);
        write_reputations(root, &rep);
        let mut out = json!({
            "ok": true,
            "type": "p2p_gossip_seed_join",
            "lane": "core/layer2/ops",
            "ts_epoch_ms": now_epoch_ms(),
            "command": command,
            "profile": profile.clone(),
            "node": node.clone(),
            "apply": apply,
            "claim_evidence": [
                {
                    "id": "V6-NETWORK-004.6",
                    "claim": "network_join_hyperspace_routes_to_core_runtime_with_deterministic_receipts",
                    "evidence": {
                        "profile": profile,
                        "node": node.clone()
                    }
                },
                {
                    "id": "V6-NETWORK-004.2",
                    "claim": "reputation_ledger_bootstraps_identity_state_on_join",
                    "evidence": {
                        "node": node,
                        "bootstrap_reputation": bootstrap_reputation
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        persist_receipt(root, &out);
        print_json_line(&out);
        return 0;
    }

    if command == "compute-proof" {
        let share = parse_bool(parse_cli_flag(argv, "share"), true);
        let node = parse_cli_flag(argv, "node").unwrap_or_else(|| "local-node".to_string());
        let matmul_size = parse_u64(parse_cli_flag(argv, "matmul-size"), 512);
        let credits = parse_f64(parse_cli_flag(argv, "credits"), 1.0).max(0.0);
        if strict && (matmul_size < 64 || !matmul_size.is_power_of_two()) {
            let mut out = json!({
                "ok": false,
                "type": "p2p_gossip_seed_compute_proof_error",
                "lane": "core/layer2/ops",
                "ts_epoch_ms": now_epoch_ms(),
                "error": "invalid_matmul_size",
                "matmul_size": matmul_size,
                "required": "power_of_two_and_gte_64"
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            persist_receipt(root, &out);
            print_json_line(&out);
            return 2;
        }
        let mut rep = reputations(root);
        let prior = rep.get(&node).and_then(Value::as_f64).unwrap_or(1.0);
        let next = if share { prior + credits } else { prior };
        rep.insert(node.clone(), Value::from(next));
        write_reputations(root, &rep);
        let challenge = json!({
            "algo": "matmul",
            "matmul_size": matmul_size,
            "node": node,
            "share": share
        });
        let challenge_id = deterministic_receipt_hash(&challenge);
        let mut contribution = contributions(root);
        let existing = contribution.get(&node).cloned().unwrap_or_else(|| {
            json!({
                "proof_count": 0,
                "total_credits": 0.0
            })
        });
        let prior_count = existing
            .get("proof_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let prior_credits = existing
            .get("total_credits")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        contribution.insert(
            node.clone(),
            json!({
                "proof_count": prior_count + 1,
                "total_credits": prior_credits + if share { credits } else { 0.0 },
                "last_challenge_id": challenge_id,
                "last_matmul_size": matmul_size,
                "updated_at_epoch_ms": now_epoch_ms()
            }),
        );
        write_contributions(root, &contribution);
        let mut out = json!({
            "ok": true,
            "type": "p2p_gossip_seed_compute_proof",
            "lane": "core/layer2/ops",
            "ts_epoch_ms": now_epoch_ms(),
            "share": share,
            "node": node.clone(),
            "proof": {
                "challenge": challenge,
                "challenge_id": challenge_id,
                "matmul_size": matmul_size
            },
            "credits": {
                "delta": credits,
                "prior": prior,
                "next": next
            },
            "contribution_ledger_path": contribution_path(root).display().to_string(),
            "claim_evidence": [
                {
                    "id": "V6-NETWORK-004.1",
                    "claim": "compute_proof_emits_matmul_challenge_receipts_and_credit_updates",
                    "evidence": {
                        "matmul_size": matmul_size,
                        "delta": credits,
                        "challenge_id": challenge_id
                    }
                },
                {
                    "id": "V6-NETWORK-004.2",
                    "claim": "reputation_ledger_is_updated_deterministically_for_compute_contributions",
                    "evidence": {
                        "node": node.clone(),
                        "prior": prior,
                        "next": next,
                        "proof_count": prior_count + 1
                    }
                },
                {
                    "id": "V6-NETWORK-004.6",
                    "claim": "compute_share_surface_routes_into_core_network_runtime_with_receipts",
                    "evidence": {
                        "surface": "compute share",
                        "share": share
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        persist_receipt(root, &out);
        print_json_line(&out);
        return 0;
    }

    if command == "gossip" {
        let topic = parse_cli_flag(argv, "topic").unwrap_or_else(|| "ranking".to_string());
        let breakthrough = parse_cli_flag(argv, "breakthrough")
            .unwrap_or_else(|| "listnet_rediscovery_candidate".to_string());
        let rep = reputations(root);
        let peers = rep.keys().cloned().collect::<Vec<_>>();
        let gossip_record = json!({
            "version": "v1",
            "ts_epoch_ms": now_epoch_ms(),
            "topic": topic,
            "breakthrough": breakthrough,
            "peers": peers
        });
        let gossip_id = deterministic_receipt_hash(&gossip_record);
        append_jsonl(&gossip_log_path(root), &gossip_record);
        let mut out = json!({
            "ok": true,
            "type": "p2p_gossip_seed_breakthrough",
            "lane": "core/layer2/ops",
            "ts_epoch_ms": now_epoch_ms(),
            "topic": topic,
            "breakthrough": breakthrough,
            "gossip_id": gossip_id,
            "gossip_log_path": gossip_log_path(root).display().to_string(),
            "propagation": {
                "peer_count": peers.len(),
                "peers": peers
            },
            "claim_evidence": [
                {
                    "id": "V6-NETWORK-004.3",
                    "claim": "breakthrough_gossip_emits_deterministic_propagation_receipts",
                    "evidence": {
                        "topic": topic,
                        "peer_count": rep.len(),
                        "gossip_id": gossip_id
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        persist_receipt(root, &out);
        print_json_line(&out);
        return 0;
    }

    if command == "idle-rss" {
        let feed = parse_cli_flag(argv, "feed").unwrap_or_else(|| "ai-news".to_string());
        let note = parse_cli_flag(argv, "note").unwrap_or_else(|| "interesting_update".to_string());
        let feed_record = json!({
            "version": "v1",
            "ts_epoch_ms": now_epoch_ms(),
            "feed": feed,
            "agent_comment": note
        });
        let comment_id = deterministic_receipt_hash(&feed_record);
        append_jsonl(&idle_feed_log_path(root), &feed_record);
        let mut out = json!({
            "ok": true,
            "type": "p2p_gossip_seed_idle_rss",
            "lane": "core/layer2/ops",
            "ts_epoch_ms": now_epoch_ms(),
            "feed": feed,
            "agent_comment": note,
            "comment_id": comment_id,
            "idle_feed_log_path": idle_feed_log_path(root).display().to_string(),
            "claim_evidence": [
                {
                    "id": "V6-NETWORK-004.4",
                    "claim": "idle_rss_ingestion_emits_feed_and_inter_agent_comment_receipts",
                    "evidence": {
                        "feed": feed,
                        "comment_id": comment_id
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        persist_receipt(root, &out);
        print_json_line(&out);
        return 0;
    }

    if command == "ranking-evolve" {
        let metric = parse_cli_flag(argv, "metric").unwrap_or_else(|| "ndcg@10".to_string());
        let delta = parse_f64(parse_cli_flag(argv, "delta"), 0.02).clamp(-1.0, 1.0);
        let mut ranking = read_json(&ranking_state_path(root)).unwrap_or_else(|| {
            json!({
                "version":"v1",
                "history":[]
            })
        });
        if !ranking.get("history").map(Value::is_array).unwrap_or(false) {
            ranking["history"] = Value::Array(Vec::new());
        }
        let prior_score = ranking
            .get("latest")
            .and_then(|v| v.get("score"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let next_score = (prior_score + delta).clamp(-1.0, 1.0);
        let entry = json!({
            "ts_epoch_ms": now_epoch_ms(),
            "metric": metric,
            "delta": delta,
            "prior_score": prior_score,
            "next_score": next_score
        });
        let mut history = ranking
            .get("history")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        history.push(entry.clone());
        ranking["history"] = Value::Array(history);
        ranking["latest"] = entry.clone();
        write_json(&ranking_state_path(root), &ranking);
        let mut out = json!({
            "ok": true,
            "type": "p2p_gossip_seed_ranking_evolve",
            "lane": "core/layer2/ops",
            "ts_epoch_ms": now_epoch_ms(),
            "metric": metric,
            "delta": delta,
            "prior_score": prior_score,
            "next_score": next_score,
            "ranking_state_path": ranking_state_path(root).display().to_string(),
            "claim_evidence": [
                {
                    "id": "V6-NETWORK-004.5",
                    "claim": "ranking_evolution_loop_emits_metric_delta_receipts",
                    "evidence": {
                        "metric": metric,
                        "delta": delta,
                        "next_score": next_score
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        persist_receipt(root, &out);
        print_json_line(&out);
        return 0;
    }

    let mut out = json!({
        "ok": false,
        "type": "p2p_gossip_seed_error",
        "lane": "core/layer2/ops",
        "ts_epoch_ms": now_epoch_ms(),
        "error": "unknown_command",
        "command": command
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    print_json_line(&out);
    1
}
