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
