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

fn verity_plane_config_path(root: &Path) -> PathBuf {
    resolve_verity_path(
        root,
        "INFRING_VERITY_PLANE_CONFIG_PATH",
        "local/state/ops/verity/plane_policy.signed.json",
    )
}

fn verity_receipts_path(root: &Path) -> PathBuf {
    resolve_verity_path(
        root,
        "INFRING_VERITY_RECEIPTS_PATH",
        "local/state/ops/verity/receipts.jsonl",
    )
}

fn verity_events_path(root: &Path) -> PathBuf {
    resolve_verity_path(
        root,
        "INFRING_VERITY_EVENTS_PATH",
        "local/state/ops/verity/events.jsonl",
    )
}

fn verity_latest_path(root: &Path) -> PathBuf {
    resolve_verity_path(
        root,
        "INFRING_VERITY_LATEST_PATH",
        "local/state/ops/verity/latest.json",
    )
}

fn verity_vector_state_path(root: &Path) -> PathBuf {
    resolve_verity_path(
        root,
        "INFRING_VERITY_VECTOR_STATE_PATH",
        "local/state/ops/verity/vector_state.json",
    )
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

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let exact = format!("--{key}");
    let prefix = format!("--{key}=");
    for (idx, token) in argv.iter().enumerate() {
        let trimmed = token.trim();
        if let Some(value) = trimmed.strip_prefix(&prefix) {
            let out = value.trim().to_string();
            if !out.is_empty() {
                return Some(out);
            }
        }
        if trimmed == exact {
            if let Some(next) = argv.get(idx + 1) {
                let out = next.trim().to_string();
                if !out.is_empty() {
                    return Some(out);
                }
            }
        }
    }
    None
}

