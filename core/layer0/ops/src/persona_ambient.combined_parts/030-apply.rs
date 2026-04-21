
fn apply(root: &Path, flags: &BTreeMap<String, String>) -> i32 {
    let policy = load_policy(root);
    if !policy.enabled || !policy.ambient_stance {
        let receipt = fail_receipt(&policy, "apply", "ambient_persona_stance_disabled", None);
        emit(&receipt);
        return 2;
    }
    if !policy.auto_apply {
        let receipt = fail_receipt(&policy, "apply", "auto_apply_disabled", None);
        emit(&receipt);
        return 2;
    }

    let persona = sanitize_persona_id(flags.get("persona").map(String::as_str));
    if persona.is_empty() {
        let receipt = fail_receipt(&policy, "apply", "persona_missing_or_invalid", None);
        emit(&receipt);
        return 2;
    }

    let stance = match parse_stance(flags) {
        Ok(v) => v,
        Err(reason) => {
            let receipt = fail_receipt(&policy, "apply", &reason, None);
            emit(&receipt);
            return 2;
        }
    };
    let Value::Object(patch_map) = stance else {
        let receipt = fail_receipt(&policy, "apply", "stance_patch_must_be_object", None);
        emit(&receipt);
        return 2;
    };

    let full_reload_requested = parse_bool(flags.get("full-reload").map(String::as_str), false);
    if full_reload_requested && !policy.full_reload {
        let receipt = fail_receipt(
            &policy,
            "apply",
            "full_reload_disabled_in_ambient_mode",
            None,
        );
        emit(&receipt);
        return 2;
    }

    let patch_bytes = serde_json::to_string(&Value::Object(patch_map.clone()))
        .map(|v| v.len())
        .unwrap_or(0);
    if patch_bytes > policy.max_patch_bytes {
        let receipt = fail_receipt(
            &policy,
            "apply",
            "stance_patch_exceeds_budget",
            Some(json!({
                "patch_bytes": patch_bytes,
                "max_patch_bytes": policy.max_patch_bytes
            })),
        );
        emit(&receipt);
        return 2;
    }

    let source = clean_text(flags.get("source").map(String::as_str), 80);
    let reason = clean_text(flags.get("reason").map(String::as_str), 180);
    let run_context = clean_text(flags.get("run-context").map(String::as_str), 40);
    let run_context = if run_context.is_empty() {
        "persona_ambient".to_string()
    } else {
        run_context
    };

    let patch_hash = deterministic_receipt_hash(&json!({
        "persona": persona,
        "patch": patch_map
    }));

    let queue_receipt = if policy.push_attention_queue {
        match enqueue_attention(&persona, &patch_hash, &run_context) {
            Ok(v) => v,
            Err(reason) => {
                let receipt = fail_receipt(&policy, "apply", &reason, None);
                emit(&receipt);
                return 2;
            }
        }
    } else {
        json!({
            "ok": true,
            "type": "attention_queue_enqueue",
            "decision": "disabled",
            "queued": false
        })
    };

    let queue_decision = queue_receipt
        .get("decision")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let queue_allowed = matches!(queue_decision.as_str(), "admitted" | "deduped" | "disabled");
    if !queue_allowed {
        let receipt = fail_receipt(
            &policy,
            "apply",
            "attention_queue_blocked_stance_apply",
            Some(json!({
                "queue_decision": queue_decision,
                "attention_receipt": queue_receipt
            })),
        );
        emit(&receipt);
        return 2;
    }

    let mut cache = load_cache(&policy.cache_path);
    let personas_value = cache
        .get_mut("personas")
        .expect("personas object missing after load");
    let personas_map = as_object_mut(personas_value);

    let is_new_persona = !personas_map.contains_key(&persona);
    if is_new_persona && personas_map.len() >= policy.max_personas {
        let receipt = fail_receipt(
            &policy,
            "apply",
            "persona_cache_capacity_exceeded",
            Some(json!({
                "max_personas": policy.max_personas
            })),
        );
        emit(&receipt);
        return 2;
    }

    let previous_entry = personas_map
        .get(&persona)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let previous_stance = previous_entry
        .get("stance")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let previous_revision = previous_entry
        .get("revision")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let (next_stance, changed_keys, removed_keys) =
        stance_diff(&previous_stance, &patch_map, full_reload_requested);
    let delta_applied = !changed_keys.is_empty() || !removed_keys.is_empty() || is_new_persona;
    let next_revision = if delta_applied {
        previous_revision + 1
    } else {
        previous_revision
    };

    let ts = now_iso();
    personas_map.insert(
        persona.clone(),
        json!({
            "persona": persona,
            "stance": next_stance,
            "revision": next_revision,
            "last_applied_at": ts,
            "last_source": if source.is_empty() { Value::Null } else { Value::String(source.clone()) },
            "last_reason": if reason.is_empty() { Value::Null } else { Value::String(reason.clone()) },
            "last_attention_decision": queue_decision,
            "last_patch_hash": patch_hash
        }),
    );

    cache["ts"] = Value::String(ts.clone());
    cache["ambient_mode_active"] = Value::Bool(policy.enabled && policy.ambient_stance);
    cache["authoritative_lane"] = Value::String("rust_persona_ambient".to_string());

    write_json(&policy.cache_path, &cache);

    let mut receipt = json!({
        "ok": true,
        "type": "persona_ambient_apply",
        "ts": ts,
        "ambient_mode_active": policy.enabled && policy.ambient_stance,
        "authoritative_lane": "rust_persona_ambient",
        "run_context": run_context,
        "persona": persona,
        "incremental_apply": true,
        "full_reload_requested": full_reload_requested,
        "full_reload_allowed": policy.full_reload,
        "delta_applied": delta_applied,
        "delta": {
            "changed_keys": changed_keys,
            "removed_keys": removed_keys
        },
        "revision": next_revision,
        "stance_key_count": cache
            .pointer(&format!("/personas/{}/stance", sanitize_json_pointer_key(&persona)))
            .and_then(Value::as_object)
            .map(|obj| obj.len())
            .unwrap_or(0),
        "patch_bytes": patch_bytes,
        "attention_queue": queue_receipt,
        "policy": policy_snapshot(&policy)
    });
    persist_and_emit(&policy.latest_path, &policy.receipts_path, &mut receipt);
    0
}

fn sanitize_json_pointer_key(raw: &str) -> String {
    raw.replace('~', "~0").replace('/', "~1")
}
