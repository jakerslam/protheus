const OPENSHELL_POLICY_SCHEMA_ID: &str = "infring_openshell_policy_v1";

#[derive(Clone, Debug)]
struct OpenShellPolicy {
    mode: String,
    sandbox_enabled: bool,
    conduit_enforced: bool,
    filesystem_allow: Vec<String>,
    filesystem_deny: Vec<String>,
    network_allow_hosts: Vec<String>,
    network_deny_hosts: Vec<String>,
    credential_allow: Vec<String>,
    privacy_redact_patterns: Vec<String>,
    privacy_block_patterns: Vec<String>,
}

fn openshell_state_dir(root: &Path) -> PathBuf {
    state_dir(root).join("openshell")
}

fn openshell_latest_path(root: &Path) -> PathBuf {
    openshell_state_dir(root).join("latest.json")
}

fn openshell_history_path(root: &Path) -> PathBuf {
    openshell_state_dir(root).join("history.jsonl")
}

fn openshell_events_path(root: &Path) -> PathBuf {
    openshell_state_dir(root).join("events.jsonl")
}

fn openshell_signed_policy_path(root: &Path) -> PathBuf {
    openshell_state_dir(root).join("policy.signed.json")
}

fn openshell_judicial_lock_path(root: &Path) -> PathBuf {
    state_dir(root).join("judicial_lock.json")
}

fn default_openshell_policy_path(root: &Path) -> PathBuf {
    root.join("client")
        .join("runtime")
        .join("config")
        .join("openshell_policy.yaml")
}

fn resolve_openshell_policy_path(root: &Path, argv: &[String]) -> PathBuf {
    if let Some(raw) = parse_flag(argv, "policy") {
        let path = PathBuf::from(raw.trim());
        if path.is_absolute() {
            path
        } else {
            root.join(path)
        }
    } else {
        default_openshell_policy_path(root)
    }
}

fn parse_token_list(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|row| clean(row, 256).to_ascii_lowercase())
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>()
}

fn parse_openshell_policy(raw: &Value) -> (OpenShellPolicy, Vec<String>) {
    let mut errors = Vec::<String>::new();
    let mode = raw
        .get("mode")
        .and_then(Value::as_str)
        .map(|v| clean(v, 32).to_ascii_lowercase())
        .unwrap_or_else(|| "production".to_string());
    if !matches!(mode.as_str(), "production" | "simulation") {
        errors.push("openshell_mode_invalid".to_string());
    }
    if raw.get("kind").and_then(Value::as_str) != Some("openshell_policy") {
        errors.push("openshell_kind_invalid".to_string());
    }
    if raw.get("version").and_then(Value::as_str) != Some("v1") {
        errors.push("openshell_version_invalid".to_string());
    }

    let sandbox = raw
        .get("sandbox")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let conduit = raw
        .get("conduit")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let filesystem = raw
        .get("filesystem")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let network = raw
        .get("network")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let credentials = raw
        .get("credentials")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let privacy = raw
        .get("privacy")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let sandbox_enabled = sandbox
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let conduit_enforced = conduit
        .get("enforce")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let filesystem_allow = parse_token_list(filesystem.get("allow").or_else(|| filesystem.get("allow_paths")));
    if filesystem_allow.is_empty() {
        errors.push("openshell_filesystem_allow_required".to_string());
    }
    let filesystem_deny = parse_token_list(filesystem.get("deny").or_else(|| filesystem.get("deny_paths")));

    let network_allow_hosts = parse_token_list(network.get("allow_hosts"));
    let network_deny_hosts = parse_token_list(network.get("deny_hosts"));

    let credential_allow = parse_token_list(credentials.get("allow"));
    if credentials.get("required").and_then(Value::as_bool) == Some(true) && credential_allow.is_empty() {
        errors.push("openshell_credentials_allow_required".to_string());
    }

    let privacy_redact_patterns = parse_token_list(privacy.get("redact_patterns"));
    let privacy_block_patterns = parse_token_list(privacy.get("block_patterns"));
    if privacy_redact_patterns.is_empty() && privacy_block_patterns.is_empty() {
        errors.push("openshell_privacy_patterns_required".to_string());
    }

    (
        OpenShellPolicy {
            mode,
            sandbox_enabled,
            conduit_enforced,
            filesystem_allow,
            filesystem_deny,
            network_allow_hosts,
            network_deny_hosts,
            credential_allow,
            privacy_redact_patterns,
            privacy_block_patterns,
        },
        errors,
    )
}

