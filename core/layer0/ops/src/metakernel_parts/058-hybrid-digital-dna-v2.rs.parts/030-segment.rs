fn run_dna_hybrid_worm_supersede(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    region_raw: Option<&String>,
    region_key_raw: Option<&String>,
    value_raw: Option<&String>,
) -> Value {
    let Some(instance_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_worm_supersede",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_raw, "instance", 96);
    let region = normalize_hybrid_region(region_raw);
    if !validate_worm_region(&region) {
        return json!({
            "ok": false,
            "type": "hybrid_dna_worm_supersede",
            "error": "worm_region_invalid",
            "region": region
        });
    }
    let value = clean(
        value_raw
            .map(String::as_str)
            .unwrap_or("worm-value")
            .to_string(),
        2048,
    );
    if value.trim().is_empty() {
        return json!({
            "ok": false,
            "type": "hybrid_dna_worm_supersede",
            "error": "worm_value_required"
        });
    }
    let region_key = normalize_token(
        region_key_raw
            .map(String::as_str)
            .unwrap_or(instance_id.as_str()),
        instance_id.as_str(),
        128,
    );
    let worm_key = format!("{region}:{region_key}");
    let value_hash = hash_json_value(&json!({ "value": value }));

    let mut hybrid_state = load_hybrid_dna_state(root);
    let previous_hash = hybrid_state
        .worm_regions
        .get(&worm_key)
        .map(|row| row.current_hash.clone());
    let payload = json!({
        "region": region,
        "region_key": region_key,
        "value_hash": value_hash,
        "previous_hash": previous_hash
    });
    let commit = build_commit_record(
        &instance_id,
        HYBRID_COMMIT_WORM_SUPERSESSION,
        &worm_key,
        hybrid_state.latest_commit_hash.clone(),
        &payload,
        None,
        None,
        true,
    );
    add_hybrid_commit(root, &commit);
    hybrid_state.latest_commit_hash = Some(commit.commit_hash.clone());

    let region_state = hybrid_state
        .worm_regions
        .entry(worm_key.clone())
        .or_insert_with(|| WormRegionState {
            region_type: region.clone(),
            region_key: region_key.clone(),
            current_hash: value_hash.clone(),
            version: 0,
            history: Vec::new(),
            failed_mutation_attempts: 0,
        });
    let version = region_state.version.saturating_add(1);
    let row = WormVersionRecord {
        version,
        value_hash: value_hash.clone(),
        previous_hash: previous_hash.clone(),
        supersession_commit_hash: commit.commit_hash.clone(),
        ts: now_iso(),
    };
    region_state.version = version;
    region_state.current_hash = value_hash;
    region_state.history.push(row);
    save_hybrid_dna_state(root, &hybrid_state);

    let receipt = write_hybrid_receipt(
        root,
        "worm_supersession",
        &instance_id,
        true,
        &payload,
        Some(&commit.commit_hash),
    );
    json!({
        "ok": true,
        "type": "hybrid_dna_worm_supersede",
        "instance_dna_ref": instance_id,
        "worm_region": worm_key,
        "version": version,
        "strict": strict,
        "commit": commit,
        "receipt": receipt
    })
}

fn run_dna_hybrid_worm_mutate_attempt(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    region_raw: Option<&String>,
    region_key_raw: Option<&String>,
) -> Value {
    let Some(instance_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_worm_mutate",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_raw, "instance", 96);
    let region = normalize_hybrid_region(region_raw);
    let region_key = normalize_token(
        region_key_raw
            .map(String::as_str)
            .unwrap_or(instance_id.as_str()),
        instance_id.as_str(),
        128,
    );
    let worm_key = format!("{region}:{region_key}");
    let mut hybrid_state = load_hybrid_dna_state(root);
    let failed_mutation_attempts = {
        let region_state = hybrid_state
            .worm_regions
            .entry(worm_key.clone())
            .or_insert_with(|| WormRegionState {
                region_type: region.clone(),
                region_key: region_key.clone(),
                current_hash: String::new(),
                version: 0,
                history: Vec::new(),
                failed_mutation_attempts: 0,
            });
        region_state.failed_mutation_attempts =
            region_state.failed_mutation_attempts.saturating_add(1);
        region_state.failed_mutation_attempts
    };
    let repeated = failed_mutation_attempts >= HYBRID_PROTECTED_REPAIR_FAILURE_LOCK_THRESHOLD;
    save_hybrid_dna_state(root, &hybrid_state);

    let invalid_lock = lock_on_hybrid_critical_event(
        root,
        strict,
        HybridCriticalEvent::InvalidWormMutationAttempt,
        &instance_id,
        json!({
            "worm_region": worm_key,
            "failed_mutation_attempts": failed_mutation_attempts
        }),
    );
    let repeated_lock = if repeated {
        lock_on_hybrid_critical_event(
            root,
            strict,
            HybridCriticalEvent::RepeatedFailedRepairOnProtectedStructure,
            &instance_id,
            json!({
                "worm_region": worm_key,
                "failed_mutation_attempts": failed_mutation_attempts
            }),
        )
    } else {
        false
    };
    let receipt = write_hybrid_receipt(
        root,
        "worm_mutation_attempt",
        &instance_id,
        false,
        &json!({
            "worm_region": worm_key,
            "failed_mutation_attempts": failed_mutation_attempts
        }),
        None,
    );
    json!({
        "ok": if strict { false } else { true },
        "type": "hybrid_dna_worm_mutate",
        "instance_dna_ref": instance_id,
        "error": "worm_region_mutation_forbidden_use_supersession",
        "worm_region": worm_key,
        "failed_mutation_attempts": failed_mutation_attempts,
        "judicial_lock": { "triggered": invalid_lock || repeated_lock },
        "receipt": receipt
    })
}

fn run_dna_hybrid_protected_lineage_check(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    expected_parent_raw: Option<&String>,
    action_raw: Option<&String>,
) -> Value {
    let Some(instance_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_protected_lineage",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_raw, "instance", 96);
    let action = normalize_token(
        action_raw.map(String::as_str).unwrap_or("invoke_agent"),
        "invoke_agent",
        96,
    );
    let check = evaluate_subservience(root, &instance_id, expected_parent_raw, &action, strict);
    let ok = check.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let lock_triggered = if strict && !ok {
        lock_on_hybrid_critical_event(
            root,
            strict,
            HybridCriticalEvent::FailedLineageCheckOnCriticalAction,
            &instance_id,
            json!({
                "action": action,
                "check": check
            }),
        )
    } else {
        false
    };
    let receipt = write_hybrid_receipt(
        root,
        "protected_lineage_check",
        &instance_id,
        ok,
        &json!({
            "action": action,
            "check": check
        }),
        None,
    );
    json!({
        "ok": if strict { ok } else { true },
        "type": "hybrid_dna_protected_lineage",
        "instance_dna_ref": instance_id,
        "check": check,
        "judicial_lock": { "triggered": lock_triggered },
        "receipt": receipt
    })
}
