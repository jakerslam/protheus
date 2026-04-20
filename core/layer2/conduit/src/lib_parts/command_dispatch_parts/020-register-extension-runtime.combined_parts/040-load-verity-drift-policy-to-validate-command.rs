
fn load_verity_drift_policy(root: &Path) -> VerityDriftRuntimePolicy {
    let config_path = resolve_verity_drift_config_path(root);
    let modified_ms = file_modified_ms(&config_path);
    let cache_key = format!(
        "{}::{}",
        config_path.to_string_lossy(),
        verity_drift_signing_key()
    );
    if let Ok(guard) = verity_drift_policy_cache().lock() {
        if let Some(entry) = guard.as_ref() {
            if entry.cache_key == cache_key && entry.modified_ms == modified_ms {
                return entry.policy.clone();
            }
        }
    }

    let policy = load_verity_drift_policy_uncached(config_path.clone());
    if let Ok(mut guard) = verity_drift_policy_cache().lock() {
        *guard = Some(VerityDriftPolicyCacheEntry {
            cache_key,
            modified_ms: file_modified_ms(&config_path),
            policy: policy.clone(),
        });
    }
    policy
}

fn drift_ms_against_now(ts_ms: u64) -> i64 {
    let now = now_ts_ms() as i128;
    let ts = ts_ms as i128;
    let drift = now - ts;
    drift.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

fn append_verity_drift_event(root: &Path, payload: &Value) {
    let path = resolve_verity_drift_events_path(root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let line = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut handle| {
            handle.write_all(line.as_bytes())?;
            handle.write_all(b"\n")
        });
}

fn activate_verity_judicial_lock(
    root: &Path,
    envelope: &CommandEnvelope,
    policy: &VerityDriftRuntimePolicy,
    validation: &ValidationReceipt,
) {
    let lock_path = resolve_verity_judicial_lock_path(root);
    if let Some(parent) = lock_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut payload = serde_json::json!({
        "active": true,
        "reason": "verity_timestamp_drift_exceeded",
        "mode": policy.mode.as_str(),
        "policy_version": policy.policy_version,
        "threshold_ms": policy.active_tolerance_ms,
        "production_tolerance_ms": policy.production_tolerance_ms,
        "simulation_tolerance_ms": policy.simulation_tolerance_ms,
        "timestamp_drift_ms": validation.timestamp_drift_ms,
        "request_id": envelope.request_id,
        "command_type": command_type_name(&envelope.command),
        "triggered_ts_ms": now_ts_ms(),
        "validation_receipt": validation,
    });
    payload["receipt_hash"] = Value::String(deterministic_receipt_hash(&payload));
    let _ = fs::write(
        lock_path,
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string()),
    );
}

fn log_verity_drift_violation(
    root: &Path,
    envelope: &CommandEnvelope,
    policy: &VerityDriftRuntimePolicy,
    validation: &ValidationReceipt,
) {
    let mut event = serde_json::json!({
        "ok": false,
        "type": "verity_drift_violation",
        "priority": "high",
        "severity": "critical",
        "mode": policy.mode.as_str(),
        "policy_version": policy.policy_version,
        "threshold_ms": policy.active_tolerance_ms,
        "production_tolerance_ms": policy.production_tolerance_ms,
        "simulation_tolerance_ms": policy.simulation_tolerance_ms,
        "signature_valid": policy.signature_valid,
        "config_path": policy.config_path.to_string_lossy().to_string(),
        "request_id": envelope.request_id,
        "command_type": command_type_name(&envelope.command),
        "ts_ms": now_ts_ms(),
        "validation_receipt": validation,
    });
    event["receipt_hash"] = Value::String(deterministic_receipt_hash(&event));
    append_verity_drift_event(root, &event);
}

pub fn validate_command<P: PolicyGate>(
    envelope: &CommandEnvelope,
    policy: &P,
    security: &mut ConduitSecurityContext,
) -> ValidationReceipt {
    let root = repo_root_from_current_dir();
    let drift_policy = load_verity_drift_policy(&root);
    let timestamp_drift_ms = drift_ms_against_now(envelope.ts_ms);
    let mode = drift_policy.mode.as_str();

    if envelope.schema_id != CONDUIT_SCHEMA_ID || envelope.schema_version != CONDUIT_SCHEMA_VERSION
    {
        return fail_closed_receipt(
            "conduit_schema_mismatch",
            "policy_not_evaluated",
            "security_not_evaluated",
            timestamp_drift_ms,
            mode,
        );
    }

    // VERITY PLANE: Drift protection - production mode is strict by default
    let drift_abs = (timestamp_drift_ms as i128).unsigned_abs();
    if drift_abs > drift_policy.active_tolerance_ms as u128 {
        let receipt = fail_closed_receipt(
            "timestamp_drift_exceeded",
            "policy_not_evaluated",
            "security_not_evaluated",
            timestamp_drift_ms,
            mode,
        );
        log_verity_drift_violation(&root, envelope, &drift_policy, &receipt);
        if drift_policy.is_production() {
            activate_verity_judicial_lock(&root, envelope, &drift_policy, &receipt);
        }
        return receipt;
    }

    let structural = validate_structure(&envelope.command);
    if let Some(reason) = structural {
        return fail_closed_receipt(
            reason,
            "policy_not_evaluated",
            "security_not_evaluated",
            timestamp_drift_ms,
            mode,
        );
    }

    let decision = policy.evaluate(&envelope.command);
    let policy_receipt_hash = deterministic_hash(&serde_json::json!({
        "allow": decision.allow,
        "reason": decision.reason,
        "command_type": command_type_name(&envelope.command)
    }));

    if !decision.allow {
        return fail_closed_receipt(
            decision.reason,
            policy_receipt_hash,
            "security_not_evaluated",
            timestamp_drift_ms,
            mode,
        );
    }

    let security_receipt_hash = match security.validate(envelope) {
        Ok(receipt_hash) => receipt_hash,
        Err(err) => {
            return fail_closed_receipt(
                err.to_string(),
                policy_receipt_hash,
                "security_denied",
                timestamp_drift_ms,
                mode,
            );
        }
    };

    success_receipt(
        policy_receipt_hash,
        security_receipt_hash,
        timestamp_drift_ms,
        mode,
    )
}
