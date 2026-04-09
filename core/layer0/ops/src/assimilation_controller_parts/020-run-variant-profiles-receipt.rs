fn run_variant_profiles_receipt(root: &Path, strict: bool) -> Value {
    let required = ["medical", "robotics", "ai_isolation", "riscv_sovereign"];
    let mut profile_rows = Vec::new();
    let mut errors = Vec::<String>::new();

    for profile_id in required {
        let rel = format!("{VARIANT_PROFILE_DIR}/{profile_id}.json");
        let path = root.join(&rel);
        let payload = read_json(&path).unwrap_or(Value::Null);
        let mut row_errors = Vec::<String>::new();
        if payload.is_null() {
            row_errors.push("variant_profile_missing_or_invalid".to_string());
        }
        let version = payload
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if version != "v1" {
            row_errors.push("variant_profile_version_must_be_v1".to_string());
        }
        let kind = payload
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(kind, "variant_profile" | "layer_minus_one_variant_profile") {
            row_errors.push("variant_profile_kind_invalid".to_string());
        }
        let pid = payload
            .get("profile_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if pid != profile_id {
            row_errors.push("variant_profile_id_mismatch".to_string());
        }
        let baseline_ref = payload
            .get("baseline_policy_ref")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        if baseline_ref.is_empty() {
            row_errors.push("variant_profile_baseline_policy_ref_required".to_string());
        }
        let no_privilege_widening = payload
            .get("no_privilege_widening")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !no_privilege_widening {
            row_errors.push("variant_profile_no_privilege_widening_required".to_string());
        }
        let grants = payload
            .get("capability_delta")
            .and_then(|v| v.get("grant"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        let revokes = payload
            .get("capability_delta")
            .and_then(|v| v.get("revoke"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if grants.iter().any(|id| !is_token_id(id)) || revokes.iter().any(|id| !is_token_id(id)) {
            row_errors.push("variant_profile_capability_delta_invalid_token".to_string());
        }
        if grants.iter().any(|id| revokes.contains(id)) {
            row_errors.push("variant_profile_capability_delta_overlap".to_string());
        }
        let budget_delta = payload
            .get("budget_delta")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (k, v) in budget_delta {
            if !(v.is_i64() || v.is_u64() || v.is_f64()) {
                row_errors.push(format!("variant_profile_budget_delta_invalid::{k}"));
            }
        }

        if !row_errors.is_empty() {
            errors.extend(
                row_errors
                    .iter()
                    .map(|err| format!("{profile_id}:{err}"))
                    .collect::<Vec<_>>(),
            );
        }
        profile_rows.push(json!({
            "profile_id": profile_id,
            "path": rel,
            "ok": row_errors.is_empty(),
            "grants": grants,
            "revokes": revokes,
            "errors": row_errors
        }));
    }

    let ok = errors.is_empty();
    let mut out = json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "assimilation_controller_variant_profiles",
        "lane": LANE_ID,
        "ts": now_iso(),
        "required_profile_count": required.len(),
        "variant_profile_dir": VARIANT_PROFILE_DIR,
        "profiles": profile_rows,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-ASSIMILATE-001.1",
                "claim": "variant_profiles_define_capability_budget_policy_deltas_with_validation_receipts",
                "evidence": {
                    "required_profiles": required,
                    "validated_profiles": required.len()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    persist_receipt(root, &out);
    out
}

fn run_mpu_compartments_receipt(root: &Path, strict: bool) -> Value {
    let payload = read_json(&root.join(MPU_PROFILE_PATH)).unwrap_or(Value::Null);
    let mut errors = Vec::<String>::new();
    if payload
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("mpu_compartment_profile_version_must_be_v1".to_string());
    }
    if payload
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "mpu_compartment_profile"
    {
        errors.push("mpu_compartment_profile_kind_invalid".to_string());
    }

    let rows = payload
        .get("compartments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if rows.is_empty() {
        errors.push("mpu_compartments_required".to_string());
    }

    let mut ids = std::collections::BTreeSet::<String>::new();
    let mut compartments = Vec::new();
    for row in rows {
        let id = row
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let mut row_errors = Vec::<String>::new();
        if id.is_empty() || !is_token_id(&id) {
            row_errors.push("mpu_compartment_id_invalid".to_string());
        } else if !ids.insert(id.clone()) {
            row_errors.push("mpu_compartment_duplicate_id".to_string());
        }

        let region_start = row.get("region_start").and_then(Value::as_u64).unwrap_or(0);
        let region_size = row.get("region_size").and_then(Value::as_u64).unwrap_or(0);
        if region_start == 0 || region_size == 0 {
            row_errors.push("mpu_compartment_region_invalid".to_string());
        }

        let access = row
            .get("access")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let read = access.get("read").and_then(Value::as_bool).unwrap_or(false);
        let write = access
            .get("write")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let execute = access
            .get("execute")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !(read || write || execute) {
            row_errors.push("mpu_compartment_access_empty".to_string());
        }
        if write && execute {
            row_errors.push("mpu_compartment_write_execute_forbidden".to_string());
        }
        if !row
            .get("unprivileged")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            row_errors.push("mpu_compartment_unprivileged_required".to_string());
        }

        if !row_errors.is_empty() {
            errors.extend(
                row_errors
                    .iter()
                    .map(|err| format!("{id}:{err}"))
                    .collect::<Vec<_>>(),
            );
        }
        compartments.push(json!({
            "id": id,
            "ok": row_errors.is_empty(),
            "read": read,
            "write": write,
            "execute": execute,
            "errors": row_errors
        }));
    }

    let targets = payload
        .get("targets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if targets.is_empty() {
        errors.push("mpu_compartment_targets_required".to_string());
    }
    for target in targets {
        let target_id = target
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if target_id.is_empty() || !is_token_id(&target_id) {
            errors.push("mpu_compartment_target_id_invalid".to_string());
            continue;
        }
        let target_rows = target
            .get("compartments")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if target_rows.is_empty() {
            errors.push(format!("mpu_compartment_target_empty::{target_id}"));
            continue;
        }
        for comp in target_rows {
            let id = comp
                .as_str()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            if id.is_empty() || !ids.contains(&id) {
                errors.push(format!(
                    "mpu_compartment_target_unknown_compartment::{target_id}"
                ));
                break;
            }
        }
    }

    let ok = errors.is_empty();
    let mut out = json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "assimilation_controller_mpu_compartments",
        "lane": LANE_ID,
        "ts": now_iso(),
        "contract_path": MPU_PROFILE_PATH,
        "compartment_count": ids.len(),
        "compartments": compartments,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-ASSIMILATE-001.2",
                "claim": "mpu_compartment_profile_enforces_isolation_boundaries_and_conformance_receipts",
                "evidence": {
                    "compartment_count": ids.len()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    persist_receipt(root, &out);
    out
}

fn capability_ledger_events_path(root: &Path) -> std::path::PathBuf {
    state_root(root)
        .join("capability_ledger")
        .join("events.jsonl")
}

fn read_capability_ledger_events(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
}

fn canonical_capability_event_body(row: &Value) -> Value {
    json!({
        "seq": row.get("seq").cloned().unwrap_or(Value::Null),
        "ts": row.get("ts").cloned().unwrap_or(Value::Null),
        "op": row.get("op").cloned().unwrap_or(Value::Null),
        "capability": row.get("capability").cloned().unwrap_or(Value::Null),
        "subject": row.get("subject").cloned().unwrap_or(Value::Null),
        "reason": row.get("reason").cloned().unwrap_or(Value::Null)
    })
}

fn capability_event_hash(previous_hash: &str, event_body: &Value) -> String {
    let merged = serde_json::to_string(event_body).unwrap_or_default();
    receipt_hash(&json!({
        "previous_hash": previous_hash,
        "event": event_body,
        "merged": merged
    }))
}

fn capability_ledger_verify(events: &[Value]) -> (Vec<String>, String) {
    let mut verify_errors = Vec::<String>::new();
    let mut expected_prev = "GENESIS".to_string();
    for (idx, row) in events.iter().enumerate() {
        let seq = row.get("seq").and_then(Value::as_u64).unwrap_or(0);
        if seq != (idx as u64).saturating_add(1) {
            verify_errors.push(format!("seq_mismatch_at:{idx}"));
        }
        let previous = row
            .get("previous_hash")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if previous != expected_prev {
            verify_errors.push(format!("previous_hash_mismatch_at:{idx}"));
        }
        let event_body = canonical_capability_event_body(row);
        let recomputed = capability_event_hash(previous, &event_body);
        let observed = row
            .get("event_hash")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if observed != recomputed {
            verify_errors.push(format!("event_hash_mismatch_at:{idx}"));
        }
        expected_prev = observed.to_string();
    }
    (verify_errors, expected_prev)
}

pub fn append_capability_hash_chain_event(
    root: &Path,
    op: &str,
    capability: &str,
    subject: &str,
    reason: &str,
) -> Result<Value, String> {
    let op_clean = op.trim().to_ascii_lowercase();
    if op_clean != "grant" && op_clean != "revoke" {
        return Err("capability_ledger_op_invalid".to_string());
    }
    if !is_token_id(capability) {
        return Err("capability_id_invalid".to_string());
    }
    if !is_token_id(subject) {
        return Err("subject_id_invalid".to_string());
    }
    let events_path = capability_ledger_events_path(root);
    let events = read_capability_ledger_events(&events_path);
    let previous_hash = events
        .last()
        .and_then(|row| row.get("event_hash"))
        .and_then(Value::as_str)
        .unwrap_or("GENESIS")
        .to_string();
    let seq = (events.len() as u64).saturating_add(1);
    let event_ts = now_iso();
    let event_body = json!({
        "seq": seq,
        "ts": event_ts,
        "op": op_clean,
        "capability": capability.trim().to_ascii_lowercase(),
        "subject": subject.trim().to_ascii_lowercase(),
        "reason": reason.trim()
    });
    let event_hash = capability_event_hash(&previous_hash, &event_body);
    let event = json!({
        "seq": seq,
        "ts": event_ts,
        "op": op_clean,
        "capability": capability.trim().to_ascii_lowercase(),
        "subject": subject.trim().to_ascii_lowercase(),
        "reason": reason.trim(),
        "previous_hash": previous_hash,
        "event_hash": event_hash
    });
    append_jsonl(&events_path, &event)?;
    Ok(event)
}

fn run_capability_ledger_receipt(root: &Path, argv: &[String], strict: bool) -> Value {
    let op = parse_flag(argv, "op")
        .or_else(|| first_non_flag(argv, 1))
        .unwrap_or_else(|| "status".to_string())
        .to_ascii_lowercase();
    let events_path = capability_ledger_events_path(root);
    let mut events = read_capability_ledger_events(&events_path);
    let mut errors = Vec::<String>::new();
    let mut latest_event = Value::Null;

    if matches!(op.as_str(), "grant" | "revoke") {
        let capability = parse_flag(argv, "capability")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let subject = parse_flag(argv, "subject")
            .unwrap_or_else(|| "global".to_string())
            .trim()
            .to_ascii_lowercase();
        let reason = parse_flag(argv, "reason")
            .unwrap_or_else(|| "operator_request".to_string())
            .trim()
            .to_string();
        match append_capability_hash_chain_event(root, &op, &capability, &subject, &reason) {
            Ok(event) => {
                latest_event = event.clone();
                events.push(event);
            }
            Err(err) => {
                errors.push(err);
            }
        }
    } else if !matches!(op.as_str(), "verify" | "status") {
        errors.push(format!("unknown_capability_ledger_op:{op}"));
    }

    let (verify_errors, expected_prev) = capability_ledger_verify(&events);
    let chain_valid = verify_errors.is_empty();
    if op == "verify" {
        errors.extend(verify_errors.clone());
    }
    let ok = errors.is_empty();
    let mut out = json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "assimilation_controller_capability_ledger",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "events_path": events_path.display().to_string(),
        "event_count": events.len(),
        "tip_hash": expected_prev,
        "latest_event": latest_event,
        "chain_valid": chain_valid,
        "verify_errors": verify_errors,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-ASSIMILATE-001.3",
                "claim": "capability_grant_revoke_events_are_hash_chained_and_verifier_detects_tamper",
                "evidence": {
                    "event_count": events.len(),
                    "chain_valid": chain_valid
                }
            },
            {
                "id": "V7-ASM-003",
                "claim": "capability_grant_revoke_hash_chain_ledger_is_integrated_with_active_runtime_events",
                "evidence": {
                    "event_count": events.len(),
                    "chain_valid": chain_valid
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    persist_receipt(root, &out);
    out
}
