fn read_state(path: &Path) -> SecretBrokerState {
    lane_utils::read_json(path)
        .and_then(|value| serde_json::from_value::<SecretBrokerState>(value).ok())
        .unwrap_or_else(|| SecretBrokerState {
            version: "1.1".to_string(),
            issued: BTreeMap::new(),
        })
}

fn write_state(path: &Path, state: &SecretBrokerState) -> Result<(), String> {
    let payload = serde_json::to_value(state)
        .map_err(|err| format!("secret_broker_kernel_state_encode_failed:{err}"))?;
    lane_utils::write_json(path, &payload)
}

fn provider_type_name(provider: &ProviderConfig) -> &'static str {
    match provider {
        ProviderConfig::Env { .. } => "env",
        ProviderConfig::JsonFile { .. } => "json_file",
        ProviderConfig::Command { .. } => "command",
    }
}

fn command_provider_ref(command: &CommandSpec) -> String {
    match command {
        CommandSpec::Argv(argv) => argv.first().cloned().unwrap_or_default(),
        CommandSpec::Shell(shell) => shell.clone(),
    }
}

fn provider_env(provider: &ProviderConfig) -> Option<Value> {
    let ProviderConfig::Env {
        env,
        rotated_at_env,
        ..
    } = provider
    else {
        return None;
    };
    let value = std::env::var(env).ok()?.trim().to_string();
    if value.is_empty() {
        return None;
    }
    let rotated_at = if rotated_at_env.trim().is_empty() {
        Value::Null
    } else {
        std::env::var(rotated_at_env)
            .ok()
            .filter(|row| !row.trim().is_empty())
            .map(Value::String)
            .unwrap_or(Value::Null)
    };
    Some(json!({
        "ok": true,
        "value": value,
        "rotated_at": rotated_at,
        "provider_type": "env",
        "provider_ref": env,
        "external": true
    }))
}

fn provider_json_file(root: &Path, secret_id: &str, provider: &ProviderConfig) -> Option<Value> {
    let ProviderConfig::JsonFile {
        paths,
        field,
        rotated_at_field,
        ..
    } = provider
    else {
        return None;
    };
    for raw_path in paths {
        let resolved = resolve_template(root, raw_path, secret_id);
        let resolved_path = PathBuf::from(&resolved);
        if !resolved_path.exists() {
            continue;
        }
        let Ok(text) = fs::read_to_string(&resolved_path) else {
            continue;
        };
        let Ok(payload) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let Some(value) = get_path_value(&payload, field).and_then(Value::as_str) else {
            continue;
        };
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let rotated_at = get_path_value(&payload, rotated_at_field)
            .cloned()
            .unwrap_or(Value::Null);
        return Some(json!({
            "ok": true,
            "value": trimmed,
            "rotated_at": rotated_at,
            "provider_type": "json_file",
            "provider_ref": resolved,
            "external": false
        }));
    }
    None
}

fn provider_command(secret_id: &str, provider: &ProviderConfig) -> Option<Value> {
    let ProviderConfig::Command {
        command,
        parse_json,
        value_path,
        rotated_at_path,
        env,
        ..
    } = provider
    else {
        return None;
    };
    let mut command_builder = match command {
        CommandSpec::Argv(argv) if !argv.is_empty() => {
            let mut builder = Command::new(&argv[0]);
            builder.args(&argv[1..]);
            builder
        }
        CommandSpec::Shell(shell) => {
            let mut builder = Command::new("/bin/sh");
            builder.args(["-lc", shell]);
            builder
        }
        _ => return None,
    };
    command_builder.env("SECRET_ID", secret_id);
    command_builder.env("SECRET_BROKER_SECRET_ID", secret_id);
    for (key, value) in env {
        command_builder.env(key, value);
    }
    let output = command_builder.output().ok()?;
    if !output.status.success() {
        return Some(json!({
            "ok": false,
            "reason": "command_exit_nonzero",
            "code": output.status.code().unwrap_or(1),
            "stderr": String::from_utf8_lossy(&output.stderr).trim().chars().take(200).collect::<String>(),
        }));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Some(json!({
            "ok": false,
            "reason": "command_empty_stdout",
        }));
    }
    if !parse_json {
        return Some(json!({
            "ok": true,
            "value": stdout,
            "rotated_at": Value::Null,
            "provider_type": "command",
            "provider_ref": command_provider_ref(command),
            "external": true
        }));
    }
    let Ok(payload) = serde_json::from_str::<Value>(&stdout) else {
        return Some(json!({
            "ok": false,
            "reason": "command_json_invalid"
        }));
    };
    let value = get_path_value(&payload, value_path)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if value.is_empty() {
        return Some(json!({
            "ok": false,
            "reason": "command_value_missing"
        }));
    }
    let rotated_at = get_path_value(&payload, rotated_at_path)
        .cloned()
        .unwrap_or(Value::Null);
    Some(json!({
        "ok": true,
        "value": value,
        "rotated_at": rotated_at,
        "provider_type": "command",
        "provider_ref": command_provider_ref(command),
        "external": true
    }))
}

