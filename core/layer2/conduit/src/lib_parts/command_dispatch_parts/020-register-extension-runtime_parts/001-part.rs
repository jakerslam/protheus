struct VerityDriftRuntimePolicy {
    mode: String,
    active_tolerance_ms: i64,
    production_tolerance_ms: i64,
    simulation_tolerance_ms: i64,
    policy_version: u32,
    config_path: PathBuf,
    signature_valid: bool,
}

impl VerityDriftRuntimePolicy {
    fn is_production(&self) -> bool {
        self.mode == VERITY_DRIFT_MODE_PRODUCTION
    }
}

#[derive(Debug, Clone)]
struct VerityDriftPolicyCacheEntry {
    cache_key: String,
    modified_ms: Option<u64>,
    policy: VerityDriftRuntimePolicy,
}

fn resolve_verity_path(root: &Path, env_key: &str, fallback_rel: &str) -> PathBuf {
    let explicit = std::env::var(env_key)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty());
    if let Some(raw) = explicit {
        let candidate = PathBuf::from(raw);
        if candidate.is_absolute() {
            return candidate;
        }
        return root.join(candidate);
    }
    root.join(fallback_rel)
}

fn resolve_verity_drift_config_path(root: &Path) -> PathBuf {
    resolve_verity_path(
        root,
        "INFRING_VERITY_DRIFT_CONFIG_PATH",
        "local/state/ops/verity/drift_policy.signed.json",
    )
}

fn resolve_verity_drift_events_path(root: &Path) -> PathBuf {
    resolve_verity_path(
        root,
        "INFRING_VERITY_DRIFT_EVENTS_PATH",
        "local/state/ops/verity/drift_events.jsonl",
    )
}

fn resolve_verity_judicial_lock_path(root: &Path) -> PathBuf {
    resolve_verity_path(
        root,
        "INFRING_VERITY_JUDICIAL_LOCK_PATH",
        "local/state/ops/verity/judicial_lock.json",
    )
}

fn verity_drift_signing_key() -> String {
    std::env::var("INFRING_VERITY_DRIFT_SIGNING_KEY")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| "infring-verity-drift-local-key".to_string())
}

fn normalize_verity_mode(raw: &str) -> String {
    let lowered = raw.trim().to_ascii_lowercase();
    if lowered == VERITY_DRIFT_MODE_SIMULATION || lowered == "sim" {
        VERITY_DRIFT_MODE_SIMULATION.to_string()
    } else {
        VERITY_DRIFT_MODE_PRODUCTION.to_string()
    }
}

fn clamp_verity_tolerance_ms(raw: i64, floor: i64, ceil: i64) -> i64 {
    raw.clamp(floor, ceil)
}

fn verity_signature_payload(config: &VerityDriftSignedConfig) -> Value {
    serde_json::json!({
        "schema_id": config.schema_id,
        "schema_version": config.schema_version,
        "policy_version": config.policy_version,
        "mode": config.mode,
        "production_tolerance_ms": config.production_tolerance_ms,
        "simulation_tolerance_ms": config.simulation_tolerance_ms
    })
}

fn sign_verity_config_payload(payload: &Value) -> String {
    let key = verity_drift_signing_key();
    let digest = deterministic_receipt_hash(&serde_json::json!({
        "payload": payload,
        "signing_key": key
    }));
    format!("sig:{digest}")
}

fn signed_default_verity_config() -> VerityDriftSignedConfig {
    let mut config = VerityDriftSignedConfig {
        schema_id: VERITY_DRIFT_CONFIG_SCHEMA_ID.to_string(),
        schema_version: VERITY_DRIFT_CONFIG_SCHEMA_VERSION,
        policy_version: VERITY_DRIFT_CONFIG_POLICY_VERSION,
        mode: VERITY_DRIFT_MODE_PRODUCTION.to_string(),
        production_tolerance_ms: VERITY_DRIFT_PRODUCTION_DEFAULT_MS,
        simulation_tolerance_ms: VERITY_DRIFT_SIMULATION_DEFAULT_MS,
        signature: String::new(),
    };
    config.signature = sign_verity_config_payload(&verity_signature_payload(&config));
    config
}

