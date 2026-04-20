        let (proof, leaf) = if proof_requested && !account.is_empty() {
            let entry = format!(
                "balance:{}:{:.8}",
                account,
                ledger
                    .get("balances")
                    .and_then(Value::as_object)
                    .map(|m| m.get(&account).and_then(Value::as_f64).unwrap_or(0.0))
                    .unwrap_or(0.0)
            );
            let idx = leaves.iter().position(|v| v == &entry).unwrap_or(0);
            (
                Value::Array(merkle_proof(&leaves, idx)),
                Value::String(entry),
            )
        } else {
            (Value::Array(Vec::new()), Value::Null)
        };

        emit(
            root,
            json!({
                "ok": true,
                "type": "network_protocol_global_merkle_root",
                "lane": "core/layer0/ops",
                "global_merkle_root": root_hash,
                "policy_hash": policy_hash,
                "leaf_count": leaves.len(),
                "inclusion_leaf": leaf,
                "inclusion_proof": proof,
                "root_progression_head": ledger.get("root_head").cloned().unwrap_or(Value::Null),
                "layer_map": ["0","1","2"],
                "claim_evidence": [
                    {
                        "id": "V8-NETWORK-002.2",
                        "claim": "global_state_root_is_deterministically_derived_from_receipt_and_policy_roots",
                        "evidence": {"leaf_count": leaves.len(), "proof_requested": proof_requested}
                    }
                ]
            }),
        )
    } else if command == "emission" {
        let gate_ok = gate_action(root, "tokenomics:emission");
        if !gate_ok {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_emission_curve",
                    "lane": "core/layer0/ops",
                    "error": "directive_gate_denied",
                    "gate_action": "tokenomics:emission",
                    "layer_map": ["0","1","2"],
                    "claim_evidence": [
                        {
                            "id": "V8-NETWORK-002.3",
                            "claim": "halving_style_emission_schedule_is_deterministic_and_receipted",
                            "evidence": {"allowed": false, "reason": "directive_gate_denied"}
                        }
                    ]
                }),
            );
        }
        let height = parse_u64(parsed.flags.get("height"), 0);
        let interval = parse_u64(parsed.flags.get("halving-interval"), 210_000).max(1);
        let initial = parse_f64(parsed.flags.get("initial-issuance"), 50.0).max(0.0);
        let epoch = height / interval;
        let issuance = initial / f64::powi(2.0, epoch as i32);
        let next_halving_height = (epoch + 1) * interval;

        let mut ledger = load_ledger(root);
        if let Some(obj) = ledger.as_object_mut() {
            obj.insert(
                "emission".to_string(),
                json!({
                    "halving_interval": interval,
                    "initial_issuance": initial,
                    "epoch": epoch,
                    "issuance_per_epoch": issuance,
                    "next_halving_height": next_halving_height
                }),
            );
        }

        match commit_ledger(
            root,
            ledger,
            "emission_update",
            json!({"height": height, "epoch": epoch, "issuance": issuance}),
        ) {
            Ok(updated) => emit(
                root,
                json!({
                    "ok": true,
                    "type": "network_protocol_emission_curve",
                    "lane": "core/layer0/ops",
                    "height": height,
                    "halving_interval": interval,
                    "epoch": epoch,
                    "issuance_per_epoch": issuance,
                    "next_halving_height": next_halving_height,
                    "network_state_root": updated.get("root_head").cloned().unwrap_or(Value::Null),
                    "layer_map": ["0","1","2"],
                    "claim_evidence": [
                        {
                            "id": "V8-NETWORK-002.3",
                            "claim": "halving_style_emission_schedule_is_deterministic_and_receipted",
                            "evidence": {"epoch": epoch, "issuance_per_epoch": issuance}
                        }
                    ]
                }),
            ),
            Err(err) => emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_emission_curve",
                    "lane": "core/layer0/ops",
                    "error": clean(err, 220)
                }),
            ),
        }
    } else if command == "zk-claim" {
        let gate_ok = gate_action(root, "tokenomics:zk-claim");
        if !gate_ok {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_zk_claim",
                    "lane": "core/layer0/ops",
                    "error": "directive_gate_denied",
                    "gate_action": "tokenomics:zk-claim",
                    "layer_map": ["0","1","2","adapter"],
                    "claim_evidence": [
                        {
                            "id": "V8-NETWORK-002.4",
                            "claim": "private_claim_verification_is_policy_gated_and_receipted",
                            "evidence": {"allowed": false, "reason": "directive_gate_denied"}
                        }
                    ]
                }),
            );
        }
        let claim_id = clean(
            parsed
                .flags
                .get("claim-id")
                .cloned()
                .unwrap_or_else(|| "claim:unknown".to_string()),
            140,
        );
        let commitment = clean(
            parsed.flags.get("commitment").cloned().unwrap_or_default(),
            256,
        );
        let challenge = clean(
            parsed.flags.get("challenge").cloned().unwrap_or_default(),
            256,
        );
        let public_input = clean(
            parsed
                .flags
                .get("public-input")
                .cloned()
                .unwrap_or_else(|| "directive-compliant".to_string()),
            320,
        );
        let strict = parse_bool(parsed.flags.get("strict"), false);

        let expected_challenge = sha256_hex_str(&format!("{}:{}", commitment, public_input));
        let verified = !commitment.is_empty()
            && !challenge.is_empty()
            && challenge.eq_ignore_ascii_case(&expected_challenge);
        let ok = verified || !strict;

