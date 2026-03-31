fn parse_json_payload(stdout: &str) -> Option<Value> {
    let raw = stdout.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(payload) = serde_json::from_str::<Value>(raw) {
        return Some(payload);
    }
    for line in raw.lines().rev() {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') {
            continue;
        }
        if let Ok(payload) = serde_json::from_str::<Value>(trimmed) {
            return Some(payload);
        }
    }
    None
}

fn resolve_cockpit_latest_path(root: &PathBuf) -> PathBuf {
    let explicit = std::env::var("COCKPIT_INBOX_LATEST_PATH")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    if let Some(path) = explicit {
        let candidate = PathBuf::from(path);
        if candidate.is_absolute() {
            return candidate;
        }
        return root.join(candidate);
    }
    root.join(runtime_paths::CLIENT_STATE_ROOT)
        .join("cockpit")
        .join("inbox")
        .join("latest.json")
}

fn load_cockpit_summary(root: &PathBuf) -> Value {
    let latest_path = resolve_cockpit_latest_path(root);
    let raw = match fs::read_to_string(&latest_path) {
        Ok(v) => v,
        Err(_) => {
            return serde_json::json!({
                "available": false,
                "path": latest_path.to_string_lossy().to_string()
            });
        }
    };
    let parsed = serde_json::from_str::<Value>(&raw).ok();
    let Some(value) = parsed else {
        return serde_json::json!({
            "available": false,
            "path": latest_path.to_string_lossy().to_string(),
            "reason": "cockpit_latest_invalid_json"
        });
    };
    let payload = value.as_object();
    let Some(obj) = payload else {
        return serde_json::json!({
            "available": false,
            "path": latest_path.to_string_lossy().to_string(),
            "reason": "cockpit_latest_not_object"
        });
    };
    serde_json::json!({
        "available": true,
        "path": latest_path.to_string_lossy().to_string(),
        "ts": obj.get("ts").cloned().unwrap_or(Value::Null),
        "sequence": obj.get("sequence").cloned().unwrap_or(Value::Null),
        "consumer_id": obj.get("consumer_id").cloned().unwrap_or(Value::Null),
        "attention_batch_count": value.pointer("/attention/batch_count").cloned().unwrap_or(Value::Null),
        "attention_queue_depth": value.pointer("/attention/queue_depth").cloned().unwrap_or(Value::Null),
        "receipt_hash": obj.get("receipt_hash").cloned().unwrap_or(Value::Null)
    })
}

const PLUGIN_REGISTRY_SCHEMA_VERSION: u32 = 1;
const PLUGIN_DEFAULT_MAX_RECOVERY_ATTEMPTS: u32 = 3;
const PLUGIN_DEFAULT_RECOVERY_BACKOFF_MS: u64 = 3_000;
const PLUGIN_MAX_RECOVERY_ATTEMPTS: u32 = 10;
const PLUGIN_MIN_RECOVERY_BACKOFF_MS: u64 = 500;
const PLUGIN_MAX_RECOVERY_BACKOFF_MS: u64 = 300_000;
const PLUGIN_MAX_STATUS_LIST: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginRegistryState {
    schema_version: u32,
    updated_ts_ms: u64,
    plugins: Vec<PluginRegistryEntry>,
}

impl Default for PluginRegistryState {
    fn default() -> Self {
        Self {
            schema_version: PLUGIN_REGISTRY_SCHEMA_VERSION,
            updated_ts_ms: 0,
            plugins: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginRegistryEntry {
    plugin_id: String,
    plugin_type: String,
    version: String,
    wasm_component_path: String,
    wasm_sha256: String,
    capabilities: Vec<String>,
    signature: Option<String>,
    provenance: Option<String>,
    enabled: bool,
    status: String,
    failure_count: u32,
    max_recovery_attempts: u32,
    recovery_backoff_ms: u64,
    next_retry_ts_ms: u64,
    last_healthcheck_ts_ms: u64,
    last_error: Option<String>,
    quarantined_reason: Option<String>,
    registered_ts_ms: u64,
}

fn resolve_plugin_registry_path(root: &PathBuf) -> PathBuf {
    let explicit = std::env::var("INFRING_PLUGIN_REGISTRY_PATH")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    if let Some(path) = explicit {
        let candidate = PathBuf::from(path);
        if candidate.is_absolute() {
            return candidate;
        }
        return root.join(candidate);
    }

    root.join(runtime_paths::CLIENT_STATE_ROOT)
        .join("extensions")
        .join("plugin_registry.json")
}

fn resolve_plugin_receipts_path(root: &PathBuf) -> PathBuf {
    let explicit = std::env::var("INFRING_PLUGIN_RUNTIME_RECEIPTS_PATH")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    if let Some(path) = explicit {
        let candidate = PathBuf::from(path);
        if candidate.is_absolute() {
            return candidate;
        }
        return root.join(candidate);
    }

    root.join(runtime_paths::CLIENT_STATE_ROOT)
        .join("extensions")
        .join("plugin_runtime_receipts.jsonl")
}

fn write_string_atomic(path: &Path, body: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&tmp, body)?;
    fs::rename(tmp, path)?;
    Ok(())
}

fn load_plugin_registry(path: &Path) -> PluginRegistryState {
    let raw = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return PluginRegistryState::default(),
    };
    let mut parsed: PluginRegistryState = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return PluginRegistryState::default(),
    };
    for plugin in &mut parsed.plugins {
        normalize_plugin_entry(plugin);
    }
    parsed
}

