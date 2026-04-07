fn register_extension_runtime(
    root: &PathBuf,
    input: RegisterExtensionInput,
) -> Result<Value, String> {
    let registry_path = resolve_plugin_registry_path(root);
    let mut registry = load_plugin_registry(&registry_path);
    let now_ms = now_ts_ms();

    let component_path = input
        .wasm_component_path
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "extension_wasm_component_path_required".to_string())?;

    let max_recovery_attempts = input
        .recovery_max_attempts
        .unwrap_or(PLUGIN_DEFAULT_MAX_RECOVERY_ATTEMPTS)
        .clamp(1, PLUGIN_MAX_RECOVERY_ATTEMPTS);
    let recovery_backoff_ms = input
        .recovery_backoff_ms
        .unwrap_or(PLUGIN_DEFAULT_RECOVERY_BACKOFF_MS)
        .clamp(
            PLUGIN_MIN_RECOVERY_BACKOFF_MS,
            PLUGIN_MAX_RECOVERY_BACKOFF_MS,
        );

    let plugin_id = input.extension_id.trim().to_string();
    let plugin_type = normalize_plugin_type(input.plugin_type.as_deref());
    let version = input
        .version
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("0.1.0")
        .to_string();

    let mut entry = PluginRegistryEntry {
        plugin_id: plugin_id.clone(),
        plugin_type,
        version,
        wasm_component_path: component_path.to_string(),
        wasm_sha256: input.wasm_sha256,
        capabilities: input.capabilities,
        signature: input.signature,
        provenance: input.provenance,
        enabled: true,
        status: "healing".to_string(),
        failure_count: 0,
        max_recovery_attempts,
        recovery_backoff_ms,
        next_retry_ts_ms: 0,
        last_healthcheck_ts_ms: 0,
        last_error: None,
        quarantined_reason: None,
        registered_ts_ms: now_ms,
    };
    normalize_plugin_entry(&mut entry);

    let mut install_event = serde_json::json!({
        "type": "plugin_runtime_registered",
        "plugin_id": plugin_id,
        "plugin_type": entry.plugin_type,
        "version": entry.version,
        "component_path": entry.wasm_component_path,
        "capabilities": entry.capabilities
    });

    match plugin_health_check(root, &entry) {
        Ok(()) => {
            let _ = mark_plugin_healthy(&mut entry, now_ms, "register");
            if let Some(obj) = install_event.as_object_mut() {
                obj.insert("status".to_string(), Value::String(entry.status.clone()));
            }
        }
        Err(err) => {
            let heal_event = mark_plugin_failure(&mut entry, &err, now_ms);
            if let Some(obj) = install_event.as_object_mut() {
                obj.insert("status".to_string(), Value::String(entry.status.clone()));
                obj.insert("health_error".to_string(), Value::String(err));
                obj.insert("heal_event".to_string(), heal_event);
            }
        }
    }

    if let Some(existing) = registry
        .plugins
        .iter_mut()
        .find(|plugin| plugin.plugin_id == entry.plugin_id)
    {
        *existing = entry.clone();
    } else {
        registry.plugins.push(entry.clone());
    }
    registry.updated_ts_ms = now_ms;

    save_plugin_registry(&registry_path, &registry)?;
    let _ = append_plugin_runtime_receipt(root, install_event);
    Ok(run_plugin_runtime_autoheal(root, "register_extension"))
}

fn repo_root_from_current_dir() -> PathBuf {
    let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut cursor = Some(start.as_path());
    while let Some(path) = cursor {
        if path
            .join("core")
            .join("layer0")
            .join("ops")
            .join("Cargo.toml")
            .exists()
            && path.join("client").join("runtime").exists()
        {
            return path.to_path_buf();
        }
        cursor = path.parent();
    }
    start
}

