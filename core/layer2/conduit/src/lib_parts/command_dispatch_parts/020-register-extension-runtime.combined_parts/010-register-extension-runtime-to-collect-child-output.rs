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

fn resolve_infring_ops_command(root: &PathBuf, domain: &str) -> (String, Vec<String>) {
    let explicit = std::env::var("INFRING_OPS_BIN").ok();
    if let Some(bin) = explicit {
        let trimmed = bin.trim();
        if !trimmed.is_empty() {
            return (trimmed.to_string(), vec![domain.to_string()]);
        }
    }

    let release = root.join("target").join("release").join("infring-ops");
    if release.exists() {
        return (
            release.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }
    let debug = root.join("target").join("debug").join("infring-ops");
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
            "infring-ops".to_string(),
            "--".to_string(),
            domain.to_string(),
        ],
    )
}

fn bridge_command_timeout_ms() -> u64 {
    std::env::var("INFRING_OPS_BRIDGE_TIMEOUT_MS")
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
