                .cloned()
                .unwrap_or_else(|| "status".to_string()),
            24,
        )
        .to_ascii_lowercase();
        let strict = parse_bool(parsed.flags.get("strict"), true);
        let mut rows = read_jsonl(&consensus_ledger_path(root));
        let mut errors = Vec::<String>::new();
        if op == "append" {
            let receipt_hash = clean(
                parsed
                    .flags
                    .get("receipt-hash")
                    .cloned()
                    .unwrap_or_default(),
                160,
            );
            let causality_hash = clean(
                parsed
                    .flags
                    .get("causality-hash")
                    .cloned()
                    .unwrap_or_default(),
                160,
            );
            if strict && (receipt_hash.is_empty() || causality_hash.is_empty()) {
                errors.push("consensus_append_requires_receipt_and_causality_hash".to_string());
            }
            if errors.is_empty() {
                let previous_hash = rows
                    .last()
                    .and_then(|row| row.get("event_hash"))
                    .and_then(Value::as_str)
                    .unwrap_or("GENESIS")
                    .to_string();
                let event_seed = json!({
                    "seq": rows.len() + 1,
                    "ts": now_iso(),
                    "receipt_hash": receipt_hash,
                    "causality_hash": causality_hash,
                    "previous_hash": previous_hash
                });
                let event_hash = sha256_hex_str(&event_seed.to_string());
                let event = json!({
                    "seq": rows.len() + 1,
                    "ts": now_iso(),
                    "receipt_hash": receipt_hash,
                    "causality_hash": causality_hash,
                    "previous_hash": previous_hash,
                    "event_hash": event_hash
                });
                let _ = append_jsonl(&consensus_ledger_path(root), &event);
                rows.push(event);
            }
        } else if !matches!(op.as_str(), "verify" | "status") {
            errors.push(format!("consensus_op_unknown:{op}"));
        }
        let mut verify_errors = Vec::<String>::new();
        let mut prev = "GENESIS".to_string();
        for (idx, row) in rows.iter().enumerate() {
            let observed_prev = row
                .get("previous_hash")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if observed_prev != prev {
                verify_errors.push(format!("consensus_previous_hash_mismatch_at:{idx}"));
            }
            let seed = json!({
                "seq": row.get("seq").cloned().unwrap_or(Value::Null),
                "ts": row.get("ts").cloned().unwrap_or(Value::Null),
                "receipt_hash": row.get("receipt_hash").cloned().unwrap_or(Value::Null),
                "causality_hash": row.get("causality_hash").cloned().unwrap_or(Value::Null),
                "previous_hash": row.get("previous_hash").cloned().unwrap_or(Value::Null)
            });
            let expected = sha256_hex_str(&seed.to_string());
            let observed = row
                .get("event_hash")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if expected != observed {
                verify_errors.push(format!("consensus_event_hash_mismatch_at:{idx}"));
            }
            prev = observed.to_string();
        }
        let chain_valid = verify_errors.is_empty();
        if op == "verify" {
            errors.extend(verify_errors.clone());
        }
        emit(
            root,
            json!({
                "ok": if strict { errors.is_empty() } else { true },
                "type": "network_protocol_consensus",
                "lane": "core/layer0/ops",
                "op": op,
                "strict": strict,
                "event_count": rows.len(),
                "chain_valid": chain_valid,
                "verify_errors": verify_errors,
                "errors": errors,
                "consensus_head": prev,
                "claim_evidence": [
                    {
                        "id": "V7-NETWORK-001.2",
                        "claim": "consensus_ledger_is_receipt_and_causality_hash_chained_with_tamper_detection",
                        "evidence": {"event_count": rows.len(), "chain_valid": chain_valid}
                    }
                ]
            }),
        )
    } else if command == "rsi-boundary" {
        let strict = parse_bool(parsed.flags.get("strict"), true);
        let stage = clean(
            parsed
                .flags
                .get("stage")
                .cloned()
                .unwrap_or_else(|| "sandbox".to_string()),
            24,
        )
        .to_ascii_lowercase();
        let action = clean(
            parsed
                .flags
                .get("action")
                .cloned()
                .unwrap_or_else(|| "simulate".to_string()),
            24,
        )
        .to_ascii_lowercase();
        let oversight_approval = parse_bool(parsed.flags.get("oversight-approval"), false);
        let mature_stage = matches!(stage.as_str(), "growth" | "expansion" | "mature");
        let gate_ok = gate_action(root, &format!("network-rsi:{stage}:{action}"));
        let mut errors = Vec::<String>::new();
        if strict && mature_stage && !oversight_approval {
            errors.push("rsi_oversight_approval_required".to_string());
        }
        if strict && !gate_ok {
            errors.push("directive_gate_denied".to_string());
        }
        emit(
            root,
            json!({
                "ok": if strict { errors.is_empty() } else { true },
                "type": "network_protocol_rsi_boundary",
                "lane": "core/layer0/ops",
                "strict": strict,
                "stage": stage,
                "action": action,
                "oversight_approval": oversight_approval,
                "directive_gate_ok": gate_ok,
                "errors": errors,
                "claim_evidence": [
                    {
                        "id": "V7-NETWORK-001.3",
                        "claim": "network_rsi_growth_paths_are_stage_gated_and_require_oversight_before_high_risk_actions",
                        "evidence": {"stage": stage, "action": action, "oversight_approval": oversight_approval}
                    }
                ]
            }),
        )
    } else if command == "join-hyperspace" {
        let strict = parse_bool(parsed.flags.get("strict"), true);
        let node = clean(
            parsed
                .flags
                .get("node")
                .cloned()
                .unwrap_or_else(|| "node-local".to_string()),
            120,
        );
        let admission_token = clean(
            parsed
                .flags
                .get("admission-token")
                .cloned()
                .unwrap_or_else(|| "local-admission-token".to_string()),
            160,
        );
        let stake = parse_f64(parsed.flags.get("stake"), 10.0).max(0.0);
