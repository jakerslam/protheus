    if command == "compliance-check" {
        let action = parsed
            .flags
            .get("action")
            .cloned()
            .unwrap_or_else(|| "unknown_action".to_string());
        let eval = evaluate_action(root, &action);
        return emit_receipt(
            root,
            json!({
                "ok": eval.get("allowed").and_then(Value::as_bool).unwrap_or(false),
                "type": "directive_kernel_compliance_check",
                "lane": "core/layer0/ops",
                "evaluation": eval,
                "gates": {
                    "conduit_required": true,
                    "prime_derived_hierarchy_enforced": true,
                    "override_flags_ignored": true,
                    "signature_verification_required": true
                },
                "layer_map": ["0","1","2"],
                "claim_evidence": [
                    {
                        "id": "V8-DIRECTIVES-001.3",
                        "claim": "all_actions_must_pass_directive_compliance_gate_before_execution",
                        "evidence": {
                            "action": clean(action, 220),
                            "allowed": eval.get("allowed").cloned().unwrap_or(Value::Bool(false)),
                            "deny_hits": eval.get("deny_hits").cloned().unwrap_or(Value::Array(Vec::new())),
                            "invalid_signature_hits": eval.get("invalid_signature_hits").cloned().unwrap_or(Value::Array(Vec::new()))
                        }
                    }
                ]
            }),
        );
    }

    if command == "parse-yaml" {
        let text = match decode_base64_text(parsed.flags.get("text-base64"), "text_base64") {
            Ok(value) => value,
            Err(err) => {
                return emit_receipt(
                    root,
                    json!({
                        "ok": false,
                        "type": "directive_kernel_parse_yaml",
                        "lane": "core/layer0/ops",
                        "error": err
                    }),
                );
            }
        };
        let parsed_yaml = match yaml_to_json(&text) {
            Ok(value) => value,
            Err(err) => {
                return emit_receipt(
                    root,
                    json!({
                        "ok": false,
                        "type": "directive_kernel_parse_yaml",
                        "lane": "core/layer0/ops",
                        "error": err
                    }),
                );
            }
        };
        return emit_receipt(
            root,
            json!({
                "ok": true,
                "type": "directive_kernel_parse_yaml",
                "lane": "core/layer0/ops",
                "parsed": parsed_yaml
            }),
        );
    }

    if command == "validate-tier1-quality" {
        let text = match decode_base64_text(parsed.flags.get("text-base64"), "text_base64") {
            Ok(value) => value,
            Err(err) => {
                return emit_receipt(
                    root,
                    json!({
                        "ok": false,
                        "type": "directive_kernel_validate_tier1_quality",
                        "lane": "core/layer0/ops",
                        "error": err
                    }),
                );
            }
        };
        let directive_id = parsed
            .flags
            .get("directive-id")
            .map(|value| clean(value, 128))
            .unwrap_or_else(|| "unknown".to_string());
        let validation = validate_tier1_directive_quality(&text, &directive_id);
        return emit_receipt(
            root,
            json!({
                "ok": validation.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "type": "directive_kernel_validate_tier1_quality",
                "lane": "core/layer0/ops",
                "validation": validation
            }),
        );
    }

    if command == "active-directives" {
        let allow_missing = parse_bool(parsed.flags.get("allow-missing"), false);
        let allow_weak_tier1 = parse_bool(parsed.flags.get("allow-weak-tier1"), false);
        let directives = match load_active_directives(root, allow_missing, allow_weak_tier1) {
            Ok(value) => value,
            Err(err) => {
                return emit_receipt(
                    root,
                    json!({
                        "ok": false,
                        "type": "directive_kernel_active_directives",
                        "lane": "core/layer0/ops",
                        "error": err
                    }),
                );
            }
        };
        return emit_receipt(
            root,
            json!({
                "ok": true,
                "type": "directive_kernel_active_directives",
                "lane": "core/layer0/ops",
                "directives": directives
            }),
        );
    }

    if command == "merge-constraints" {
        let payload = match decode_payload_or_emit(
            root,
            parsed.flags.get("payload-base64"),
            "directive_kernel_merge_constraints",
        ) {
            Ok(value) => value,
            Err(code) => return code,
        };
        let directives = payload
            .get("directives")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let constraints = merge_active_constraints(&directives);
        return emit_receipt(
            root,
            json!({
                "ok": true,
                "type": "directive_kernel_merge_constraints",
                "lane": "core/layer0/ops",
                "constraints": constraints
            }),
        );
    }

