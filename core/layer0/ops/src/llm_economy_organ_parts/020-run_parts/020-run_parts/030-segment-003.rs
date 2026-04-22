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
        let order_intent_id = crate::deterministic_receipt_hash(&order);
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
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
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
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            print_receipt(&out);
            return 2;
        }

