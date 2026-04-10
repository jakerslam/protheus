#[derive(Debug, Deserialize)]
struct GuardRegistryCheck {
    id: Option<String>,
}
fn parse_guard_registry_check_ids(raw: &str) -> Result<std::collections::BTreeSet<String>, String> {
    let parsed: GuardRegistrySnapshot =
        serde_json::from_str(raw).map_err(|_| "guard_registry_invalid_json".to_string())?;
    let checks = parsed
        .merge_guard
        .and_then(|mg| mg.checks)
        .ok_or_else(|| "guard_registry_checks_missing".to_string())?;

    let mut ids = std::collections::BTreeSet::new();
    for row in checks {
        if let Some(id) = row.id {
            ids.insert(id);
        }
    }
    Ok(ids)
}

#[derive(Debug, Clone)]
pub struct ConduitSecurityContext {
    signer: MessageSigner,
    token_authority: CapabilityTokenAuthority,
    rate_limiter: RateLimiter,
    command_required_capabilities: BTreeMap<String, String>,
}

impl ConduitSecurityContext {
    pub fn new(
        signer: MessageSigner,
        token_authority: CapabilityTokenAuthority,
        rate_limiter: RateLimiter,
        command_required_capabilities: BTreeMap<String, String>,
    ) -> Self {
        Self {
            signer,
            token_authority,
            rate_limiter,
            command_required_capabilities,
        }
    }

    pub fn from_policy(
        policy: &ConduitPolicy,
        signing_key_id: impl Into<String>,
        signing_secret: impl Into<String>,
        token_key_id: impl Into<String>,
        token_secret: impl Into<String>,
    ) -> Self {
        Self {
            signer: MessageSigner::new(signing_key_id, signing_secret),
            token_authority: CapabilityTokenAuthority::new(token_key_id, token_secret),
            rate_limiter: RateLimiter::new(policy.rate_limit.clone()),
            command_required_capabilities: policy.command_required_capabilities.clone(),
        }
    }

    pub fn mint_security_metadata(
        &self,
        client_id: impl Into<String>,
        request_id: &str,
        ts_ms: u64,
        command: &TsCommand,
        token_ttl_ms: u64,
    ) -> CommandSecurityMetadata {
        let client_id = client_id.into();
        let command_type = command_type_name(command);
        let scope = self
            .command_required_capabilities
            .get(command_type)
            .cloned()
            .unwrap_or_else(|| "system.read".to_string());
        let issued_at_ms = now_ts_ms();
        let token = self.token_authority.mint(
            format!("tok-{request_id}-{issued_at_ms}"),
            client_id.clone(),
            vec![scope],
            issued_at_ms,
            issued_at_ms.saturating_add(token_ttl_ms),
        );

        let nonce = format!("nonce-{request_id}-{issued_at_ms}");
        let payload = signing_payload(SigningPayload {
            schema_id: CONDUIT_SCHEMA_ID,
            schema_version: CONDUIT_SCHEMA_VERSION,
            request_id,
            ts_ms,
            command,
            client_id: &client_id,
            key_id: self.signer.key_id(),
            nonce: &nonce,
            capability_token: &token,
        });

        let signature = self.signer.sign_value(&payload);
        CommandSecurityMetadata {
            client_id,
            key_id: self.signer.key_id().to_string(),
            nonce,
            signature,
            capability_token: token,
        }
    }

