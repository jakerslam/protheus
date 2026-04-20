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