fn load_openshell_policy(path: &Path) -> Result<(Value, OpenShellPolicy, Vec<String>), String> {
    let raw_text = fs::read_to_string(path).map_err(|e| format!("openshell_policy_read_failed:{e}"))?;
    let parsed_yaml: serde_yaml::Value =
        serde_yaml::from_str(&raw_text).map_err(|e| format!("openshell_policy_yaml_parse_failed:{e}"))?;
    let parsed_json =
        serde_json::to_value(parsed_yaml).map_err(|e| format!("openshell_policy_yaml_encode_failed:{e}"))?;
    let (policy, errors) = parse_openshell_policy(&parsed_json);
    Ok((parsed_json, policy, errors))
}

fn host_allowed(policy: &OpenShellPolicy, host: &str) -> bool {
    let lowered = clean(host, 200).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if policy
        .network_deny_hosts
        .iter()
        .any(|deny| !deny.is_empty() && lowered.contains(deny))
    {
        return false;
    }
    if policy.network_allow_hosts.is_empty() {
        return true;
    }
    policy
        .network_allow_hosts
        .iter()
        .any(|allow| !allow.is_empty() && lowered.contains(allow))
}

fn path_allowed(policy: &OpenShellPolicy, file_path: &str) -> bool {
    let lowered = clean(file_path, 400).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if policy
        .filesystem_deny
        .iter()
        .any(|deny| !deny.is_empty() && lowered.starts_with(deny))
    {
        return false;
    }
    if policy.filesystem_allow.is_empty() {
        return false;
    }
    policy
        .filesystem_allow
        .iter()
        .any(|allow| !allow.is_empty() && lowered.starts_with(allow))
}

fn redact_payload(payload: &str, patterns: &[String]) -> String {
    let mut out = payload.to_string();
    for pattern in patterns {
        if pattern.is_empty() {
            continue;
        }
        let escaped = regex::escape(pattern);
        if let Ok(re) = regex::RegexBuilder::new(&escaped)
            .case_insensitive(true)
            .build()
        {
            out = re.replace_all(&out, "[REDACTED]").to_string();
        }
    }
    out
}

fn has_sensitive_pattern(payload: &str, patterns: &[String]) -> bool {
    let lowered = payload.to_ascii_lowercase();
    patterns
        .iter()
        .any(|pattern| !pattern.is_empty() && lowered.contains(pattern))
}

fn openshell_backend_available(argv: &[String]) -> (bool, String) {
    let backend_bin = parse_flag(argv, "backend-bin").unwrap_or_else(|| "openshell".to_string());
    match Command::new(&backend_bin).arg("--version").output() {
        Ok(output) => (output.status.success(), backend_bin),
        Err(_) => (false, backend_bin),
    }
}

fn persist_openshell_artifacts(root: &Path, command: &str, payload: &Value) {
    let latest = openshell_latest_path(root);
    let history = openshell_history_path(root);
    let events = openshell_events_path(root);
    write_json(&latest, payload);
    append_jsonl(&history, payload);
    append_jsonl(
        &events,
        &json!({
            "ts": now_iso(),
            "command": command,
            "ok": payload.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "violation_codes": payload.get("violation_codes").cloned().unwrap_or_else(|| json!([]))
        }),
    );
}

fn maybe_trigger_judicial_lock(root: &Path, strict: bool, command: &str, violation_codes: &[String]) -> bool {
    let triggered = strict && !violation_codes.is_empty();
    let payload = json!({
        "type": "security_plane_judicial_lock",
        "active": triggered,
        "command": command,
        "ts": now_iso(),
        "violation_codes": violation_codes,
    });
    write_json(&openshell_judicial_lock_path(root), &payload);
    triggered
}

