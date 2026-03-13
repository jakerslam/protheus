use super::*;

pub(super) fn command_status(root: &Path) -> i32 {
    let ledger = load_ledger(root);
    let provider_count = ledger
        .get("providers")
        .and_then(Value::as_object)
        .map(|m| m.len())
        .unwrap_or(0);
    emit(
        root,
        json!({
            "ok": true,
            "type": "intelligence_nexus_status",
            "lane": "core/layer0/ops",
            "provider_count": provider_count,
            "ledger": ledger,
            "latest": read_json(&latest_path(root))
        }),
    )
}

pub(super) fn command_open(root: &Path) -> i32 {
    emit(
        root,
        json!({
            "ok": true,
            "type": "intelligence_nexus_open",
            "lane": "core/layer0/ops",
            "workspace_route": "/workspace/keys",
            "dashboard_vital": "credit_health",
            "commands": ["protheus keys open", "protheus model buy credits"],
            "gates": {
                "conduit_required": true,
                "prime_directive_gate": true,
                "sovereign_identity_required": true
            }
        }),
    )
}

pub(super) fn command_add_key(root: &Path, parsed: &ParsedArgs) -> i32 {
    let provider = provider_name(parsed.flags.get("provider"));
    let gate_action = format!("keys:add:{provider}");
    if !directive_kernel::action_allowed(root, &gate_action) {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "intelligence_nexus_add_key",
                "lane": "core/layer0/ops",
                "provider": provider,
                "error": "directive_gate_denied",
                "action": gate_action
            }),
        );
    }

    let Some((raw_key, key_source)) = key_from_flags(parsed) else {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "intelligence_nexus_add_key",
                "lane": "core/layer0/ops",
                "provider": provider,
                "error": "missing_key_material"
            }),
        );
    };

    let validation = validate_key(&provider, &raw_key);
    let valid = validation
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !valid {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "intelligence_nexus_add_key",
                "lane": "core/layer0/ops",
                "provider": provider,
                "validation": validation,
                "error": "key_validation_failed"
            }),
        );
    }

    let fingerprint = key_fingerprint(&raw_key);
    let masked = key_masked(&raw_key);
    let sealed = seal_key(&raw_key, &provider, &fingerprint);
    let mut ledger = load_ledger(root);
    let key_event = append_key_event(
        &mut ledger,
        &provider,
        "add",
        json!({
            "fingerprint": fingerprint,
            "masked_key": masked,
            "key_source": clean(key_source.clone(), 64)
        }),
    );
    {
        let obj = ledger_obj_mut(&mut ledger);
        map_mut(obj, "providers").insert(
            provider.clone(),
            json!({
                "provider": provider,
                "fingerprint": fingerprint,
                "masked_key": masked,
                "sealed_key": sealed,
                "seal_algorithm": if vault_secret().is_some() { "xor_sha256_v1" } else { "none_descriptor_only" },
                "key_source": clean(key_source, 64),
                "validation": validation,
                "updated_at": now_iso()
            }),
        );
    }
    if let Err(err) = store_ledger(root, &ledger) {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "intelligence_nexus_add_key",
                "lane": "core/layer0/ops",
                "provider": provider,
                "error": clean(err, 200)
            }),
        );
    }

    emit(
        root,
        json!({
            "ok": true,
            "type": "intelligence_nexus_add_key",
            "lane": "core/layer0/ops",
            "provider": provider,
            "descriptor_only_client_persistence": true,
            "vault_encryption_enabled": vault_secret().is_some(),
            "raw_key_persisted": false,
            "key_event": key_event
        }),
    )
}