    pub fn validate(&mut self, envelope: &CommandEnvelope) -> Result<String, SecurityError> {
        if envelope.security.key_id != self.signer.key_id() {
            return Err(SecurityError::SignatureInvalid);
        }

        let payload = signing_payload(SigningPayload {
            schema_id: &envelope.schema_id,
            schema_version: &envelope.schema_version,
            request_id: &envelope.request_id,
            ts_ms: envelope.ts_ms,
            command: &envelope.command,
            client_id: &envelope.security.client_id,
            key_id: &envelope.security.key_id,
            nonce: &envelope.security.nonce,
            capability_token: &envelope.security.capability_token,
        });

        if !self
            .signer
            .verify_value(&payload, &envelope.security.signature)
        {
            return Err(SecurityError::SignatureInvalid);
        }

        let command_type = command_type_name(&envelope.command);
        let required_scope = self
            .command_required_capabilities
            .get(command_type)
            .ok_or_else(|| SecurityError::CapabilityTokenMissingScope(command_type.to_string()))?
            .clone();

        self.token_authority.validate(
            &envelope.security.capability_token,
            now_ts_ms(),
            &required_scope,
        )?;

        self.rate_limiter
            .allow(&envelope.security.client_id, command_type, envelope.ts_ms)?;

        let receipt = serde_json::json!({
            "allow": true,
            "command_type": command_type,
            "client_id": envelope.security.client_id,
            "required_scope": required_scope,
            "token_key_id": self.token_authority.key_id(),
            "signing_key_id": self.signer.key_id()
        });
        Ok(deterministic_hash(&receipt))
    }
}

pub trait CommandHandler {
    fn handle(&mut self, command: &TsCommand) -> RustEvent;
}

#[derive(Debug, Default)]
pub struct EchoCommandHandler;