fn save_plugin_registry(path: &Path, registry: &PluginRegistryState) -> Result<(), String> {
    let body = serde_json::to_string_pretty(registry).map_err(|e| format!("encode_failed:{e}"))?;
    write_string_atomic(path, &body).map_err(|e| format!("write_failed:{e}"))
}

fn append_plugin_runtime_receipt(root: &PathBuf, mut payload: Value) -> Result<(), String> {
    let receipts_path = resolve_plugin_receipts_path(root);
    if let Some(parent) = receipts_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir_failed:{e}"))?;
    }
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("ts_ms".to_string(), serde_json::json!(now_ts_ms()));
    }
    let mut handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&receipts_path)
        .map_err(|e| format!("open_receipts_failed:{e}"))?;
    let line = serde_json::to_string(&payload).map_err(|e| format!("encode_receipt_failed:{e}"))?;
    handle
        .write_all(line.as_bytes())
        .and_then(|_| handle.write_all(b"\n"))
        .map_err(|e| format!("append_receipt_failed:{e}"))
}

fn normalize_plugin_type(raw: Option<&str>) -> String {
    let candidate = raw
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("substrate_adapter")
        .to_ascii_lowercase();
    if is_valid_plugin_type(&candidate) {
        candidate
    } else {
        "substrate_adapter".to_string()
    }
}

fn normalize_plugin_entry(plugin: &mut PluginRegistryEntry) {
    plugin.max_recovery_attempts = plugin
        .max_recovery_attempts
        .clamp(1, PLUGIN_MAX_RECOVERY_ATTEMPTS);
    plugin.recovery_backoff_ms = plugin.recovery_backoff_ms.clamp(
        PLUGIN_MIN_RECOVERY_BACKOFF_MS,
        PLUGIN_MAX_RECOVERY_BACKOFF_MS,
    );
    if plugin.status.trim().is_empty() {
        plugin.status = "healing".to_string();
    }
    if plugin.plugin_type.trim().is_empty() || !is_valid_plugin_type(&plugin.plugin_type) {
        plugin.plugin_type = "substrate_adapter".to_string();
    }
}

fn resolve_plugin_component_path(root: &PathBuf, raw: &str) -> PathBuf {
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn hash_file_sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("read_failed:{e}"))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn plugin_health_check(root: &PathBuf, plugin: &PluginRegistryEntry) -> Result<(), String> {
    let path = resolve_plugin_component_path(root, &plugin.wasm_component_path);
    if !path.exists() {
        return Err("wasm_component_missing".to_string());
    }
    if !path.is_file() {
        return Err("wasm_component_not_file".to_string());
    }
    let observed = hash_file_sha256(&path)?;
    if !observed.eq_ignore_ascii_case(&plugin.wasm_sha256) {
        return Err("wasm_component_sha_mismatch".to_string());
    }
    Ok(())
}

fn mark_plugin_failure(plugin: &mut PluginRegistryEntry, reason: &str, now_ms: u64) -> Value {
    plugin.failure_count = plugin.failure_count.saturating_add(1);
    plugin.last_healthcheck_ts_ms = now_ms;
    plugin.last_error = Some(reason.to_string());

    if plugin.failure_count >= plugin.max_recovery_attempts {
        plugin.status = "quarantined".to_string();
        plugin.enabled = false;
        plugin.next_retry_ts_ms = 0;
        plugin.quarantined_reason = Some(reason.to_string());
        return serde_json::json!({
            "type": "plugin_runtime_quarantined",
            "plugin_id": plugin.plugin_id,
            "status": plugin.status,
            "reason": reason,
            "failure_count": plugin.failure_count
        });
    }

    let exponent = plugin.failure_count.saturating_sub(1).min(8);
    let multiplier = 1_u64 << exponent;
    let retry_delay = plugin.recovery_backoff_ms.saturating_mul(multiplier).clamp(
        PLUGIN_MIN_RECOVERY_BACKOFF_MS,
        PLUGIN_MAX_RECOVERY_BACKOFF_MS,
    );
    plugin.status = "healing".to_string();
    plugin.next_retry_ts_ms = now_ms.saturating_add(retry_delay);

    serde_json::json!({
        "type": "plugin_runtime_retry_scheduled",
        "plugin_id": plugin.plugin_id,
        "status": plugin.status,
        "reason": reason,
        "failure_count": plugin.failure_count,
        "next_retry_ts_ms": plugin.next_retry_ts_ms
    })
}

