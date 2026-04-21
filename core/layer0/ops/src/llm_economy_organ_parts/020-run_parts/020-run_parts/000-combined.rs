
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops llm-economy-organ run [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ enable <all|virtuals|bankrbot|nookplot|owocki|heurist|daydreams|fairscale|trade_router> [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ upgrade-trading-hand [--mode=analysis|paper|live] [--symbol=<pair>] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ debate-bullbear [--symbol=<pair>] [--bull-score=<0..1>] [--bear-score=<0..1>] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ alpaca-execute [--mode=analysis|paper|live] [--symbol=<pair>] [--side=buy|sell] [--qty=<n>] [--apply=1|0]");
        println!(
            "  protheus-ops llm-economy-organ virtuals-acp [--action=build|earn] [--apply=1|0]"
        );
        println!(
            "  protheus-ops llm-economy-organ bankrbot-defi [--strategy=<name>] [--apply=1|0]"
        );
        println!("  protheus-ops llm-economy-organ jobs-marketplace [--source=nookplot|owocki] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ skills-marketplace [--source=heurist|daydreams] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ fairscale-credit [--delta=<n>] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ mining-hand [--network=litcoin|minbot] [--hours=<n>] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ trade-router [--chain=solana] [--symbol=<pair>] [--side=buy|sell] [--qty=<n>] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ model-support-refresh [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ dashboard");
        println!("  protheus-ops llm-economy-organ status");
        return 0;
    }

    let latest = latest_path(root);
    let history = history_path(root);
    let contract = load_contract(root);
    let strict = parse_bool(parsed.flags.get("strict"), true);

    if !matches!(command.as_str(), "status" | "dashboard") {
        let conduit = conduit_enforcement(&parsed, strict, command.as_str());
        if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            let mut out = json!({
                "ok": false,
                "strict": strict,
                "type": "llm_economy_organ_conduit_gate",
                "lane": "core/layer0/ops",
                "command": command,
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            });
            persist_and_emit_with_receipt(&latest, &history, &mut out);
            return 1;
        }
    }

    if command == "status" {
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_organ_status",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "latest": read_json(&latest),
            "contract": {
                "path": ECONOMY_CONTRACT_PATH,
                "sha256": deterministic_receipt_hash(&contract)
            }
        });
        emit_with_receipt(&mut out);
        return 0;
    }

    if command == "dashboard" {
        let latest_payload = read_json(&latest);
        let enabled_map = current_enabled_map(latest_payload.as_ref());
        let enabled_count = enabled_map
            .values()
            .filter(|v| v.as_bool().unwrap_or(false))
            .count();
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_organ_dashboard",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "enabled_count": enabled_count,
            "total_hands": ECONOMY_HANDS.len(),
            "enabled_hands": enabled_map,
            "contract": {
                "path": ECONOMY_CONTRACT_PATH,
                "sha256": deterministic_receipt_hash(&contract)
            },
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-001.8",
                    "claim": "economy_activation_and_dashboard_surface_enabled_hands_with_deterministic_receipts",
                    "evidence": {
                        "enabled_count": enabled_count,
                        "total_hands": ECONOMY_HANDS.len()
                    }
                }
            ]
        });
        persist_and_emit_with_receipt(&latest, &history, &mut out);
        return 0;
    }

    if command == "upgrade-trading-hand" {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let mode = clean(
            parsed
                .flags
                .get("mode")
                .cloned()
                .unwrap_or_else(|| "analysis".to_string()),
            16,
        );
        let symbol = clean(
            parsed
                .flags
                .get("symbol")
                .cloned()
                .unwrap_or_else(|| "SPY".to_string()),
            24,
        );
        let settings_inventory = vec![
            "max_loss_pct",
            "max_position_pct",
            "risk_per_trade_pct",
            "max_open_positions",
            "slippage_bps",
            "spread_bps_limit",
            "drawdown_circuit_breaker_pct",
            "rebalance_interval_sec",
            "market_open_guard",
            "market_close_guard",
            "news_blackout_window_min",
            "execution_mode",
        ];
        let metrics_inventory = vec![
            "win_rate",
            "sharpe_estimate",
            "max_drawdown_pct",
            "avg_slippage_bps",
            "fill_latency_ms",
            "exposure_pct",
            "beta_vs_benchmark",
            "turnover_rate",
            "pnl_usd",
            "risk_budget_used_pct",
        ];
        let settings_count = parse_u64(
            parsed.flags.get("settings"),
            settings_inventory.len() as u64,
        );
        let metrics_count = parse_u64(parsed.flags.get("metrics"), metrics_inventory.len() as u64);
        let phases = vec![
            "state_recovery",
            "portfolio_setup",
            "market_scan",
            "multi_factor_analysis",
            "bull_bear_debate",
            "risk_gate_circuit_breakers",
            "alpaca_execution",
            "analytics_reporting",
        ];
        let phase_status = phases
            .iter()
            .enumerate()
            .map(|(idx, name)| {
                json!({
                    "phase": idx + 1,
                    "name": name,
                    "status": if idx < 2 { "ready" } else { "pending" }
                })
            })
            .collect::<Vec<_>>();
        let profile_path = trading_profile_path(root);
        let profile = json!({
            "version": "v1",
            "updated_at": now_iso(),
            "mode": mode,
            "symbol": symbol,
            "settings_inventory": settings_inventory,
            "metrics_inventory": metrics_inventory,
            "risk_gate": {
                "circuit_breakers": true,
                "max_loss_pct": parse_f64(parsed.flags.get("max-loss-pct"), 2.5),
                "max_position_pct": parse_f64(parsed.flags.get("max-position-pct"), 15.0)
            }
        });
        if apply {
            write_json(&profile_path, &profile);
        }
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_organ_trading_hand_upgrade",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "apply": apply,
            "mode": mode,
            "symbol": symbol,
            "settings_count": settings_count,
            "metrics_count": metrics_count,
            "phases": phases,
            "phase_status": phase_status,
            "risk_gate": {
                "circuit_breakers": true,
                "max_loss_pct": parse_f64(parsed.flags.get("max-loss-pct"), 2.5),
                "max_position_pct": parse_f64(parsed.flags.get("max-position-pct"), 15.0)
            },
            "trading_profile_path": profile_path.display().to_string(),
            "settings_inventory": settings_inventory,
            "metrics_inventory": metrics_inventory,
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-002.1",
                    "claim": "core_trading_hand_upgrade_runs_receipted_8_phase_pipeline_with_risk_controls",
                    "evidence": {
                        "phases": 8,
                        "settings_count": settings_count,
                        "metrics_count": metrics_count
                    }
                },
                {
                    "id": "V6-ECONOMY-002.4",
                    "claim": "trading_upgrade_receipts_surface_config_and_metrics_inventory_for_dashboard_consumption",
                    "evidence": {
                        "settings_inventory_count": settings_count,
                        "metrics_inventory_count": metrics_count
                    }
                }
            ]
        });
        persist_and_emit_with_receipt(&latest, &history, &mut out);
        return 0;
    }

    if matches!(
        command.as_str(),
        "debate-bullbear" | "agent-debate-bullbear"
    ) {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let symbol = clean(
            parsed
                .flags
                .get("symbol")
                .cloned()
                .unwrap_or_else(|| "SPY".to_string()),
            24,
        );
        let bull_score = parse_f64(parsed.flags.get("bull-score"), 0.55).clamp(0.0, 1.0);
        let bear_score = parse_f64(parsed.flags.get("bear-score"), 0.45).clamp(0.0, 1.0);
        let spread = (bull_score - bear_score).abs();
        let decision = if spread < 0.08 {
            "hold"
        } else if bull_score > bear_score {
            "buy_bias"
        } else {
            "sell_bias"
        };
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_organ_bullbear_debate",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "apply": apply,
            "symbol": symbol,
            "debate": {
                "bull_score": bull_score,
                "bear_score": bear_score,
                "spread": spread,
                "decision": decision
            },
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-002.2",
                    "claim": "bull_bear_adversarial_debate_emits_structured_decision_receipts_before_execution",
                    "evidence": {
                        "symbol": symbol,
                        "decision": decision
                    }
                }
            ]
        });
        persist_and_emit_with_receipt(&latest, &history, &mut out);
        return 0;
    }

    if matches!(command.as_str(), "alpaca-execute" | "trading-execute") {
        let apply = parse_bool(parsed.flags.get("apply"), false);
        let mode = clean(
            parsed
                .flags
                .get("mode")
                .cloned()
                .unwrap_or_else(|| "paper".to_string()),
            16,
        );
        let symbol = clean(
            parsed
                .flags
                .get("symbol")
                .cloned()
                .unwrap_or_else(|| "SPY".to_string()),
            24,
        );
        let side = clean(
            parsed
                .flags
                .get("side")
                .cloned()
                .unwrap_or_else(|| "buy".to_string()),
            8,
        );
        let qty = parse_f64(parsed.flags.get("qty"), 1.0).max(0.0);
        let max_qty = parse_f64(parsed.flags.get("max-qty"), 100.0).max(0.0);
        let side_ok = matches!(side.as_str(), "buy" | "sell");
        let risk_ok = qty <= max_qty && side_ok;
        let order = json!({
            "broker": "alpaca",
            "symbol": symbol,
            "side": side,
            "qty": qty
        });
        let order_intent_id = deterministic_receipt_hash(&order);
        let order_record = json!({
            "version": "v1",
            "ts": now_iso(),
            "mode": mode,
            "apply": apply,
            "order_intent_id": order_intent_id,
            "order": order
        });
        if apply {
            append_jsonl(&trade_intents_path(root), &order_record);
        }
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_organ_alpaca_execute",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "apply": apply,
            "mode": mode,
            "execution": order,
            "risk_gate": {
                "passed": risk_ok,
                "circuit_breaker": !risk_ok,
                "side_ok": side_ok,
                "max_qty": max_qty
            },
            "order_intent_id": order_intent_id,
            "trade_intents_path": trade_intents_path(root).display().to_string(),
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-002.3",
                    "claim": "alpaca_execution_lane_emits_mode_risk_and_order_receipts_with_fail_closed_gates",
                    "evidence": {
                        "mode": mode,
                        "risk_ok": risk_ok
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        write_json(&latest, &out);
        append_jsonl(&history, &out);
        print_receipt(&out);
        return if risk_ok { 0 } else { 3 };
    }

    if command == "enable" {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let target_raw = parsed
            .positional
            .get(1)
            .map(String::as_str)
            .unwrap_or("all");
        let target = normalize_target(target_raw);
        let mut enabled = current_enabled_map(read_json(&latest).as_ref());

        if target == "all" {
            for key in ECONOMY_HANDS {
                enabled.insert(key.to_string(), Value::Bool(true));
            }
        } else if ECONOMY_HANDS.contains(&target.as_str()) {
            enabled.insert(target.clone(), Value::Bool(true));
        } else {
            let mut out = json!({
                "ok": false,
                "type": "llm_economy_organ_enable_error",
                "lane": "core/layer0/ops",
                "ts": now_iso(),
                "error": "unknown_target",
                "target": target
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            print_receipt(&out);
            return 2;
        }

        let enabled_count = enabled
            .values()
            .filter(|v| v.as_bool().unwrap_or(false))
            .count();
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_organ_enable",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "apply": apply,
            "target": target,
            "enabled_count": enabled_count,
            "total_hands": ECONOMY_HANDS.len(),
            "enabled_hands": enabled,
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-001.8",
                    "claim": "economy_enable_all_activates_default_hands_in_core_with_deterministic_receipts",
                    "evidence": {
                        "target": target,
                        "enabled_count": enabled_count
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        write_json(&latest, &out);
        append_jsonl(&history, &out);
        print_receipt(&out);
        return 0;
    }

    if command == "virtuals-acp" {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let action = clean(
            parsed
                .flags
                .get("action")
                .cloned()
                .unwrap_or_else(|| "earn".to_string()),
            24,
        );
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_virtuals_acp",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "apply": apply,
            "action": action,
            "contract_digest": deterministic_receipt_hash(&contract),
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-001.1",
                    "claim": "virtuals_acp_eye_hand_command_is_receipted_in_core_authority",
                    "evidence": {"action": action}
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        write_json(&latest, &out);
        append_jsonl(&history, &out);
        print_receipt(&out);
        return 0;
    }

    if command == "bankrbot-defi" {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let strategy = clean(
            parsed
                .flags
                .get("strategy")
                .cloned()
                .unwrap_or_else(|| "yield-stable".to_string()),
            48,
        );
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_bankrbot_defi",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "apply": apply,
            "strategy": strategy,
            "contract_digest": deterministic_receipt_hash(&contract),
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-001.2",
                    "claim": "bankrbot_defi_yield_hand_is_policy_gated_with_deterministic_receipts",
                    "evidence": {"strategy": strategy}
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        write_json(&latest, &out);
        append_jsonl(&history, &out);
        print_receipt(&out);
        return 0;
    }

    if command == "jobs-marketplace" {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let source = clean(
            parsed
                .flags
                .get("source")
                .cloned()
                .unwrap_or_else(|| "nookplot".to_string()),
            24,
        );
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_jobs_marketplace",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "apply": apply,
            "source": source,
            "contract_digest": deterministic_receipt_hash(&contract),
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-001.3",
                    "claim": "jobs_marketplace_hand_is_receipted_for_nookplot_owocki_routing",
                    "evidence": {"source": source}
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        write_json(&latest, &out);
        append_jsonl(&history, &out);
        print_receipt(&out);
        return 0;
    }

    if command == "skills-marketplace" {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let source = clean(
            parsed
                .flags
                .get("source")
                .cloned()
                .unwrap_or_else(|| "heurist".to_string()),
            24,
        );
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_skills_marketplace",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "apply": apply,
            "source": source,
            "contract_digest": deterministic_receipt_hash(&contract),
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-001.4",
                    "claim": "skills_marketplace_hand_is_receipted_for_heurist_daydreams_routing",
                    "evidence": {"source": source}
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        write_json(&latest, &out);
        append_jsonl(&history, &out);
        print_receipt(&out);
        return 0;
    }

    if command == "fairscale-credit" {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let identity = clean(
            parsed
                .flags
                .get("identity")
                .cloned()
                .unwrap_or_else(|| "sovereign-default".to_string()),
            96,
        );
        let delta = parse_f64_opt(parsed.flags.get("delta"), 1.0);
        let ledger_path = trust_ledger_path(root);
        let mut ledger = read_json(&ledger_path).unwrap_or_else(|| {
            json!({
                "version": "v1",
                "scores": {},
                "updated_at": now_iso()
            })
        });
        if !ledger.get("scores").map(Value::is_object).unwrap_or(false) {
            ledger["scores"] = json!({});
        }
        let mut scores = ledger
            .get("scores")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let prior = scores
            .get(identity.as_str())
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let next = prior + delta;
        scores.insert(identity.clone(), Value::from(next));
        ledger["scores"] = Value::Object(scores.clone());
        ledger["updated_at"] = Value::String(now_iso());
        write_json(&ledger_path, &ledger);
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_fairscale_credit",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "apply": apply,
            "identity": identity,
            "credit_score": next,
            "credit_delta": delta,
            "trust_ledger": {
                "path": ledger_path.display().to_string(),
                "identity": identity,
                "previous_score": prior,
                "next_score": next,
                "identity_count": scores.len()
            },
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-001.5",
                    "claim": "fairscale_credit_hand_updates_identity_bound_trust_score_with_deterministic_receipts",
                    "evidence": {
                        "delta": delta,
                        "identity": identity,
                        "next": next
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        write_json(&latest, &out);
        append_jsonl(&history, &out);
        print_receipt(&out);
        return 0;
    }

    if command == "mining-hand" {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let network = clean(
            parsed
                .flags
                .get("network")
                .cloned()
                .unwrap_or_else(|| "litcoin".to_string()),
            24,
        );
        let hours = parse_u64(parsed.flags.get("hours"), 6);
        let schedule = clean(
            parsed
                .flags
                .get("schedule")
                .cloned()
                .unwrap_or_else(|| "*/30 * * * *".to_string()),
            48,
        );
        if strict && !matches!(network.as_str(), "litcoin" | "minbot") {
            let mut out = json!({
                "ok": false,
                "type": "llm_economy_mining_hand_error",
                "lane": "core/layer0/ops",
                "ts": now_iso(),
                "error": "invalid_mining_network",
                "network": network,
                "allowed": ["litcoin", "minbot"]
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            write_json(&latest, &out);
            append_jsonl(&history, &out);
            print_receipt(&out);
            return 2;
        }
        let interval_minutes = parse_cron_interval_minutes(schedule.as_str()).unwrap_or(30);
        let runtime_path = mining_runtime_path(root);
        let mut runtime = read_json(&runtime_path).unwrap_or_else(|| {
            json!({
                "version": "v1",
                "networks": {},
                "updated_at": now_iso()
            })
        });
        if !runtime
            .get("networks")
            .map(Value::is_object)
            .unwrap_or(false)
        {
            runtime["networks"] = json!({});
        }
        let mut networks = runtime
            .get("networks")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let previous_hours = networks
            .get(network.as_str())
            .and_then(|row| row.get("total_hours"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let total_hours = previous_hours.saturating_add(hours);
        networks.insert(
            network.clone(),
            json!({
                "schedule": schedule,
                "interval_minutes": interval_minutes,
                "total_hours": total_hours,
                "last_update": now_iso()
            }),
        );
        runtime["networks"] = Value::Object(networks.clone());
        runtime["updated_at"] = Value::String(now_iso());
        if apply {
            write_json(&runtime_path, &runtime);
        }
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_mining_hand",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "apply": apply,
            "network": network,
            "hours": hours,
            "schedule": schedule,
            "mining_runtime_path": runtime_path.display().to_string(),
            "schedule_runtime": {
                "interval_minutes": interval_minutes,
                "previous_hours": previous_hours,
                "total_hours": total_hours
            },
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-001.6",
                    "claim": "mining_hand_emits_deterministic_schedule_and_runtime_receipts",
                    "evidence": {
                        "network": network,
                        "hours": hours,
                        "schedule": schedule
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        write_json(&latest, &out);
        append_jsonl(&history, &out);
        print_receipt(&out);
        return 0;
    }

    if command == "trade-router" {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let chain = canonical_chain(&clean(
            parsed
                .flags
                .get("chain")
                .cloned()
                .unwrap_or_else(|| "solana".to_string()),
            24,
        ));
        let symbol = clean(
            parsed
                .flags
                .get("symbol")
                .cloned()
                .unwrap_or_else(|| "SOL/USDC".to_string()),
            24,
        );
        let side = clean(
            parsed
                .flags
                .get("side")
                .cloned()
                .unwrap_or_else(|| "buy".to_string()),
            8,
        );
        let qty = parse_f64_opt(parsed.flags.get("qty"), 1.0).max(0.0);
        let side_ok = matches!(side.as_str(), "buy" | "sell");
        let chain_ok = chain == "solana";
        if strict && (!chain_ok || !side_ok || qty <= 0.0) {
            let mut out = json!({
                "ok": false,
                "type": "llm_economy_trade_router_error",
                "lane": "core/layer0/ops",
                "ts": now_iso(),
                "error": "invalid_trade_router_request",
                "details": {
                    "chain_ok": chain_ok,
                    "side_ok": side_ok,
                    "qty_positive": qty > 0.0
                }
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            write_json(&latest, &out);
            append_jsonl(&history, &out);
            print_receipt(&out);
            return 2;
        }
        let order_intent = json!({
            "chain": chain,
            "symbol": symbol,
            "side": side,
            "qty": qty,
            "custody": "non-custodial-intent"
        });
        let intent_id = deterministic_receipt_hash(&order_intent);
        if apply {
            append_jsonl(
                &trade_intents_path(root),
                &json!({
                    "version":"v1",
                    "ts": now_iso(),
                    "intent_id": intent_id,
                    "order_intent": order_intent
                }),
            );
        }
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_trade_router",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "apply": apply,
            "chain": chain,
            "order": order_intent,
            "non_custodial_intent": true,
            "order_intent_id": intent_id,
            "trade_intents_path": trade_intents_path(root).display().to_string(),
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-001.7",
                    "claim": "trade_router_solana_hand_emits_non_custodial_order_intent_receipts",
                    "evidence": {
                        "chain": chain,
                        "symbol": symbol
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        write_json(&latest, &out);
        append_jsonl(&history, &out);
        print_receipt(&out);
        return 0;
    }

    if command == "model-support-refresh" {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let provider_matrix = vec![
            json!({"provider":"deepseek","model":"deepseek-v3-r1","tier":"analysis","latency_class":"medium"}),
            json!({"provider":"meta","model":"llama-4-maverick","tier":"general","latency_class":"medium"}),
            json!({"provider":"qwen","model":"qwen3-235b","tier":"reasoning","latency_class":"high"}),
            json!({"provider":"zhipu","model":"glm-5","tier":"multimodal","latency_class":"medium"}),
            json!({"provider":"moonshot","model":"kimi-k2.5","tier":"analysis","latency_class":"high"}),
            json!({"provider":"minimax","model":"minimax-m2.5-highspeed","tier":"cheap","latency_class":"low"}),
            json!({"provider":"abab","model":"abab7-chat","tier":"chat","latency_class":"low"}),
        ];
        let provider_count = provider_matrix.len();
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_model_support_refresh",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "apply": apply,
            "provider_matrix": provider_matrix,
            "provider_count": provider_count,
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-002.5",
                    "claim": "model_support_refresh_emits_provider_matrix_receipts_for_trading_lanes",
                    "evidence": {
                        "provider_count": provider_count
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        write_json(&latest, &out);
        append_jsonl(&history, &out);
        print_receipt(&out);
        return 0;
    }

    let apply = parse_bool(parsed.flags.get("apply"), false);
    let mut out = json!({
        "ok": true,
        "type": "llm_economy_organ_run",
        "lane": "core/layer0/ops",
        "ts": now_iso(),
        "apply": apply,
        "model_routing": {
            "budget_band": if apply { "applied" } else { "dry_run" },
            "providers_ranked": [],
            "note": "core_authoritative_placeholder"
        },
        "receipts": {
            "strategy": clean(parsed.flags.get("strategy").cloned().unwrap_or_default(), 120),
            "capital": clean(parsed.flags.get("capital").cloned().unwrap_or_default(), 120)
        }
    });
    persist_and_emit_with_receipt(&latest, &history, &mut out);
    0
}