fn resolve_protheus_ops_command(root: &PathBuf, domain: &str) -> (String, Vec<String>) {
    let explicit = std::env::var("PROTHEUS_OPS_BIN").ok();
    if let Some(bin) = explicit {
        let trimmed = bin.trim();
        if !trimmed.is_empty() {
            return (trimmed.to_string(), vec![domain.to_string()]);
        }
    }

    let release = root.join("target").join("release").join("protheus-ops");
    if release.exists() {
        return (
            release.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }
    let debug = root.join("target").join("debug").join("protheus-ops");
    if debug.exists() {
        return (
            debug.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }

    (
        "cargo".to_string(),
        vec![
            "run".to_string(),
            "--quiet".to_string(),
            "--manifest-path".to_string(),
            "core/layer0/ops/Cargo.toml".to_string(),
            "--bin".to_string(),
            "protheus-ops".to_string(),
            "--".to_string(),
            domain.to_string(),
        ],
    )
}

fn bridge_command_timeout_ms() -> u64 {
    std::env::var("PROTHEUS_OPS_BRIDGE_TIMEOUT_MS")
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .map(|ms| ms.clamp(1_000, 15 * 60 * 1_000))
        .unwrap_or(110_000)
}

fn collect_child_output(child: &mut std::process::Child) -> (String, String) {
    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut handle) = child.stdout.take() {
        let mut buf = Vec::new();
        let _ = handle.read_to_end(&mut buf);
        stdout = String::from_utf8_lossy(&buf).to_string();
    }
    if let Some(mut handle) = child.stderr.take() {
        let mut buf = Vec::new();
        let _ = handle.read_to_end(&mut buf);
        stderr = String::from_utf8_lossy(&buf).to_string();
    }
    (stdout, stderr)
}

fn execute_ops_bridge_command(domain: &str, args: &[String], run_context: Option<&str>) -> Value {
    let root = repo_root_from_current_dir();
    let (command, mut command_args) = resolve_protheus_ops_command(&root, domain);
    command_args.extend(args.iter().cloned());
    let timeout_ms = bridge_command_timeout_ms();

    let mut cmd = Command::new(&command);
    cmd.args(&command_args)
        .current_dir(&root)
        .env(
            "PROTHEUS_NODE_BINARY",
            std::env::var("PROTHEUS_NODE_BINARY").unwrap_or_else(|_| "node".to_string()),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(context) = run_context {
        let trimmed = context.trim();
        if !trimmed.is_empty() {
            cmd.env("SPINE_RUN_CONTEXT", trimmed);
        }
    }

    match cmd.spawn() {
        Ok(mut child) => match child.wait_timeout(Duration::from_millis(timeout_ms)) {
            Ok(Some(status)) => {
                let (stdout, stderr) = collect_child_output(&mut child);
                let exit_code = status.code().unwrap_or(1);
                let spine_receipt = parse_json_payload(&stdout);
                let mut detail = serde_json::json!({
                    "ok": exit_code == 0,
                    "type": if exit_code == 0 {
                        format!("{domain}_bridge_ok")
                    } else {
                        format!("{domain}_bridge_error")
                    },
                    "exit_code": exit_code,
                    "command": command,
                    "args": command_args,
                    "run_context": run_context,
                    "stdout": stdout,
                    "stderr": stderr,
                    "routed_via": "conduit",
                    "domain": domain,
                    "bridge_timeout_ms": timeout_ms
                });
                if let Some(receipt) = spine_receipt {
                    detail["domain_receipt"] = receipt.clone();
                    if let Some(kind) = receipt.get("type").and_then(Value::as_str) {
                        detail["type"] = Value::String(kind.to_string());
                    }
                    if let Some(ok) = receipt.get("ok").and_then(Value::as_bool) {
                        detail["ok"] = Value::Bool(ok && exit_code == 0);
                    }
                    if let Some(reason) = receipt.get("reason").and_then(Value::as_str) {
                        detail["reason"] = Value::String(reason.to_string());
                    } else if let Some(reason) =
                        receipt.get("failure_reason").and_then(Value::as_str)
                    {
                        detail["reason"] = Value::String(reason.to_string());
                    }
                }
                detail["receipt_hash"] = Value::String(deterministic_receipt_hash(&detail));
                detail
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let (stdout, stderr) = collect_child_output(&mut child);
                let mut detail = serde_json::json!({
                    "ok": false,
                    "type": format!("{domain}_bridge_timeout"),
                    "exit_code": 124,
                    "reason": format!("{domain}_bridge_timeout:{timeout_ms}"),
                    "command": command,
                    "args": command_args,
                    "run_context": run_context,
                    "stdout": stdout,
                    "stderr": stderr,
                    "routed_via": "conduit",
                    "domain": domain,
                    "bridge_timeout_ms": timeout_ms
                });
                detail["receipt_hash"] = Value::String(deterministic_receipt_hash(&detail));
                detail
            }
            Err(err) => {
                let mut detail = serde_json::json!({
                    "ok": false,
                    "type": format!("{domain}_bridge_wait_error"),
                    "exit_code": 1,
                    "reason": format!("{domain}_bridge_wait_failed:{err}"),
                    "command": command,
                    "args": command_args,
                    "run_context": run_context,
                    "stdout": "",
                    "stderr": "",
                    "routed_via": "conduit",
                    "domain": domain,
                    "bridge_timeout_ms": timeout_ms
                });
                detail["receipt_hash"] = Value::String(deterministic_receipt_hash(&detail));
                detail
            }
        },
        Err(err) => {
            let mut detail = serde_json::json!({
                "ok": false,
                "type": format!("{domain}_bridge_spawn_error"),
                "exit_code": 1,
                "reason": format!("{domain}_bridge_spawn_failed:{err}"),
                "command": command,
                "args": command_args,
                "run_context": run_context,
                "stdout": "",
                "stderr": "",
                "routed_via": "conduit",
                "domain": domain,
                "bridge_timeout_ms": timeout_ms
            });
            detail["receipt_hash"] = Value::String(deterministic_receipt_hash(&detail));
            detail
        }
    }
}

fn execute_spine_bridge_command(args: &[String], run_context: Option<&str>) -> Value {
    let mut detail = execute_ops_bridge_command("spine", args, run_context);
    if let Some(receipt) = detail.get("domain_receipt").cloned() {
        detail["spine_receipt"] = receipt;
    }
    detail
}

fn execute_attention_bridge_command(args: &[String]) -> Value {
    let mut detail = execute_ops_bridge_command("attention-queue", args, None);
    if let Some(receipt) = detail.get("domain_receipt").cloned() {
        detail["attention_receipt"] = receipt;
    }
    detail
}

fn execute_persona_ambient_bridge_command(args: &[String]) -> Value {
    let mut detail = execute_ops_bridge_command("persona-ambient", args, None);
    if let Some(receipt) = detail.get("domain_receipt").cloned() {
        detail["persona_ambient_receipt"] = receipt;
    }
    detail
}

fn execute_dopamine_ambient_bridge_command(args: &[String]) -> Value {
    let mut detail = execute_ops_bridge_command("dopamine-ambient", args, None);
    if let Some(receipt) = detail.get("domain_receipt").cloned() {
        detail["dopamine_ambient_receipt"] = receipt;
    }
    detail
}

fn execute_memory_ambient_bridge_command(args: &[String]) -> Value {
    let mut detail = execute_ops_bridge_command("memory-ambient", args, None);
    if let Some(receipt) = detail.get("domain_receipt").cloned() {
        detail["memory_ambient_receipt"] = receipt;
    }
    detail
}

#[cfg(feature = "edge")]
fn edge_backend_label() -> &'static str {
    "picolm_static_stub"
}

#[cfg(not(feature = "edge"))]
fn edge_backend_label() -> &'static str {
    "edge_feature_disabled"
}