pub(super) fn command_rotate_key(root: &Path, parsed: &ParsedArgs) -> i32 {
    let provider = provider_name(parsed.flags.get("provider"));
    let gate_action = format!("keys:rotate:{provider}");
    if !directive_kernel::action_allowed(root, &gate_action) {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "intelligence_nexus_rotate_key",
                "lane": "core/layer0/ops",
                "provider": provider,
                "error": "directive_gate_denied",
                "action": gate_action
            }),
        );
    }

    let Some((raw_key, key_source)) = key_from_flags(parsed) else {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "intelligence_nexus_rotate_key",
                "lane": "core/layer0/ops",
                "provider": provider,
                "error": "missing_key_material"
            }),
        );
    };

    let validation = validate_key(&provider, &raw_key);
    if !validation
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "intelligence_nexus_rotate_key",
                "lane": "core/layer0/ops",
                "provider": provider,
                "validation": validation,
                "error": "key_validation_failed"
            }),
        );
    }

    let apply = parse_bool(parsed.flags.get("apply"), true);
    let allow_same = parse_bool(parsed.flags.get("allow-same"), false);
    let mut ledger = load_ledger(root);
    let existing = ledger
        .get("providers")
        .and_then(Value::as_object)
        .and_then(|m| m.get(&provider))
        .cloned();
    let Some(previous_record) = existing else {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "intelligence_nexus_rotate_key",
                "lane": "core/layer0/ops",
                "provider": provider,
                "error": "provider_not_found"
            }),
        );
    };

    let old_fingerprint = previous_record
        .get("fingerprint")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let old_masked = previous_record
        .get("masked_key")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let new_fingerprint = key_fingerprint(&raw_key);
    if !allow_same && !old_fingerprint.is_empty() && old_fingerprint == new_fingerprint {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "intelligence_nexus_rotate_key",
                "lane": "core/layer0/ops",
                "provider": provider,
                "error": "same_key_material",
                "allow_same": allow_same
            }),
        );
    }

    let masked = key_masked(&raw_key);
    let sealed = seal_key(&raw_key, &provider, &new_fingerprint);
    let key_event = if apply {
        let event = append_key_event(
            &mut ledger,
            &provider,
            "rotate",
            json!({
                "from_fingerprint": old_fingerprint,
                "to_fingerprint": new_fingerprint,
                "from_masked_key": old_masked,
                "to_masked_key": masked,
                "key_source": clean(key_source.clone(), 64)
            }),
        );
        {
            let obj = ledger_obj_mut(&mut ledger);
            map_mut(obj, "providers").insert(
                provider.clone(),
                json!({
                    "provider": provider,
                    "fingerprint": new_fingerprint,
                    "masked_key": masked,
                    "sealed_key": sealed,
                    "seal_algorithm": if vault_secret().is_some() { "xor_sha256_v1" } else { "none_descriptor_only" },
                    "key_source": clean(key_source, 64),
                    "validation": validation,
                    "updated_at": now_iso(),
                    "rotated_from": old_fingerprint,
                    "rotated_at": now_iso()
                }),
            );
        }
        event
    } else {
        Value::Null
    };

    if let Err(err) = store_ledger(root, &ledger) {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "intelligence_nexus_rotate_key",
                "lane": "core/layer0/ops",
                "provider": provider,
                "error": clean(err, 200)
            }),
        );
    }

    emit(
        root,
        json!({
            "ok": true,
            "type": "intelligence_nexus_rotate_key",
            "lane": "core/layer0/ops",
            "provider": provider,
            "apply": apply,
            "raw_key_persisted": false,
            "descriptor_only_client_persistence": true,
            "key_event": key_event
        }),
    )
}

pub(super) fn command_revoke_key(root: &Path, parsed: &ParsedArgs) -> i32 {
    let provider = provider_name(parsed.flags.get("provider"));
    let gate_action = format!("keys:revoke:{provider}");
    if !directive_kernel::action_allowed(root, &gate_action) {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "intelligence_nexus_revoke_key",
                "lane": "core/layer0/ops",
                "provider": provider,
                "error": "directive_gate_denied",
                "action": gate_action
            }),
        );
    }
    let apply = parse_bool(parsed.flags.get("apply"), true);
    let reason = clean(
        parsed
            .flags
            .get("reason")
            .cloned()
            .unwrap_or_else(|| "operator_revoke".to_string()),
        220,
    );

    let mut ledger = load_ledger(root);
    let existing = ledger
        .get("providers")
        .and_then(Value::as_object)
        .and_then(|m| m.get(&provider))
        .cloned();
    let Some(existing_record) = existing else {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "intelligence_nexus_revoke_key",
                "lane": "core/layer0/ops",
                "provider": provider,
                "error": "provider_not_found"
            }),
        );
    };

    let removed_fingerprint = existing_record
        .get("fingerprint")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let removed_masked_key = existing_record
        .get("masked_key")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let key_event = if apply {
        let event = append_key_event(
            &mut ledger,
            &provider,
            "revoke",
            json!({
                "reason": reason,
                "removed_fingerprint": removed_fingerprint,
                "removed_masked_key": removed_masked_key
            }),
        );
        {
            let obj = ledger_obj_mut(&mut ledger);
            map_mut(obj, "providers").remove(&provider);
            map_mut(obj, "credit_balances").remove(&provider);
            map_mut(obj, "credit_usage").remove(&provider);
            map_mut(obj, "spend_limits").remove(&provider);
        }
        event
    } else {
        Value::Null
    };

    if let Err(err) = store_ledger(root, &ledger) {
        return emit(
            root,
            json!({
                "ok": false,
                "type": "intelligence_nexus_revoke_key",
                "lane": "core/layer0/ops",
                "provider": provider,
                "error": clean(err, 200)
            }),
        );
    }

    emit(
        root,
        json!({
            "ok": true,
            "type": "intelligence_nexus_revoke_key",
            "lane": "core/layer0/ops",
            "provider": provider,
            "apply": apply,
            "key_event": key_event
        }),
    )
}
