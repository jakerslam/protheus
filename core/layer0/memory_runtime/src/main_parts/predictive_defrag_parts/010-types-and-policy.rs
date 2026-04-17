use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PREDICTIVE_DEFRAG_CONFIG_SCHEMA_ID: &str = "infring_memory_predictive_defrag_policy";
const PREDICTIVE_DEFRAG_CONFIG_SCHEMA_VERSION: u32 = 1;
const PREDICTIVE_DEFRAG_CONFIG_POLICY_VERSION: u32 = 1;
const PREDICTIVE_DEFRAG_MODE_PRODUCTION: &str = "production";
const PREDICTIVE_DEFRAG_MODE_SIMULATION: &str = "simulation";
const PREDICTIVE_DEFRAG_THRESHOLD_DEFAULT_PERCENT: f64 = 6.0;
const PREDICTIVE_DEFRAG_REACTIVE_THRESHOLD_PERCENT: f64 = 14.3;
const PREDICTIVE_DEFRAG_POLL_INTERVAL_DEFAULT_MS: u64 = 1_200;
const PREDICTIVE_DEFRAG_TRIGGER_COOLDOWN_MS: u64 = 3_000;

fn normalize_predictive_execution_status(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "ok" | "success" | "succeeded" | "ready" => "success",
        "timeout" | "timed_out" | "timed-out" => "timeout",
        "throttled" | "rate_limited" | "rate-limited" | "429" => "throttled",
        _ => "error",
    }
}