fn normalize_edge_prompt(prompt: &str) -> String {
    let normalized = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        "(empty_prompt)".to_string()
    } else {
        normalized
    }
}

fn summarize_for_edge_backend(prompt: &str, token_cap: usize) -> String {
    let tokens = prompt.split_whitespace().collect::<Vec<_>>();
    if tokens.len() <= token_cap {
        return tokens.join(" ");
    }
    tokens
        .into_iter()
        .take(token_cap)
        .collect::<Vec<_>>()
        .join(" ")
}

fn clean_lane_id(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        .collect::<String>()
        .to_ascii_uppercase()
}

pub fn deterministic_receipt_hash<T: Serialize>(value: &T) -> String {
    let canonical = canonical_json(value);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

const VERITY_DRIFT_CONFIG_SCHEMA_ID: &str = "infring_verity_drift_policy";
const VERITY_DRIFT_CONFIG_SCHEMA_VERSION: u32 = 1;
const VERITY_DRIFT_CONFIG_POLICY_VERSION: u32 = 1;
const VERITY_DRIFT_MODE_PRODUCTION: &str = "production";
const VERITY_DRIFT_MODE_SIMULATION: &str = "simulation";
const VERITY_DRIFT_PRODUCTION_DEFAULT_MS: i64 = 500;
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
            if !patch_id.starts_with("constitution_safe/") {
                return Some("policy_update_must_be_constitution_safe".to_string());
            }
        }
        TsCommand::InstallExtension {
            extension_id,
            wasm_sha256,
            capabilities,
            plugin_type,
            wasm_component_path,
            ..
        } => {
            if extension_id.trim().is_empty() {
                return Some("extension_id_required".to_string());
            }
            if !is_valid_sha256(wasm_sha256) {
                return Some("extension_wasm_sha256_invalid".to_string());
            }
            if capabilities.is_empty() || capabilities.iter().any(|cap| cap.trim().is_empty()) {
                return Some("extension_capabilities_invalid".to_string());
            }
            if wasm_component_path
                .as_deref()
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .is_none()
            {
                return Some("extension_wasm_component_path_required".to_string());
            }
            if let Some(plugin_type) = plugin_type {
                if !is_valid_plugin_type(plugin_type.trim()) {
                    return Some("extension_plugin_type_invalid".to_string());
                }
            }
        }
        TsCommand::ListActiveAgents | TsCommand::GetSystemStatus => {}
    }
    None
}
