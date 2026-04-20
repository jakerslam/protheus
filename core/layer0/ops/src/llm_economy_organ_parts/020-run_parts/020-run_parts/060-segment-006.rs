
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