fn mark_plugin_healthy(
    plugin: &mut PluginRegistryEntry,
    now_ms: u64,
    source: &str,
) -> Option<Value> {
    let changed =
        plugin.status != "healthy" || plugin.failure_count > 0 || plugin.last_error.is_some();
    plugin.status = "healthy".to_string();
    plugin.failure_count = 0;
    plugin.next_retry_ts_ms = 0;
    plugin.last_healthcheck_ts_ms = now_ms;
    plugin.last_error = None;
    plugin.quarantined_reason = None;
    if !changed {
        return None;
    }
    Some(serde_json::json!({
        "type": "plugin_runtime_recovered",
        "plugin_id": plugin.plugin_id,
        "status": plugin.status,
        "source": source
    }))
}

fn summarize_plugin_registry(
    root: &PathBuf,
    registry_path: &Path,
    receipts_path: &Path,
    registry: &PluginRegistryState,
    changed: bool,
    events: &[Value],
    reason: &str,
) -> Value {
    let mut healthy = 0usize;
    let mut healing = 0usize;
    let mut quarantined = 0usize;
    let mut disabled = 0usize;
    for plugin in &registry.plugins {
        match plugin.status.as_str() {
            "healthy" => healthy += 1,
            "healing" => healing += 1,
            "quarantined" => quarantined += 1,
            _ => {}
        }
        if !plugin.enabled {
            disabled += 1;
        }
    }

    let plugins = registry
        .plugins
        .iter()
        .take(PLUGIN_MAX_STATUS_LIST)
        .map(|plugin| {
            let path = resolve_plugin_component_path(root, &plugin.wasm_component_path);
            serde_json::json!({
                "plugin_id": plugin.plugin_id,
                "plugin_type": plugin.plugin_type,
                "version": plugin.version,
                "status": plugin.status,
                "enabled": plugin.enabled,
                "capabilities": plugin.capabilities,
                "component_path": plugin.wasm_component_path,
                "component_exists": path.exists(),
                "failure_count": plugin.failure_count,
                "max_recovery_attempts": plugin.max_recovery_attempts,
                "next_retry_ts_ms": plugin.next_retry_ts_ms,
                "last_healthcheck_ts_ms": plugin.last_healthcheck_ts_ms,
                "last_error": plugin.last_error,
                "quarantined_reason": plugin.quarantined_reason
            })
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "available": true,
        "schema_version": registry.schema_version,
        "updated_ts_ms": registry.updated_ts_ms,
        "path": registry_path.to_string_lossy().to_string(),
        "receipts_path": receipts_path.to_string_lossy().to_string(),
        "changed": changed,
        "plugin_count": registry.plugins.len(),
        "healthy_count": healthy,
        "healing_count": healing,
        "quarantined_count": quarantined,
        "disabled_count": disabled,
        "auto_heal_reason": reason,
        "events": events,
        "plugins": plugins
    })
}

fn run_plugin_runtime_autoheal(root: &PathBuf, reason: &str) -> Value {
    let registry_path = resolve_plugin_registry_path(root);
    let receipts_path = resolve_plugin_receipts_path(root);
    let mut registry = load_plugin_registry(&registry_path);
    let now_ms = now_ts_ms();
    let mut changed = false;
    let mut events = Vec::new();

    for plugin in &mut registry.plugins {
        normalize_plugin_entry(plugin);
        if !plugin.enabled {
            continue;
        }
        if plugin.status == "healing" && plugin.next_retry_ts_ms > now_ms {
            continue;
        }
        match plugin_health_check(root, plugin) {
            Ok(()) => {
                if let Some(event) = mark_plugin_healthy(plugin, now_ms, reason) {
                    events.push(event);
                    changed = true;
                } else {
                    plugin.last_healthcheck_ts_ms = now_ms;
                }
            }
            Err(err) => {
                let event = mark_plugin_failure(plugin, &err, now_ms);
                events.push(event);
                changed = true;
            }
        }
    }

    if changed {
        registry.updated_ts_ms = now_ms;
        let _ = save_plugin_registry(&registry_path, &registry);
        for event in &events {
            let _ = append_plugin_runtime_receipt(root, event.clone());
        }
    }

    summarize_plugin_registry(
        root,
        &registry_path,
        &receipts_path,
        &registry,
        changed,
        &events,
        reason,
    )
}

struct RegisterExtensionInput {
    extension_id: String,
    wasm_sha256: String,
    capabilities: Vec<String>,
    plugin_type: Option<String>,
    version: Option<String>,
    wasm_component_path: Option<String>,
    signature: Option<String>,
    provenance: Option<String>,
    recovery_max_attempts: Option<u32>,
    recovery_backoff_ms: Option<u64>,
}
