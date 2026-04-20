    if command == "validate-action-envelope" {
        let payload = match decode_payload_or_emit(
            root,
            parsed.flags.get("payload-base64"),
            "directive_kernel_validate_action_envelope",
        ) {
            Ok(value) => value,
            Err(code) => return code,
        };
        let envelope = payload
            .get("action_envelope")
            .cloned()
            .unwrap_or(Value::Null);
        let validation = match validate_action_envelope(root, &envelope) {
            Ok(value) => value,
            Err(err) => {
                return emit_receipt(
                    root,
                    json!({
                        "ok": false,
                        "type": "directive_kernel_validate_action_envelope",
                        "lane": "core/layer0/ops",
                        "error": err
                    }),
                );
            }
        };
        let ok = validation
            .get("allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || validation
                .get("requires_approval")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        return emit_receipt(
            root,
            json!({
                "ok": ok,
                "type": "directive_kernel_validate_action_envelope",
                "lane": "core/layer0/ops",
                "validation": validation
            }),
        );
    }

    if command == "tier-conflict" {
        let payload = match decode_payload_or_emit(
            root,
            parsed.flags.get("payload-base64"),
            "directive_kernel_tier_conflict",
        ) {
            Ok(value) => value,
            Err(code) => return code,
        };
        let lower = payload
            .get("lower_tier_action")
            .cloned()
            .unwrap_or(Value::Null);
        let higher = payload
            .get("higher_tier_directive")
            .cloned()
            .unwrap_or(Value::Null);
        let conflict = check_tier_conflict(&lower, &higher);
        return emit_receipt(
            root,
            json!({
                "ok": true,
                "type": "directive_kernel_tier_conflict",
                "lane": "core/layer0/ops",
                "conflict": conflict
            }),
        );
    }

    if command == "bridge-rsi" {
        let proposal = parsed
            .flags
            .get("proposal")
            .cloned()
            .unwrap_or_else(|| "propose_loop_optimization".to_string());
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let action = format!("rsi:{}", clean(&proposal, 220).to_ascii_lowercase());
        let eval = evaluate_action(root, &action);
        let allowed = eval
            .get("allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let rollback_pointer = format!(
            "rollback://directive_bridge/{}",
            &sha256_hex_str(&format!("{}:{}", now_iso(), proposal))[..18]
        );

        if apply && !allowed {
            let _ = append_jsonl(
                &history_path(root),
                &json!({
                    "ok": false,
                    "type": "directive_kernel_rsi_bridge_rollback",
                    "ts": now_iso(),
                    "proposal": clean(&proposal, 220),
                    "rollback_pointer": rollback_pointer,
                    "reason": "directive_gate_denied"
                }),
            );
        }

        return emit_receipt(
            root,
            json!({
                "ok": allowed,
                "type": "directive_kernel_rsi_bridge",
                "lane": "core/layer0/ops",
                "proposal": clean(&proposal, 220),
                "apply": apply,
                "allowed": allowed,
                "evaluation": eval,
                "rollback_pointer": rollback_pointer,
                "layer_map": ["0","1","2","3"],
                "claim_evidence": [
                    {
                        "id": "V8-DIRECTIVES-001.4",
                        "claim": "rsi_and_inversion_mutations_are_bound_to_prime_and_derived_directive_checks",
                        "evidence": {"allowed": allowed, "proposal": clean(proposal, 220)}
                    }
                ]
            }),
        );
    }

