const VERITY_DRIFT_SIMULATION_DEFAULT_MS: i64 = 30_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerityDriftSignedConfig {
    schema_id: String,
    schema_version: u32,
    policy_version: u32,
    mode: String,
    production_tolerance_ms: i64,
    simulation_tolerance_ms: i64,
    signature: String,
}

#[derive(Debug, Clone)]
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