fn sanitize_predictive_token(raw: &str, max_len: usize) -> String {
    let mut out = String::with_capacity(raw.len().min(max_len));
    let mut prev_underscore = false;
    for ch in raw.trim().to_ascii_lowercase().chars() {
        let keep = ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.');
        if keep {
            out.push(ch);
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
        if out.len() >= max_len {
            break;
        }
    }
    while out.starts_with('_') {
        out.remove(0);
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

fn predictive_execution_receipt(command: &str, status: &str, error_kind: Option<&str>) -> Value {
    let normalized_status = normalize_predictive_execution_status(status);
    let normalized_command = sanitize_predictive_token(command, 96);
    let normalized_error = error_kind.and_then(|raw| {
        let token = sanitize_predictive_token(raw, 64);
        if token.is_empty() { None } else { Some(token) }
    });
    let seed = json!({
        "command": normalized_command,
        "status": normalized_status,
        "error_kind": normalized_error
    });
    let call_id = format!("predictive-defrag-{}", &sha256_hex(&stable_json_string(&seed))[..16]);
    json!({
        "call_id": call_id,
        "status": normalized_status,
        "error_kind": normalized_error,
        "telemetry": {
            "duration_ms": 0,
            "tokens_used": 0
        }
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PredictiveDefragFlagConfig {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    threshold_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PredictiveDefragMemoryConfig {
    #[serde(default)]
    predictive_defrag: PredictiveDefragFlagConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PredictiveDefragSignedConfig {
    schema_id: String,
    schema_version: u32,
    policy_version: u32,
    mode: String,
    #[serde(default)]
    memory: PredictiveDefragMemoryConfig,
    #[serde(default)]
    production_threshold_percent: Option<f64>,
    #[serde(default)]
    simulation_threshold_percent: Option<f64>,
    reactive_threshold_percent: f64,
    poll_interval_ms: u64,
    signature: String,
}

#[derive(Debug, Clone, Serialize)]
struct PredictiveDefragRuntimePolicy {
    mode: String,
    enabled: bool,
    active_threshold_percent: f64,
    reactive_threshold_percent: f64,
    poll_interval_ms: u64,
    policy_version: u32,
    signature_valid: bool,
    config_path: String,
}

#[derive(Debug, Clone, Default, Serialize)]
struct PredictiveDefragMonitorState {
    checks: u64,
    trigger_count: u64,
    last_checked_at_ms: u64,
    last_triggered_at_ms: u64,
    last_trigger_fragmentation_percent: f64,
    last_before_fidelity_score: f64,
    last_after_fidelity_score: f64,
    last_drift_delta: f64,
    last_energy_improvement_percent: f64,
    last_latency_improvement_percent: f64,
    last_receipt_hash: String,
    last_receipt_path: String,
    last_error: String,
    mode: String,
    enabled: bool,
    active_threshold_percent: f64,
    reactive_threshold_percent: f64,
}

struct PredictiveDefragMonitorHandle {
    root: PathBuf,
    mode_hint: String,
    db_path_raw: String,
    state: Arc<Mutex<PredictiveDefragMonitorState>>,
    shutdown: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl PredictiveDefragMonitorHandle {
    fn status_payload(&self) -> Value {
        let snapshot = self
            .state
            .lock()
            .ok()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        build_predictive_defrag_status_payload(
            &self.root,
            &self.mode_hint,
            &self.db_path_raw,
            snapshot,
        )
    }

    fn shutdown(&mut self) {
        self.shutdown.store(true, AtomicOrdering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn build_predictive_defrag_status_payload(
    root: &Path,
    mode_hint: &str,
    db_path_raw: &str,
    snapshot: PredictiveDefragMonitorState,
) -> Value {
    let policy = load_predictive_defrag_policy(root, mode_hint);
    let mut current_fragmentation_percent = 0.0;
    let mut tier_stats = json!({});
    if let Ok(db) = MemoryDb::open(root, db_path_raw) {
        if let Ok(stats) = db.fragmentation_stats() {
            current_fragmentation_percent = round4(stats.fragmentation_ratio * 100.0);
            tier_stats = json!({
                "working": stats.working_rows,
                "episodic": stats.episodic_rows,
                "semantic": stats.semantic_rows
            });
        }
    }
    let execution_receipt = predictive_execution_receipt(
        "memory_predictive_defrag_status",
        "success",
        None,
    );
    json!({
        "ok": true,
        "type": "memory_predictive_defrag_status",
        "execution_receipt": execution_receipt,
        "policy": policy,
        "current_fragmentation_percent": current_fragmentation_percent,
        "tiers": tier_stats,
        "monitor": snapshot
    })
}

fn predictive_defrag_status_payload(args: &HashMap<String, String>) -> Value {
    let root = PathBuf::from(arg_or_default(
        args,
        "root",
        detect_default_root().to_string_lossy().as_ref(),
    ));
    let mode_hint = resolve_predictive_mode_hint(args);
    let db_path_raw = arg_any(args, &["db-path", "db_path"]);
    build_predictive_defrag_status_payload(
        &root,
        &mode_hint,
        &db_path_raw,
        PredictiveDefragMonitorState::default(),
    )
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn normalize_predictive_mode(raw: &str) -> String {
    let lowered = raw.trim().to_ascii_lowercase();
    if lowered == PREDICTIVE_DEFRAG_MODE_SIMULATION || lowered == "sim" {
        PREDICTIVE_DEFRAG_MODE_SIMULATION.to_string()
    } else {
        PREDICTIVE_DEFRAG_MODE_PRODUCTION.to_string()
    }
}

fn resolve_predictive_mode_hint(args: &HashMap<String, String>) -> String {
    let arg_mode = arg_any(args, &["mode", "runtime-mode", "runtime_mode"]);
    if !arg_mode.trim().is_empty() {
        return normalize_predictive_mode(&arg_mode);
    }
    let env_mode = env::var("INFRING_RUNTIME_MODE")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| env::var("PROTHEUS_RUNTIME_MODE").ok())
        .unwrap_or_else(|| PREDICTIVE_DEFRAG_MODE_PRODUCTION.to_string());
    normalize_predictive_mode(&env_mode)
}

fn default_predictive_enabled(mode: &str) -> bool {
    mode == PREDICTIVE_DEFRAG_MODE_PRODUCTION
}

fn resolve_predictive_defrag_path(root: &Path, env_key: &str, fallback_rel: &str) -> PathBuf {
    let explicit = env::var(env_key)
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

fn resolve_predictive_defrag_config_path(root: &Path) -> PathBuf {
    resolve_predictive_defrag_path(
        root,
        "INFRING_MEMORY_PREDICTIVE_DEFRAG_CONFIG_PATH",
        "local/state/ops/verity/memory/predictive_defrag_policy.signed.json",
    )
}

fn resolve_predictive_defrag_events_path(root: &Path) -> PathBuf {
    resolve_predictive_defrag_path(
        root,
        "INFRING_MEMORY_PREDICTIVE_DEFRAG_EVENTS_PATH",
        "local/state/ops/verity/memory/predictive_defrag_events.jsonl",
    )
}

fn resolve_predictive_defrag_latest_path(root: &Path) -> PathBuf {
    resolve_predictive_defrag_path(
        root,
        "INFRING_MEMORY_PREDICTIVE_DEFRAG_LATEST_PATH",
        "local/state/ops/verity/memory/predictive_defrag_latest.json",
    )
}

fn predictive_defrag_signing_key() -> String {
    env::var("INFRING_MEMORY_PREDICTIVE_DEFRAG_SIGNING_KEY")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| "infring-memory-predictive-defrag-local-key".to_string())
}

fn predictive_defrag_signature_payload(config: &PredictiveDefragSignedConfig) -> Value {
    json!({
        "schema_id": config.schema_id,
        "schema_version": config.schema_version,
        "policy_version": config.policy_version,
        "mode": config.mode,
        "memory": {
            "predictive_defrag": {
                "enabled": config.memory.predictive_defrag.enabled,
                "threshold_percent": config.memory.predictive_defrag.threshold_percent
            }
        },
        "reactive_threshold_percent": config.reactive_threshold_percent,
        "poll_interval_ms": config.poll_interval_ms
    })
}

fn sign_predictive_defrag_payload(payload: &Value) -> String {
    let encoded = serde_json::to_string(&json!({
        "payload": payload,
        "signing_key": predictive_defrag_signing_key()
    }))
    .unwrap_or_else(|_| "{}".to_string());
    format!("sig:{}", sha256_hex(&encoded))
}

fn clamp_percent(value: f64, floor: f64, ceil: f64) -> f64 {
    if !value.is_finite() {
        return floor;
    }
    value.clamp(floor, ceil)
}

fn signed_default_predictive_defrag_config(mode_hint: &str) -> PredictiveDefragSignedConfig {
    let mode = normalize_predictive_mode(mode_hint);
    let mut config = PredictiveDefragSignedConfig {
        schema_id: PREDICTIVE_DEFRAG_CONFIG_SCHEMA_ID.to_string(),
        schema_version: PREDICTIVE_DEFRAG_CONFIG_SCHEMA_VERSION,
        policy_version: PREDICTIVE_DEFRAG_CONFIG_POLICY_VERSION,
        mode: mode.clone(),
        memory: PredictiveDefragMemoryConfig {
            predictive_defrag: PredictiveDefragFlagConfig {
                enabled: Some(default_predictive_enabled(&mode)),
                threshold_percent: Some(PREDICTIVE_DEFRAG_THRESHOLD_DEFAULT_PERCENT),
            },
        },
        production_threshold_percent: None,
        simulation_threshold_percent: None,
        reactive_threshold_percent: PREDICTIVE_DEFRAG_REACTIVE_THRESHOLD_PERCENT,
        poll_interval_ms: PREDICTIVE_DEFRAG_POLL_INTERVAL_DEFAULT_MS,
        signature: String::new(),
    };
    config.signature = sign_predictive_defrag_payload(&predictive_defrag_signature_payload(&config));
    config
}

fn write_predictive_defrag_signed_config(path: &Path, config: &PredictiveDefragSignedConfig) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let body = serde_json::to_string_pretty(config).unwrap_or_else(|_| "{}".to_string());
    let _ = fs::write(path, body);
}

fn runtime_predictive_policy_from_signed(
    config_path: PathBuf,
    mut signed: PredictiveDefragSignedConfig,
    signature_valid: bool,
) -> PredictiveDefragRuntimePolicy {
    signed.mode = normalize_predictive_mode(&signed.mode);
    signed.reactive_threshold_percent =
        clamp_percent(signed.reactive_threshold_percent, PREDICTIVE_DEFRAG_THRESHOLD_DEFAULT_PERCENT, 99.0);
    signed.poll_interval_ms = signed.poll_interval_ms.clamp(250, 60_000);
    let enabled = signed
        .memory
        .predictive_defrag
        .enabled
        .unwrap_or_else(|| default_predictive_enabled(&signed.mode));
    let legacy_threshold = signed.production_threshold_percent.or(signed.simulation_threshold_percent);
    let configured_threshold = signed
        .memory
        .predictive_defrag
        .threshold_percent
        .or(legacy_threshold)
        .unwrap_or(PREDICTIVE_DEFRAG_THRESHOLD_DEFAULT_PERCENT);
    let active_threshold_percent = clamp_percent(configured_threshold, 0.5, 95.0);
    PredictiveDefragRuntimePolicy {
        mode: signed.mode,
        enabled,
        active_threshold_percent: round4(active_threshold_percent),
        reactive_threshold_percent: round4(signed.reactive_threshold_percent),
        poll_interval_ms: signed.poll_interval_ms,
        policy_version: signed.policy_version.max(1),
        signature_valid,
        config_path: config_path.to_string_lossy().to_string(),
    }
}

fn load_predictive_defrag_policy(root: &Path, mode_hint: &str) -> PredictiveDefragRuntimePolicy {
    let config_path = resolve_predictive_defrag_config_path(root);
    let default_signed = signed_default_predictive_defrag_config(mode_hint);
    let raw = match fs::read_to_string(&config_path) {
        Ok(value) => value,
        Err(_) => {
            write_predictive_defrag_signed_config(&config_path, &default_signed);
            return runtime_predictive_policy_from_signed(config_path, default_signed, true);
        }
    };
    let mut signed = match serde_json::from_str::<PredictiveDefragSignedConfig>(&raw) {
        Ok(value) => value,
        Err(_) => {
            write_predictive_defrag_signed_config(&config_path, &default_signed);
            return runtime_predictive_policy_from_signed(config_path, default_signed, false);
        }
    };
    let expected = sign_predictive_defrag_payload(&predictive_defrag_signature_payload(&signed));
    let signature_valid = signed.signature.trim() == expected;
    if !signature_valid {
        write_predictive_defrag_signed_config(&config_path, &default_signed);
        return runtime_predictive_policy_from_signed(config_path, default_signed, false);
    }
    if signed.schema_id != PREDICTIVE_DEFRAG_CONFIG_SCHEMA_ID
        || signed.schema_version != PREDICTIVE_DEFRAG_CONFIG_SCHEMA_VERSION
    {
        signed.schema_id = PREDICTIVE_DEFRAG_CONFIG_SCHEMA_ID.to_string();
        signed.schema_version = PREDICTIVE_DEFRAG_CONFIG_SCHEMA_VERSION;
        signed.signature = sign_predictive_defrag_payload(&predictive_defrag_signature_payload(&signed));
        write_predictive_defrag_signed_config(&config_path, &signed);
    }
    runtime_predictive_policy_from_signed(config_path, signed, signature_valid)
}

fn memory_fidelity_score(fragmentation_percent: f64) -> f64 {
    round4((1.0 - (fragmentation_percent / 100.0)).clamp(0.0, 1.0))
}

fn estimate_memory_energy_units(fragmentation_percent: f64, stats: &db::DbFragmentationStats) -> f64 {
    let tier_load = stats.working_rows + stats.episodic_rows + stats.semantic_rows;
    round4(8.0 + (fragmentation_percent * 0.45) + (tier_load as f64 / 8_000.0))
}

fn estimate_context_switch_latency_ms(
    fragmentation_percent: f64,
    stats: &db::DbFragmentationStats,
) -> f64 {
    let tier_load = stats.working_rows + stats.episodic_rows + stats.semantic_rows;
    round4(2.2 + (fragmentation_percent * 0.22) + (tier_load as f64 / 16_000.0))
}

fn append_predictive_defrag_receipt(root: &Path, payload: &Value) -> String {
    let events_path = resolve_predictive_defrag_events_path(root);
    let latest_path = resolve_predictive_defrag_latest_path(root);
    if let Some(parent) = events_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Some(parent) = latest_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let encoded = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&events_path)
        .and_then(|mut handle| {
            handle.write_all(encoded.as_bytes())?;
            handle.write_all(b"\n")
        });
    let _ = fs::write(
        latest_path,
        serde_json::to_string_pretty(payload).unwrap_or_else(|_| "{}".to_string()),
    );
    events_path.to_string_lossy().to_string()
}
