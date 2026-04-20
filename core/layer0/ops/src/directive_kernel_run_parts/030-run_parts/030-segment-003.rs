        let mut entry = match append_directive_entry(
            root,
            "derived",
            &directive,
            &signer,
            Some(&parent_id),
            None,
            "derived_engine",
        ) {
            Ok(v) => v,
            Err(err) => {
                return emit_receipt(
                    root,
                    json!({
                        "ok": false,
                        "type": "directive_kernel_derive",
                        "lane": "core/layer0/ops",
                        "error": clean(&err, 240),
                        "claim_evidence": [
                            {
                                "id": "V8-DIRECTIVES-001.2",
                                "claim": "derived_directives_require_parent_linkage_and_fail_on_inheritance_conflict",
                                "evidence": {"accepted": false, "reason": clean(&err, 240)}
                            }
                        ]
                    }),
                );
            }
        };
        entry["accepted"] = Value::Bool(true);

        let mut vault2 = load_vault(root);
        let obj = vault_obj_mut(&mut vault2);
        let rows = ensure_array(obj, "derived");
        if let Some(last) = rows.last_mut() {
            *last = entry.clone();
        }
        let _ = write_vault(root, &vault2);

        return emit_receipt(
            root,
            json!({
                "ok": true,
                "type": "directive_kernel_derive",
                "lane": "core/layer0/ops",
                "entry": entry,
                "parent": parent,
                "policy_hash": directive_vault_hash(root),
                "layer_map": ["0","1","2"],
                "claim_evidence": [
                    {
                        "id": "V8-DIRECTIVES-001.2",
                        "claim": "derived_directives_require_parent_linkage_and_fail_on_inheritance_conflict",
                        "evidence": {"accepted": true, "parent_id": parent_id}
                    }
                ]
            }),
        );
    }

    if command == "supersede" {
        let target_hint = parsed.flags.get("target").cloned().unwrap_or_default();
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
                    "type": "directive_kernel_supersede",
                    "lane": "core/layer0/ops",
                    "error": "missing_signing_key",
                    "signing_env": SIGNING_ENV,
                    "claim_evidence": [
                        {
                            "id": "V8-DIRECTIVES-001.1",
                            "claim": "prime_directives_are_append_only_signed_objects_with_supersession_not_inline_edits",
                            "evidence": {"accepted": false, "reason": "missing_signing_key"}
                        }
                    ]
                }),
            );
        }
        let vault = load_vault(root);
        let Some(target) = resolve_parent(&vault, &target_hint) else {
            return emit_receipt(
                root,
                json!({
                    "ok": false,
                    "type": "directive_kernel_supersede",
                    "lane": "core/layer0/ops",
                    "error": "target_not_found",
                    "target": clean(target_hint, 320)
                }),
            );
        };
        let target_id = target
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if target_id.is_empty() {
            return emit_receipt(
                root,
                json!({
                    "ok": false,
                    "type": "directive_kernel_supersede",
                    "lane": "core/layer0/ops",
                    "error": "target_id_missing"
                }),
            );
        }
        let entry = match append_directive_entry(
            root,
            "derived",
            &directive,
            &signer,
            Some(&target_id),
            Some(&target_id),
            "supersession",
        ) {
            Ok(v) => v,
            Err(err) => {
                return emit_receipt(
                    root,
                    json!({
                        "ok": false,
                        "type": "directive_kernel_supersede",
                        "lane": "core/layer0/ops",
                        "error": clean(&err, 240)
                    }),
                );
            }
        };

        return emit_receipt(
            root,
            json!({
                "ok": true,
                "type": "directive_kernel_supersede",
                "lane": "core/layer0/ops",
                "target": target,
                "entry": entry,
                "policy_hash": directive_vault_hash(root),
                "layer_map": ["0","1","2"],
                "claim_evidence": [
                    {
                        "id": "V8-DIRECTIVES-001.1",
                        "claim": "prime_directives_are_append_only_signed_objects_with_supersession_not_inline_edits",
                        "evidence": {
                            "target_id": target_id,
                            "superseding_entry_id": entry.get("id").cloned().unwrap_or(Value::Null)
                        }
                    }
                ]
            }),
        );
    }

