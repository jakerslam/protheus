    if command == "migrate" {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let allow_unsigned = parse_bool(parsed.flags.get("allow-unsigned"), false);
        let repair_signatures = parse_bool(parsed.flags.get("repair-signatures"), false);
        if apply && !allow_unsigned && !signing_key_present() {
            return emit_receipt(
                root,
                json!({
                    "ok": false,
                    "type": "directive_kernel_migrate",
                    "lane": "core/layer0/ops",
                    "error": "missing_signing_key",
                    "signing_env": SIGNING_ENV,
                    "layer_map": ["0","1","2","client","app"],
                    "claim_evidence": [
                        {
                            "id": "V8-DIRECTIVES-001.5",
                            "claim": "directive_migration_and_status_are_available_as_one_command_core_paths",
                            "evidence": {"apply": apply, "ok": false, "reason": "missing_signing_key"}
                        }
                    ]
                }),
            );
        }
        let migrated = migrate_legacy_markdown(root, apply).unwrap_or_else(|err| {
            json!({
                "error": clean(err, 220),
                "harvested_count": 0,
                "imported_count": 0
            })
        });
        let signature_repair = if repair_signatures {
            Some(
                repair_vault_signatures(root, apply, allow_unsigned).unwrap_or_else(|err| {
                    json!({
                        "error": clean(err, 220),
                        "apply": apply
                    })
                }),
            )
        } else {
            None
        };
        let migration_ok = !migrated.get("error").is_some();
        let repair_ok = signature_repair
            .as_ref()
            .map(|v| !v.get("error").is_some())
            .unwrap_or(true);
        let ok = migration_ok && repair_ok;
        return emit_receipt(
            root,
            json!({
                "ok": ok,
                "type": "directive_kernel_migrate",
                "lane": "core/layer0/ops",
                "apply": apply,
                "migration": migrated,
                "signature_repair": signature_repair,
                "commands": ["protheus directives migrate", "protheus directives status", "protheus prime sign", "protheus directives supersede"],
                "policy_hash": directive_vault_hash(root),
                "layer_map": ["0","1","2","client","app"],
                "claim_evidence": [
                    {
                        "id": "V8-DIRECTIVES-001.5",
                        "claim": "directive_migration_and_status_are_available_as_one_command_core_paths",
                        "evidence": {
                            "apply": apply,
                            "ok": ok,
                            "repair_signatures": repair_signatures
                        }
                    }
                ]
            }),
        );
    }

    emit_receipt(
        root,
        json!({
            "ok": false,
            "type": "directive_kernel_error",
            "lane": "core/layer0/ops",
            "error": "unknown_command",
            "command": command,
            "exit_code": 2
        }),
    )
}