fn run_openshell_runtime_command(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    let command = parse_subcommand(argv, "status");
    let policy_path = resolve_openshell_policy_path(root, argv);
    let (backend_available, backend_bin) = openshell_backend_available(argv);
    let require_backend =
        parse_bool(parse_flag(argv, "require-openshell"), false)
            || parse_bool(parse_flag(argv, "require-backend"), false);

    let mut violation_codes = Vec::<String>::new();
    let mut policy_errors = Vec::<String>::new();
    let mut loaded_policy = json!({});
    let mut policy_digest = String::new();
    let mut parsed_policy: Option<OpenShellPolicy> = None;

    match load_openshell_policy(&policy_path) {
        Ok((raw, parsed, errors)) => {
            loaded_policy = raw;
            policy_digest = crate::deterministic_receipt_hash(&loaded_policy);
            policy_errors = errors;
            parsed_policy = Some(parsed);
        }
        Err(error) => {
            policy_errors.push(error);
        }
    }
    if !policy_errors.is_empty() {
        violation_codes.push("openshell_policy_invalid".to_string());
    }
    if require_backend && !backend_available {
        violation_codes.push("openshell_backend_unavailable".to_string());
    }

    let mut route = json!({"mode": "idle"});
    let mut sanitized_payload = Value::Null;
    if let Some(policy) = parsed_policy.clone() {
        if command == "enforce" || command == "run-agent" || command == "privacy-route" {
            let fs_path = parse_flag(argv, "file").or_else(|| parse_flag(argv, "path"));
            let host = parse_flag(argv, "host").or_else(|| parse_flag(argv, "network-host"));
            let credential = parse_flag(argv, "credential");
            let outbound_payload = parse_flag(argv, "payload")
                .or_else(|| parse_flag(argv, "outbound"))
                .unwrap_or_default();

            if policy.sandbox_enabled {
                if let Some(path) = fs_path.as_deref() {
                    if !path_allowed(&policy, path) {
                        violation_codes.push("filesystem_denied".to_string());
                    }
                }
                if let Some(h) = host.as_deref() {
                    if !host_allowed(&policy, h) {
                        violation_codes.push("network_egress_denied".to_string());
                    }
                }
                if let Some(cref) = credential.as_deref() {
                    let normalized = clean(cref, 160).to_ascii_lowercase();
                    if !normalized.is_empty()
                        && !policy.credential_allow.iter().any(|allowed| allowed == &normalized)
                    {
                        violation_codes.push("credential_access_denied".to_string());
                    }
                }
            }

            if !outbound_payload.trim().is_empty() {
                if has_sensitive_pattern(&outbound_payload, &policy.privacy_block_patterns) {
                    violation_codes.push("privacy_router_blocked_payload".to_string());
                }
                let redacted = redact_payload(&outbound_payload, &policy.privacy_redact_patterns);
                sanitized_payload = json!({
                    "raw_sha256": hash_text(&outbound_payload),
                    "sanitized_sha256": hash_text(&redacted),
                    "sanitized_preview": clean(&redacted, 240),
                    "redacted": redacted != outbound_payload
                });
            }

            route = json!({
                "mode": command,
                "conduit_enforced": policy.conduit_enforced,
                "sandbox_enabled": policy.sandbox_enabled,
                "filesystem_path": fs_path,
                "host": host,
                "credential": credential.map(|v| clean(v, 40)),
            });
        }
    }

    let judicial_lock_triggered =
        maybe_trigger_judicial_lock(root, strict, &command, &violation_codes);
    let ok = violation_codes.is_empty();

    let mut payload = json!({
        "ok": if strict { ok } else { true },
        "type": "security_plane_openshell_runtime",
        "lane": "core/layer1/security",
        "strict": strict,
        "command": command,
        "policy_path": policy_path.display().to_string(),
        "policy_schema_id": OPENSHELL_POLICY_SCHEMA_ID,
        "policy_digest": policy_digest,
        "policy_errors": policy_errors,
        "loaded_policy": loaded_policy,
        "openshell_backend": {
            "binary": backend_bin,
            "available": backend_available,
            "required": require_backend
        },
        "route": route,
        "sanitized_payload": sanitized_payload,
        "violation_codes": violation_codes,
        "judicial_lock": {
            "triggered": judicial_lock_triggered,
            "path": openshell_judicial_lock_path(root).display().to_string()
        },
        "claim_evidence": [{
            "id": "V6-SEC-OSHELL-001",
            "claim": "openshell_style_sandbox_and_yaml_policy_enforcement_harden_conduit_actions_with_fail_closed_verity_receipts",
            "evidence": {
                "policy_path": policy_path.display().to_string(),
                "backend_available": backend_available,
                "command": command
            }
        }]
    });

    if command == "validate-policy" && policy_errors.is_empty() {
        let signed_record = json!({
            "schema_id": OPENSHELL_POLICY_SCHEMA_ID,
            "policy_path": policy_path.display().to_string(),
            "mode": parsed_policy.as_ref().map(|v| v.mode.clone()).unwrap_or_else(|| "production".to_string()),
            "signed_at": now_iso(),
            "signature": crate::deterministic_receipt_hash(&json!({
                "schema_id": OPENSHELL_POLICY_SCHEMA_ID,
                "policy_digest": policy_digest
            })),
            "policy_digest": policy_digest,
        });
        write_json(&openshell_signed_policy_path(root), &signed_record);
        payload["signed_policy"] = signed_record;
    }

    payload["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&payload));
    persist_openshell_artifacts(root, &command, &payload);
    let exit = if strict && !ok { 2 } else { 0 };
    (payload, exit)
}