fn parse_usize(raw: Option<String>, fallback: usize, min: usize, max: usize) -> usize {
    raw.and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn parse_f64(raw: Option<String>, fallback: f64, min: f64, max: f64) -> f64 {
    raw.and_then(|value| value.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn parse_json(raw: &str) -> Option<Value> {
    serde_json::from_str::<Value>(raw).ok()
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn normalize_mode(raw: &str) -> String {
    let lowered = raw.trim().to_ascii_lowercase();
    if lowered == VERITY_MODE_SIMULATION || lowered == "sim" {
        VERITY_MODE_SIMULATION.to_string()
    } else {
        // VERITY PLANE: Drift protection - production mode is strict by default
        VERITY_MODE_PRODUCTION.to_string()
    }
}

fn verity_plane_signing_key() -> String {
    std::env::var("INFRING_VERITY_PLANE_SIGNING_KEY")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| "infring-verity-plane-local-key".to_string())
}

fn verity_drift_signing_key() -> String {
    std::env::var("INFRING_VERITY_DRIFT_SIGNING_KEY")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| "infring-verity-drift-local-key".to_string())
}

fn sign_payload(payload: &Value, signing_key: &str) -> String {
    let digest = deterministic_receipt_hash(&json!({
        "payload": payload,
        "signing_key": signing_key
    }));
    format!("sig:{digest}")
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn normalize_verity_plane_config_fields(cfg: &mut VerityPlaneSignedConfig) {
    cfg.mode = normalize_mode(&cfg.mode);
    cfg.fidelity_warning_threshold = clamp01(cfg.fidelity_warning_threshold);
    cfg.fidelity_lock_threshold = clamp01(cfg.fidelity_lock_threshold);
    cfg.vector_warning_threshold = clamp01(cfg.vector_warning_threshold);
    if cfg.fidelity_lock_threshold > cfg.fidelity_warning_threshold {
        cfg.fidelity_lock_threshold = cfg.fidelity_warning_threshold;
    }
    cfg.policy_version = cfg.policy_version.max(1);
    cfg.schema_id = VERITY_PLANE_SCHEMA_ID.to_string();
    cfg.schema_version = VERITY_PLANE_SCHEMA_VERSION;
}

fn verity_plane_signature_payload(cfg: &VerityPlaneSignedConfig) -> Value {
    json!({
        "schema_id": cfg.schema_id,
        "schema_version": cfg.schema_version,
        "policy_version": cfg.policy_version,
        "mode": cfg.mode,
        "fidelity_warning_threshold": cfg.fidelity_warning_threshold,
        "fidelity_lock_threshold": cfg.fidelity_lock_threshold,
        "vector_warning_threshold": cfg.vector_warning_threshold,
    })
}

fn signed_default_verity_plane_config() -> VerityPlaneSignedConfig {
    let mut cfg = VerityPlaneSignedConfig {
        schema_id: VERITY_PLANE_SCHEMA_ID.to_string(),
        schema_version: VERITY_PLANE_SCHEMA_VERSION,
        policy_version: VERITY_PLANE_POLICY_VERSION,
        mode: VERITY_MODE_PRODUCTION.to_string(),
        fidelity_warning_threshold: VERITY_FIDELITY_WARNING_DEFAULT,
        fidelity_lock_threshold: VERITY_FIDELITY_LOCK_DEFAULT,
        vector_warning_threshold: VERITY_VECTOR_WARNING_DEFAULT,
        signature: String::new(),
    };
    cfg.signature = sign_payload(
        &verity_plane_signature_payload(&cfg),
        &verity_plane_signing_key(),
    );
    cfg
}

fn signed_default_drift_config() -> VerityDriftSignedConfig {
    let mut cfg = VerityDriftSignedConfig {
        schema_id: VERITY_DRIFT_CONFIG_SCHEMA_ID.to_string(),
        schema_version: VERITY_DRIFT_CONFIG_SCHEMA_VERSION,
        policy_version: VERITY_DRIFT_CONFIG_POLICY_VERSION,
        mode: VERITY_MODE_PRODUCTION.to_string(),
        production_tolerance_ms: VERITY_DRIFT_PRODUCTION_DEFAULT_MS,
        simulation_tolerance_ms: VERITY_DRIFT_SIMULATION_DEFAULT_MS,
        signature: String::new(),
    };
    cfg.signature = sign_payload(
        &json!({
            "schema_id": cfg.schema_id,
            "schema_version": cfg.schema_version,
            "policy_version": cfg.policy_version,
            "mode": cfg.mode,
            "production_tolerance_ms": cfg.production_tolerance_ms,
            "simulation_tolerance_ms": cfg.simulation_tolerance_ms
        }),
        &verity_drift_signing_key(),
    );
    cfg
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn write_json(path: &Path, payload: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(encoded) = serde_json::to_string_pretty(payload) {
        let _ = fs::write(path, encoded);
    }
}

fn append_jsonl(path: &Path, payload: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let encoded = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
        let _ = writeln!(file, "{encoded}");
    }
}

fn load_recent_jsonl(path: &Path, limit: usize) -> Vec<Value> {
    let raw = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let mut rows = raw
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>();
    if rows.len() > limit {
        rows = rows.split_off(rows.len() - limit);
    }
    rows
}

fn load_verity_plane_config(root: &Path) -> (VerityPlaneSignedConfig, bool, PathBuf) {
    let path = verity_plane_config_path(root);
    let default_cfg = signed_default_verity_plane_config();
    let mut cfg = match fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<VerityPlaneSignedConfig>(&raw).ok())
    {
        Some(value) => value,
        None => {
            write_json(
                &path,
                &serde_json::to_value(&default_cfg).unwrap_or_else(|_| json!({})),
            );
            return (default_cfg, true, path);
        }
    };
    normalize_verity_plane_config_fields(&mut cfg);
    let expected = sign_payload(
        &verity_plane_signature_payload(&cfg),
        &verity_plane_signing_key(),
    );
    if cfg.signature.trim() != expected {
        write_json(
            &path,
            &serde_json::to_value(&default_cfg).unwrap_or_else(|_| json!({})),
        );
        return (default_cfg, false, path);
    }
    cfg.signature = expected;
    write_json(
        &path,
        &serde_json::to_value(&cfg).unwrap_or_else(|_| json!({})),
    );
    (cfg, true, path)
}

fn load_verity_drift_snapshot(root: &Path, limit: usize) -> Value {
    let config_path = resolve_verity_drift_config_path(root);
    let events_path = resolve_verity_drift_events_path(root);
    let lock_path = resolve_verity_judicial_lock_path(root);
    let default_cfg = signed_default_drift_config();
    let cfg = fs::read_to_string(&config_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<VerityDriftSignedConfig>(&raw).ok())
        .unwrap_or_else(|| {
            write_json(
                &config_path,
                &serde_json::to_value(&default_cfg).unwrap_or_else(|_| json!({})),
            );
            default_cfg
        });
    let normalized_mode = normalize_mode(&cfg.mode);
    let expected_signature = sign_payload(
        &json!({
            "schema_id": cfg.schema_id,
            "schema_version": cfg.schema_version,
            "policy_version": cfg.policy_version,
            "mode": normalized_mode,
            "production_tolerance_ms": cfg.production_tolerance_ms.clamp(1, 60_000),
            "simulation_tolerance_ms": cfg.simulation_tolerance_ms.clamp(cfg.production_tolerance_ms.clamp(1, 60_000), 300_000)
        }),
        &verity_drift_signing_key(),
    );
    let signature_valid = cfg.signature.trim() == expected_signature;
    let production_tolerance_ms = cfg.production_tolerance_ms.clamp(1, 60_000);
    let simulation_tolerance_ms = cfg
        .simulation_tolerance_ms
        .clamp(production_tolerance_ms, 300_000);
    let active_tolerance_ms = if normalized_mode == VERITY_MODE_SIMULATION {
        simulation_tolerance_ms
    } else {
        production_tolerance_ms
    };
    json!({
        "mode": normalized_mode,
        "active_tolerance_ms": active_tolerance_ms,
        "production_tolerance_ms": production_tolerance_ms,
        "simulation_tolerance_ms": simulation_tolerance_ms,
        "config_path": config_path.to_string_lossy().to_string(),
        "events_path": events_path.to_string_lossy().to_string(),
        "judicial_lock_path": lock_path.to_string_lossy().to_string(),
        "judicial_lock": read_json(&lock_path),
        "signature_valid": signature_valid,
        "recent_events_limit": limit,
        "recent_events": load_recent_jsonl(&events_path, limit),
    })
}