fn verity_drift_policy_cache() -> &'static std::sync::Mutex<Option<VerityDriftPolicyCacheEntry>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<Option<VerityDriftPolicyCacheEntry>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(None))
}

fn file_modified_ms(path: &Path) -> Option<u64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    modified
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|value| value.as_millis() as u64)
}

fn write_verity_signed_config(path: &Path, config: &VerityDriftSignedConfig) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let body = serde_json::to_string_pretty(config).unwrap_or_else(|_| "{}".to_string());
    let _ = fs::write(path, body);
}

fn runtime_policy_from_signed(
    config_path: PathBuf,
    mut signed: VerityDriftSignedConfig,
    signature_valid: bool,
) -> VerityDriftRuntimePolicy {
    signed.mode = normalize_verity_mode(&signed.mode);
    signed.production_tolerance_ms =
        clamp_verity_tolerance_ms(signed.production_tolerance_ms, 1, 60_000);
    signed.simulation_tolerance_ms = clamp_verity_tolerance_ms(
        signed.simulation_tolerance_ms,
        signed.production_tolerance_ms,
        300_000,
    );
    let active = if signed.mode == VERITY_DRIFT_MODE_SIMULATION {
        signed.simulation_tolerance_ms
    } else {
        signed.production_tolerance_ms
    };
    VerityDriftRuntimePolicy {
        mode: signed.mode,
        active_tolerance_ms: active,
        production_tolerance_ms: signed.production_tolerance_ms,
        simulation_tolerance_ms: signed.simulation_tolerance_ms,
        policy_version: signed.policy_version.max(1),
        config_path,
        signature_valid,
    }
}

fn load_verity_drift_policy_uncached(config_path: PathBuf) -> VerityDriftRuntimePolicy {
    let default_signed = signed_default_verity_config();
    let raw = match fs::read_to_string(&config_path) {
        Ok(value) => value,
        Err(_) => {
            write_verity_signed_config(&config_path, &default_signed);
            return runtime_policy_from_signed(config_path, default_signed, true);
        }
    };

    let parsed = serde_json::from_str::<VerityDriftSignedConfig>(&raw);
    let mut signed = match parsed {
        Ok(value) => value,
        Err(_) => {
            write_verity_signed_config(&config_path, &default_signed);
            return runtime_policy_from_signed(config_path, default_signed, false);
        }
    };

    let payload = verity_signature_payload(&signed);
    let expected = sign_verity_config_payload(&payload);
    let signature_valid = signed.signature.trim() == expected;
    if !signature_valid {
        write_verity_signed_config(&config_path, &default_signed);
        return runtime_policy_from_signed(config_path, default_signed, false);
    }

    if signed.schema_id != VERITY_DRIFT_CONFIG_SCHEMA_ID
        || signed.schema_version != VERITY_DRIFT_CONFIG_SCHEMA_VERSION
    {
        signed.schema_id = VERITY_DRIFT_CONFIG_SCHEMA_ID.to_string();
        signed.schema_version = VERITY_DRIFT_CONFIG_SCHEMA_VERSION;
        signed.signature = sign_verity_config_payload(&verity_signature_payload(&signed));
        write_verity_signed_config(&config_path, &signed);
    }

    runtime_policy_from_signed(config_path, signed, signature_valid)
}

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

fn validate_structure(command: &TsCommand) -> Option<String> {
    match command {
        TsCommand::StartAgent { agent_id } | TsCommand::StopAgent { agent_id } => {
            if agent_id.trim().is_empty() {
                return Some("agent_id_required".to_string());
            }
        }
        TsCommand::QueryReceiptChain { limit, .. } => {
            if let Some(value) = limit {
                if *value == 0 || *value > 1000 {
                    return Some("receipt_query_limit_out_of_range".to_string());
                }
            }
        }
        TsCommand::ApplyPolicyUpdate { patch_id, .. } => {
            if patch_id.trim().is_empty() {
                return Some("policy_patch_id_required".to_string());
            }
