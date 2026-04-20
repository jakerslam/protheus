    if command == "prime-sign"
        || (command == "prime"
            && parsed
                .positional
                .get(1)
                .map(|v| v.eq_ignore_ascii_case("sign"))
                .unwrap_or(false))
    {
        let directive = parsed
            .flags
            .get("directive")
            .cloned()
            .unwrap_or_else(|| "deny:unsafe_action".to_string());
        let signer = parsed
            .flags
            .get("signer")
            .cloned()
            .unwrap_or_else(|| "operator".to_string());
        let allow_unsigned = parse_bool(parsed.flags.get("allow-unsigned"), false);
        if !allow_unsigned && !signing_key_present() {
            return emit_receipt(
                root,
                json!({
                    "ok": false,
                    "type": "directive_kernel_prime_sign",
                    "lane": "core/layer0/ops",
                    "error": "missing_signing_key",
                    "signing_env": SIGNING_ENV,
                    "claim_evidence": [
                        {
                            "id": "V8-DIRECTIVES-001.1",
                            "claim": "prime_directives_are_append_only_signed_objects_not_inline_mutations",
                            "evidence": {"accepted": false, "reason": "missing_signing_key"}
                        }
                    ]
                }),
            );
        }

        let entry = match append_directive_entry(
            root,
            "prime",
            &directive,
            &signer,
            None,
            None,
            "operator_sign",
        ) {
            Ok(v) => v,
            Err(err) => {
                return emit_receipt(
                    root,
                    json!({
                        "ok": false,
                        "type": "directive_kernel_prime_sign",
                        "lane": "core/layer0/ops",
                        "error": clean(&err, 240),
                        "claim_evidence": [
                            {
                                "id": "V8-DIRECTIVES-001.1",
                                "claim": "prime_directives_are_append_only_signed_objects_not_inline_mutations",
                                "evidence": {"error": clean(&err, 240)}
                            }
                        ]
                    }),
                );
            }
        };
        return emit_receipt(
            root,
            json!({
                "ok": true,
                "type": "directive_kernel_prime_sign",
                "lane": "core/layer0/ops",
                "entry": entry,
                "policy_hash": directive_vault_hash(root),
                "layer_map": ["0","1","2"],
                "claim_evidence": [
                    {
                        "id": "V8-DIRECTIVES-001.1",
                        "claim": "prime_directives_are_append_only_signed_objects_not_inline_mutations",
                        "evidence": {
                            "entry_id": entry.get("id").cloned().unwrap_or(Value::Null),
                            "signature_present": entry.get("signature").and_then(Value::as_str).map(|s| !s.is_empty()).unwrap_or(false)
                        }
                    }
                ]
            }),
        );
    }

    if command == "derive" {
        let parent_hint = parsed.flags.get("parent").cloned().unwrap_or_default();
        let directive = parsed
            .flags
            .get("directive")
            .cloned()
            .unwrap_or_else(|| "allow:bounded_autonomy".to_string());
        let signer = parsed
            .flags
            .get("signer")
            .cloned()
            .unwrap_or_else(|| "system".to_string());
        let allow_unsigned = parse_bool(parsed.flags.get("allow-unsigned"), false);
        if !allow_unsigned && !signing_key_present() {
            return emit_receipt(
                root,
                json!({
                    "ok": false,
                    "type": "directive_kernel_derive",
                    "lane": "core/layer0/ops",
                    "error": "missing_signing_key",
                    "signing_env": SIGNING_ENV,
                    "layer_map": ["0","1","2"],
                    "claim_evidence": [
                        {
                            "id": "V8-DIRECTIVES-001.2",
                            "claim": "derived_directives_require_parent_linkage_and_fail_on_inheritance_conflict",
                            "evidence": {"accepted": false, "reason": "missing_signing_key"}
                        }
                    ]
                }),
            );
        }

        let vault = load_vault(root);
        let Some(parent) = resolve_parent(&vault, &parent_hint) else {
            return emit_receipt(
                root,
                json!({
                    "ok": false,
                    "type": "directive_kernel_derive",
                    "lane": "core/layer0/ops",
                    "error": "parent_not_found",
                    "parent": clean(parent_hint, 320),
                    "layer_map": ["0","1","2"],
                    "claim_evidence": [
                        {
                            "id": "V8-DIRECTIVES-001.2",
                            "claim": "derived_directives_require_parent_linkage_and_fail_on_inheritance_conflict",
                            "evidence": {"accepted": false, "reason": "parent_not_found"}
                        }
                    ]
                }),
            );
        };

        let (child_kind, child_pattern) = normalize_rule(&directive);
        if has_inheritance_conflict(&parent, &child_kind, &child_pattern) {
            return emit_receipt(
                root,
                json!({
                    "ok": false,
                    "type": "directive_kernel_derive",
                    "lane": "core/layer0/ops",
                    "error": "inheritance_conflict",
                    "parent": parent,
                    "directive": clean(directive, 320),
                    "layer_map": ["0","1","2"],
                    "claim_evidence": [
                        {
                            "id": "V8-DIRECTIVES-001.2",
                            "claim": "derived_directives_require_parent_linkage_and_fail_on_inheritance_conflict",
                            "evidence": {"accepted": false, "reason": "inheritance_conflict"}
                        }
                    ]
                }),
            );
        }

        let parent_id = parent
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