impl CommandHandler for EchoCommandHandler {
    fn handle(&mut self, command: &TsCommand) -> RustEvent {
        match command {
            TsCommand::StartAgent { agent_id } => RustEvent::AgentLifecycle {
                state: AgentLifecycleState::Started,
                agent_id: agent_id.clone(),
            },
            TsCommand::StopAgent { agent_id } => RustEvent::AgentLifecycle {
                state: AgentLifecycleState::Stopped,
                agent_id: agent_id.clone(),
            },
            TsCommand::QueryReceiptChain { .. } => RustEvent::ReceiptAdded {
                receipt_hash: "query_receipt_chain_ack".to_string(),
            },
            TsCommand::ListActiveAgents | TsCommand::GetSystemStatus => {
                let root = repo_root_from_current_dir();
                let cockpit_context = load_cockpit_summary(&root);
                let plugin_runtime = run_plugin_runtime_autoheal(&root, "status_poll");
                RustEvent::SystemFeedback {
                    status: "ok".to_string(),
                    detail: serde_json::json!({
                        "mode":"hosted",
                        "cockpit_context": cockpit_context,
                        "plugin_runtime": plugin_runtime
                    }),
                    violation_reason: None,
                }
            }
            TsCommand::ApplyPolicyUpdate { .. } => RustEvent::SystemFeedback {
                status: "policy_update_accepted".to_string(),
                detail: serde_json::json!({"source":"conduit"}),
                violation_reason: None,
            },
            TsCommand::InstallExtension {
                extension_id,
                wasm_sha256,
                capabilities,
                plugin_type,
                version,
                wasm_component_path,
                signature,
                provenance,
                recovery_max_attempts,
                recovery_backoff_ms,
            } => {
                let root = repo_root_from_current_dir();
                match register_extension_runtime(
                    &root,
                    RegisterExtensionInput {
                        extension_id: extension_id.clone(),
                        wasm_sha256: wasm_sha256.clone(),
                        capabilities: capabilities.clone(),
                        plugin_type: plugin_type.clone(),
                        version: version.clone(),
                        wasm_component_path: wasm_component_path.clone(),
                        signature: signature.clone(),
                        provenance: provenance.clone(),
                        recovery_max_attempts: *recovery_max_attempts,
                        recovery_backoff_ms: *recovery_backoff_ms,
                    },
                ) {
                    Ok(plugin_runtime) => RustEvent::SystemFeedback {
                        status: "extension_install_accepted".to_string(),
                        detail: serde_json::json!({
                            "extension_id": extension_id,
                            "plugin_runtime": plugin_runtime
                        }),
                        violation_reason: None,
                    },
                    Err(err) => RustEvent::SystemFeedback {
                        status: "extension_install_failed".to_string(),
                        detail: serde_json::json!({
                            "extension_id": extension_id,
                            "error": err
                        }),
                        violation_reason: Some("extension_runtime_registration_failed".to_string()),
                    },
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct KernelLaneCommandHandler;

impl CommandHandler for KernelLaneCommandHandler {
    fn handle(&mut self, command: &TsCommand) -> RustEvent {
        match command {
            TsCommand::StartAgent { agent_id } => {
                match decode_edge_bridge_message(agent_id) {
                    Ok(Some(message)) => return execute_edge_bridge_message(message),
                    Ok(None) => {
                        if agent_id.starts_with("lane:") {
                            let lane_receipt = build_legacy_lane_receipt(
                                agent_id.strip_prefix("lane:").unwrap_or_default(),
                            );
                            let status = if lane_receipt
                                .get("ok")
                                .and_then(Value::as_bool)
                                .unwrap_or(false)
                            {
                                "legacy_lane_receipt"
                            } else {
                                "legacy_lane_error"
                            };
                            return RustEvent::SystemFeedback {
                                status: status.to_string(),
                                detail: serde_json::json!({ "lane_receipt": lane_receipt }),
                                violation_reason: None,
                            };
                        }
                    }
                    Err(reason) => {
                        let detail = serde_json::json!({
                            "ok": false,
                            "type": "edge_bridge_error",
                            "reason": reason,
                            "receipt_hash": deterministic_receipt_hash(&serde_json::json!({
                                "type": "edge_bridge_error",
                                "reason": reason
                            }))
                        });
                        return RustEvent::SystemFeedback {
                            status: "edge_bridge_error".to_string(),
                            detail,
                            violation_reason: Some("edge_bridge_parse_failed".to_string()),
                        };
                    }
                }
                fallback_command_handle(command)
            }
            _ => fallback_command_handle(command),
        }
    }
}

fn fallback_command_handle(command: &TsCommand) -> RustEvent {
    let mut fallback = EchoCommandHandler;
    fallback.handle(command)
}

fn with_receipt_hash(mut payload: Value) -> Value {
    payload["receipt_hash"] = Value::String(deterministic_receipt_hash(&payload));
    payload
}

fn build_legacy_lane_receipt(raw_lane_id: &str) -> Value {
    let lane_id = clean_lane_id(raw_lane_id);
    if lane_id.is_empty() {
        return build_legacy_lane_error(raw_lane_id, "lane_id_missing_or_invalid");
    }

    let ts_ms = now_ts_ms();
    let lane_hash_seed = serde_json::json!({
        "lane": lane_id,
        "ts_ms": ts_ms,
        "type": "legacy_retired_lane",
    });
    let lane_hash_full = deterministic_receipt_hash(&lane_hash_seed);
    let lane_hash = lane_hash_full.chars().take(32).collect::<String>();

    with_receipt_hash(serde_json::json!({
        "ok": true,
        "type": "legacy_retired_lane",
        "lane_id": lane_id,
        "ts_ms": ts_ms,
        "lane_hash": lane_hash,
        "contract": {
            "deterministic": true,
            "reversible": true,
            "receipt_ready": true,
            "migrated_to_rust": true
        }
    }))
}

fn build_legacy_lane_error(raw_lane_id: &str, reason: &str) -> Value {
    with_receipt_hash(serde_json::json!({
        "ok": false,
        "type": "legacy_retired_lane_cli_error",
        "lane_id": clean_lane_id(raw_lane_id),
        "error": reason,
        "ts_ms": now_ts_ms(),
    }))
}

fn decode_edge_bridge_message(agent_id: &str) -> Result<Option<EdgeBridgeMessage>, String> {
    let trimmed = agent_id.trim();
    if trimmed.eq_ignore_ascii_case("edge_status") {
        return Ok(Some(EdgeBridgeMessage::EdgeStatus { probe: None }));
    }
    if let Some(prompt) = trimmed.strip_prefix("edge_inference:") {
        return Ok(Some(EdgeBridgeMessage::EdgeInference {
            prompt: prompt.to_string(),
            max_tokens: Some(64),
        }));
    }
    if let Some(raw_json) = trimmed.strip_prefix("edge_json:") {
        let parsed = serde_json::from_str::<EdgeBridgeMessage>(raw_json)
            .map_err(|err| format!("edge_bridge_json_invalid:{err}"))?;
        return Ok(Some(parsed));
    }
    Ok(None)
}
