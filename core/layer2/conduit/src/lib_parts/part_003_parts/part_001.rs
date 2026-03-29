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
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
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

pub fn validate_command<P: PolicyGate>(
    envelope: &CommandEnvelope,
    policy: &P,
    security: &mut ConduitSecurityContext,
) -> ValidationReceipt {
    if envelope.schema_id != CONDUIT_SCHEMA_ID || envelope.schema_version != CONDUIT_SCHEMA_VERSION
    {
        return fail_closed_receipt(
            "conduit_schema_mismatch",
            "policy_not_evaluated",
            "security_not_evaluated",
        );
    }

    let structural = validate_structure(&envelope.command);
    if let Some(reason) = structural {
        return fail_closed_receipt(reason, "policy_not_evaluated", "security_not_evaluated");
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
        );
    }

    let security_receipt_hash = match security.validate(envelope) {
        Ok(receipt_hash) => receipt_hash,
        Err(err) => {
            return fail_closed_receipt(err.to_string(), policy_receipt_hash, "security_denied");
        }
    };

    success_receipt(policy_receipt_hash, security_receipt_hash)
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