fn evaluate_rotation(
    rotation_cfg: &RotationConfig,
    rotated_at: Option<&Value>,
    now_ms: i64,
) -> RotationHealth {
    let rotated_at_ms = rotated_at.and_then(parse_ts_ms);
    if rotated_at_ms.is_none() {
        return RotationHealth {
            status: if rotation_cfg.require_rotated_at {
                "critical".to_string()
            } else {
                "unknown".to_string()
            },
            reason: "rotated_at_missing".to_string(),
            rotated_at: None,
            age_days: None,
            warn_after_days: rotation_cfg.warn_after_days,
            max_after_days: rotation_cfg.max_after_days,
            require_rotated_at: rotation_cfg.require_rotated_at,
            enforce_on_issue: rotation_cfg.enforce_on_issue,
        };
    }
    let rotated_at_ms = rotated_at_ms.unwrap_or(now_ms);
    let age_days = ((now_ms - rotated_at_ms).max(0) as f64) / 86_400_000f64;
    let (status, reason) = if age_days > rotation_cfg.max_after_days {
        ("critical", "rotation_age_exceeded")
    } else if age_days > rotation_cfg.warn_after_days {
        ("warn", "rotation_age_warning")
    } else {
        ("ok", "rotation_fresh")
    };
    RotationHealth {
        status: status.to_string(),
        reason: reason.to_string(),
        rotated_at: Some(iso_from_ms(rotated_at_ms)),
        age_days: Some((age_days * 1000.0).round() / 1000.0),
        warn_after_days: rotation_cfg.warn_after_days,
        max_after_days: rotation_cfg.max_after_days,
        require_rotated_at: rotation_cfg.require_rotated_at,
        enforce_on_issue: rotation_cfg.enforce_on_issue,
    }
}

fn load_secret_by_id(
    root: &Path,
    payload: &Map<String, Value>,
    policy: &SecretBrokerPolicy,
    audit_path: &Path,
    with_audit: bool,
) -> LoadedSecret {
    let secret_id = text(payload.get("secret_id"), 160);
    let now = now_ms(payload);
    let Some(spec) = policy.secrets.get(&secret_id) else {
        return LoadedSecret {
            ok: false,
            secret_id,
            error: Some("secret_id_unsupported".to_string()),
            ..LoadedSecret::default()
        };
    };
    let mut provider_errors = Vec::new();
    for provider in &spec.providers {
        let enabled = match provider {
            ProviderConfig::Env { enabled, .. }
            | ProviderConfig::JsonFile { enabled, .. }
            | ProviderConfig::Command { enabled, .. } => *enabled,
        };
        if !enabled {
            continue;
        }
        let result = match provider {
            ProviderConfig::Env { .. } => provider_env(provider),
            ProviderConfig::JsonFile { .. } => provider_json_file(root, &secret_id, provider),
            ProviderConfig::Command { .. } => provider_command(&secret_id, provider),
        };
        let Some(result) = result else {
            provider_errors.push(json!({
                "provider_type": provider_type_name(provider),
                "reason": "provider_failed"
            }));
            continue;
        };
        if result.get("ok").and_then(Value::as_bool) != Some(true) {
            provider_errors.push(json!({
                "provider_type": result.get("provider_type").and_then(Value::as_str).unwrap_or(provider_type_name(provider)),
                "reason": result.get("reason").and_then(Value::as_str).unwrap_or("provider_failed"),
                "code": result.get("code").cloned().unwrap_or(Value::Null),
                "ref": result.get("provider_ref").cloned().unwrap_or(Value::Null)
            }));
            continue;
        }
        let value = text(result.get("value"), 8192);
        if value.is_empty() {
            provider_errors.push(json!({
                "provider_type": result.get("provider_type").and_then(Value::as_str).unwrap_or("unknown"),
                "reason": "value_empty"
            }));
            continue;
        }
        let rotation = evaluate_rotation(&spec.rotation, result.get("rotated_at"), now);
        let backend = ResolvedBackend {
            provider_type: text(result.get("provider_type"), 64),
            provider_ref: {
                let v = text(result.get("provider_ref"), 240);
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
            external: bool_value(result.get("external"), false),
        };
        if with_audit {
            let _ = append_audit(
                audit_path,
                json!({
                    "type": "secret_value_loaded",
                    "secret_id": secret_id,
                    "provider_type": backend.provider_type,
                    "provider_ref": if policy.include_backend_details { backend.provider_ref.clone() } else { None },
                    "external_backend": backend.external,
                    "value_hash": sha16(&value),
                    "rotation_status": rotation.status,
                    "rotation_age_days": rotation.age_days,
                }),
            );
        }
        return LoadedSecret {
            ok: true,
            secret_id: secret_id.clone(),
            value: value.clone(),
            value_hash: sha16(&value),
            backend: Some(backend),
            rotation: Some(rotation),
            error: None,
            provider_errors: Vec::new(),
        };
    }
    if with_audit {
        let _ = append_audit(
            audit_path,
            json!({
                "type": "secret_value_load_failed",
                "secret_id": secret_id,
                "reason": "all_providers_failed",
                "provider_errors": provider_errors,
            }),
        );
    }
    LoadedSecret {
        ok: false,
        secret_id,
        error: Some("secret_value_missing".to_string()),
        provider_errors,
        ..LoadedSecret::default()
    }
}
