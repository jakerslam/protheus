use conduit_security::{
    deterministic_hash, CapabilityToken, CapabilityTokenAuthority, MessageSigner, RateLimitPolicy,
    RateLimiter, SecurityError,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{self, BufRead, Write};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CONDUIT_SCHEMA_ID: &str = "protheus_conduit";
pub const CONDUIT_SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TsCommand {
    StartAgent {
        agent_id: String,
    },
    StopAgent {
        agent_id: String,
    },
    QueryReceiptChain {
        from_hash: Option<String>,
        limit: Option<u32>,
    },
    ListActiveAgents,
    GetSystemStatus,
    ApplyPolicyUpdate {
        patch_id: String,
        patch: Value,
    },
    InstallExtension {
        extension_id: String,
        wasm_sha256: String,
        capabilities: Vec<String>,
    },
}

pub const TS_COMMAND_TYPES: [&str; 7] = [
    "start_agent",
    "stop_agent",
    "query_receipt_chain",
    "list_active_agents",
    "get_system_status",
    "apply_policy_update",
    "install_extension",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RustEvent {
    AgentStarted { agent_id: String },
    AgentStopped { agent_id: String },
    ReceiptAdded { receipt_hash: String },
    SystemStatus { status: String, detail: Value },
    PolicyViolation { reason: String },
}

pub const RUST_EVENT_TYPES: [&str; 5] = [
    "agent_started",
    "agent_stopped",
    "receipt_added",
    "system_status",
    "policy_violation",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSecurityMetadata {
    pub client_id: String,
    pub key_id: String,
    pub nonce: String,
    pub signature: String,
    pub capability_token: CapabilityToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub schema_id: String,
    pub schema_version: String,
    pub request_id: String,
    pub ts_ms: u64,
    pub command: TsCommand,
    pub security: CommandSecurityMetadata,
}

impl CommandEnvelope {
    pub fn new(
        request_id: impl Into<String>,
        command: TsCommand,
        security: CommandSecurityMetadata,
    ) -> Self {
        Self {
            schema_id: CONDUIT_SCHEMA_ID.to_string(),
            schema_version: CONDUIT_SCHEMA_VERSION.to_string(),
            request_id: request_id.into(),
            ts_ms: now_ts_ms(),
            command,
            security,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub schema_id: String,
    pub schema_version: String,
    pub request_id: String,
    pub ts_ms: u64,
    pub event: RustEvent,
    pub validation: ValidationReceipt,
    pub crossing: CrossingReceipt,
    pub receipt_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossingDirection {
    TsToRust,
    RustToTs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossingReceipt {
    pub crossing_id: String,
    pub direction: CrossingDirection,
    pub command_type: String,
    pub deterministic_hash: String,
    pub ts_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReceipt {
    pub ok: bool,
    pub fail_closed: bool,
    pub reason: String,
    pub policy_receipt_hash: String,
    pub security_receipt_hash: String,
    pub receipt_hash: String,
}

pub trait PolicyGate {
    fn evaluate(&self, command: &TsCommand) -> PolicyDecision;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allow: bool,
    pub reason: String,
}

impl PolicyDecision {
    pub fn allow() -> Self {
        Self {
            allow: true,
            reason: "policy_allow".to_string(),
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            allow: false,
            reason: reason.into(),
        }
    }
}

pub struct FailClosedPolicy;

impl PolicyGate for FailClosedPolicy {
    fn evaluate(&self, _command: &TsCommand) -> PolicyDecision {
        PolicyDecision::deny("policy_gate_not_configured")
    }
}

pub struct AllowAllPolicy;

impl PolicyGate for AllowAllPolicy {
    fn evaluate(&self, _command: &TsCommand) -> PolicyDecision {
        PolicyDecision::allow()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConduitPolicy {
    pub constitution_path: String,
    pub guard_registry_path: String,
    pub required_constitution_markers: Vec<String>,
    pub required_guard_checks: Vec<String>,
    pub command_required_capabilities: BTreeMap<String, String>,
    pub allow_policy_update_prefixes: Vec<String>,
    pub rate_limit: RateLimitPolicy,
}

impl Default for ConduitPolicy {
    fn default() -> Self {
        let mut capabilities = BTreeMap::new();
        capabilities.insert("start_agent".to_string(), "agent.lifecycle".to_string());
        capabilities.insert("stop_agent".to_string(), "agent.lifecycle".to_string());
        capabilities.insert(
            "query_receipt_chain".to_string(),
            "receipt.read".to_string(),
        );
        capabilities.insert("list_active_agents".to_string(), "system.read".to_string());
        capabilities.insert("get_system_status".to_string(), "system.read".to_string());
        capabilities.insert(
            "apply_policy_update".to_string(),
            "policy.update".to_string(),
        );
        capabilities.insert(
            "install_extension".to_string(),
            "extension.install".to_string(),
        );

        Self {
            constitution_path: "AGENT-CONSTITUTION.md".to_string(),
            guard_registry_path: "config/guard_check_registry.json".to_string(),
            required_constitution_markers: vec![
                "Mind Sovereignty Covenant".to_string(),
                "RSI Guardrails".to_string(),
            ],
            required_guard_checks: vec![
                "contract_check".to_string(),
                "formal_invariant_engine".to_string(),
            ],
            command_required_capabilities: capabilities,
            allow_policy_update_prefixes: vec!["constitution_safe/".to_string()],
            rate_limit: RateLimitPolicy::default(),
        }
    }
}

impl ConduitPolicy {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
        let raw = fs::read_to_string(path)?;
        serde_json::from_str(&raw).map_err(invalid_data)
    }
}

pub struct RegistryPolicyGate {
    policy: ConduitPolicy,
    constitution_body: Option<String>,
    guard_check_ids: Option<std::collections::BTreeSet<String>>,
    bootstrap_error: Option<String>,
}

impl RegistryPolicyGate {
    pub fn new(policy: ConduitPolicy) -> Self {
        let mut gate = Self {
            policy,
            constitution_body: None,
            guard_check_ids: None,
            bootstrap_error: None,
        };
        gate.bootstrap();
        gate
    }

    pub fn policy(&self) -> &ConduitPolicy {
        &self.policy
    }

    fn bootstrap(&mut self) {
        let constitution_body = match fs::read_to_string(&self.policy.constitution_path) {
            Ok(body) => body,
            Err(_) => {
                self.bootstrap_error = Some("constitution_file_unavailable".to_string());
                return;
            }
        };

        let registry_body = match fs::read_to_string(&self.policy.guard_registry_path) {
            Ok(body) => body,
            Err(_) => {
                self.bootstrap_error = Some("guard_registry_unavailable".to_string());
                return;
            }
        };
        let parsed: Value = match serde_json::from_str(&registry_body) {
            Ok(parsed) => parsed,
            Err(_) => {
                self.bootstrap_error = Some("guard_registry_invalid_json".to_string());
                return;
            }
        };

        let checks = match parsed
            .pointer("/merge_guard/checks")
            .and_then(Value::as_array)
        {
            Some(checks) => checks,
            None => {
                self.bootstrap_error = Some("guard_registry_checks_missing".to_string());
                return;
            }
        };

        let mut found = std::collections::BTreeSet::new();
        for row in checks {
            if let Some(id) = row.get("id").and_then(Value::as_str) {
                found.insert(id.to_string());
            }
        }

        self.constitution_body = Some(constitution_body);
        self.guard_check_ids = Some(found);
    }

    fn validate_constitution(&self) -> Result<(), String> {
        let body = self
            .constitution_body
            .as_ref()
            .ok_or_else(|| "constitution_file_unavailable".to_string())?;
        for marker in &self.policy.required_constitution_markers {
            if !body.contains(marker) {
                return Err(format!("constitution_marker_missing:{marker}"));
            }
        }
        Ok(())
    }

    fn validate_guard_registry(&self) -> Result<(), String> {
        let checks = self
            .guard_check_ids
            .as_ref()
            .ok_or_else(|| "guard_registry_checks_missing".to_string())?;
        for required in &self.policy.required_guard_checks {
            if !checks.contains(required) {
                return Err(format!("guard_registry_required_check_missing:{required}"));
            }
        }
        Ok(())
    }

    fn validate_command_mapping(&self, command: &TsCommand) -> Result<(), String> {
        let command_type = command_type_name(command);
        if !self
            .policy
            .command_required_capabilities
            .contains_key(command_type)
        {
            return Err(format!(
                "policy_missing_command_capability_mapping:{command_type}"
            ));
        }
        if let TsCommand::ApplyPolicyUpdate { patch_id, .. } = command {
            if !self
                .policy
                .allow_policy_update_prefixes
                .iter()
                .any(|prefix| patch_id.starts_with(prefix))
            {
                return Err("policy_update_must_be_constitution_safe".to_string());
            }
        }
        Ok(())
    }
}

impl PolicyGate for RegistryPolicyGate {
    fn evaluate(&self, command: &TsCommand) -> PolicyDecision {
        if let Some(reason) = &self.bootstrap_error {
            return PolicyDecision::deny(reason.clone());
        }
        if let Err(reason) = self.validate_constitution() {
            return PolicyDecision::deny(reason);
        }
        if let Err(reason) = self.validate_guard_registry() {
            return PolicyDecision::deny(reason);
        }
        if let Err(reason) = self.validate_command_mapping(command) {
            return PolicyDecision::deny(reason);
        }
        PolicyDecision::allow()
    }
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
            TsCommand::StartAgent { agent_id } => RustEvent::AgentStarted {
                agent_id: agent_id.clone(),
            },
            TsCommand::StopAgent { agent_id } => RustEvent::AgentStopped {
                agent_id: agent_id.clone(),
            },
            TsCommand::QueryReceiptChain { .. } => RustEvent::ReceiptAdded {
                receipt_hash: "query_receipt_chain_ack".to_string(),
            },
            TsCommand::ListActiveAgents | TsCommand::GetSystemStatus => RustEvent::SystemStatus {
                status: "ok".to_string(),
                detail: serde_json::json!({"mode":"hosted"}),
            },
            TsCommand::ApplyPolicyUpdate { .. } => RustEvent::SystemStatus {
                status: "policy_update_accepted".to_string(),
                detail: serde_json::json!({"source":"conduit"}),
            },
            TsCommand::InstallExtension { extension_id, .. } => RustEvent::SystemStatus {
                status: "extension_install_accepted".to_string(),
                detail: serde_json::json!({"extension_id": extension_id}),
            },
        }
    }
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
        }
        TsCommand::ListActiveAgents | TsCommand::GetSystemStatus => {}
    }
    None
}

fn fail_closed_receipt(
    reason: impl Into<String>,
    policy_receipt_hash: impl Into<String>,
    security_receipt_hash: impl Into<String>,
) -> ValidationReceipt {
    let reason = reason.into();
    let policy_receipt_hash = policy_receipt_hash.into();
    let security_receipt_hash = security_receipt_hash.into();
    let payload = serde_json::json!({
        "ok": false,
        "fail_closed": true,
        "reason": reason,
        "policy_receipt_hash": policy_receipt_hash,
        "security_receipt_hash": security_receipt_hash,
    });
    ValidationReceipt {
        ok: false,
        fail_closed: true,
        reason,
        policy_receipt_hash,
        security_receipt_hash,
        receipt_hash: deterministic_receipt_hash(&payload),
    }
}

fn success_receipt(
    policy_receipt_hash: impl Into<String>,
    security_receipt_hash: impl Into<String>,
) -> ValidationReceipt {
    let policy_receipt_hash = policy_receipt_hash.into();
    let security_receipt_hash = security_receipt_hash.into();
    let payload = serde_json::json!({
        "ok": true,
        "fail_closed": false,
        "reason": "validated",
        "policy_receipt_hash": policy_receipt_hash,
        "security_receipt_hash": security_receipt_hash,
    });

    ValidationReceipt {
        ok: true,
        fail_closed: false,
        reason: "validated".to_string(),
        policy_receipt_hash,
        security_receipt_hash,
        receipt_hash: deterministic_receipt_hash(&payload),
    }
}

fn is_valid_sha256(raw: &str) -> bool {
    raw.len() == 64 && raw.chars().all(|ch| ch.is_ascii_hexdigit())
}

pub fn process_command<P: PolicyGate, H: CommandHandler>(
    envelope: &CommandEnvelope,
    policy: &P,
    security: &mut ConduitSecurityContext,
    handler: &mut H,
) -> ResponseEnvelope {
    let validation = validate_command(envelope, policy, security);
    let command_type = command_type_name(&envelope.command);

    let event = if validation.ok {
        handler.handle(&envelope.command)
    } else {
        RustEvent::PolicyViolation {
            reason: validation.reason.clone(),
        }
    };

    let crossing = CrossingReceipt {
        crossing_id: envelope.request_id.clone(),
        direction: CrossingDirection::TsToRust,
        command_type: command_type.to_string(),
        deterministic_hash: deterministic_receipt_hash(envelope),
        ts_ms: now_ts_ms(),
    };

    let mut response = ResponseEnvelope {
        schema_id: CONDUIT_SCHEMA_ID.to_string(),
        schema_version: CONDUIT_SCHEMA_VERSION.to_string(),
        request_id: envelope.request_id.clone(),
        ts_ms: now_ts_ms(),
        event,
        validation,
        crossing,
        receipt_hash: String::new(),
    };
    response.receipt_hash = deterministic_receipt_hash(&response);
    response
}

pub fn run_stdio_once<R: BufRead, W: Write, P: PolicyGate, H: CommandHandler>(
    mut reader: R,
    writer: &mut W,
    policy: &P,
    security: &mut ConduitSecurityContext,
    handler: &mut H,
) -> io::Result<bool> {
    let mut line = String::new();
    let read = reader.read_line(&mut line)?;
    if read == 0 {
        return Ok(false);
    }

    let parsed = serde_json::from_str::<CommandEnvelope>(&line).map_err(invalid_data)?;
    let response = process_command(&parsed, policy, security, handler);
    serde_json::to_writer(&mut *writer, &response).map_err(invalid_data)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(true)
}

#[cfg(unix)]
pub fn run_unix_socket_server<P: AsRef<std::path::Path>, G: PolicyGate, H: CommandHandler>(
    socket_path: P,
    policy: &G,
    security: &mut ConduitSecurityContext,
    handler: &mut H,
) -> io::Result<()> {
    use std::io::BufReader;
    use std::os::unix::net::UnixListener;

    let path = socket_path.as_ref();
    if path.exists() {
        fs::remove_file(path)?;
    }

    let listener = UnixListener::bind(path)?;
    let (stream, _) = listener.accept()?;
    let read_stream = stream.try_clone()?;
    let mut reader = BufReader::new(read_stream);
    let mut writer = stream;

    while run_stdio_once(&mut reader, &mut writer, policy, security, handler)? {}
    Ok(())
}

fn invalid_data(err: impl fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err.to_string())
}

fn command_type_name(command: &TsCommand) -> &'static str {
    match command {
        TsCommand::StartAgent { .. } => "start_agent",
        TsCommand::StopAgent { .. } => "stop_agent",
        TsCommand::QueryReceiptChain { .. } => "query_receipt_chain",
        TsCommand::ListActiveAgents => "list_active_agents",
        TsCommand::GetSystemStatus => "get_system_status",
        TsCommand::ApplyPolicyUpdate { .. } => "apply_policy_update",
        TsCommand::InstallExtension { .. } => "install_extension",
    }
}

struct SigningPayload<'a> {
    schema_id: &'a str,
    schema_version: &'a str,
    request_id: &'a str,
    ts_ms: u64,
    command: &'a TsCommand,
    client_id: &'a str,
    key_id: &'a str,
    nonce: &'a str,
    capability_token: &'a CapabilityToken,
}

fn signing_payload(input: SigningPayload<'_>) -> Value {
    serde_json::json!({
        "schema_id": input.schema_id,
        "schema_version": input.schema_version,
        "request_id": input.request_id,
        "ts_ms": input.ts_ms,
        "command": input.command,
        "security": {
            "client_id": input.client_id,
            "key_id": input.key_id,
            "nonce": input.nonce,
            "capability_token": input.capability_token,
        }
    })
}

fn canonical_json<T: Serialize>(value: &T) -> String {
    let json = serde_json::to_value(value).expect("serialization must succeed");
    let normalized = normalize_value(json);
    serde_json::to_string(&normalized).expect("canonical serialization must succeed")
}

fn normalize_value(value: Value) -> Value {
    match value {
        Value::Array(rows) => Value::Array(rows.into_iter().map(normalize_value).collect()),
        Value::Object(map) => {
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
            let mut out = Map::new();
            for (key, value) in entries {
                out.insert(key, normalize_value(value));
            }
            Value::Object(out)
        }
        other => other,
    }
}

fn now_ts_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        process_command, run_stdio_once, ConduitPolicy, ConduitSecurityContext, EchoCommandHandler,
        PolicyGate, RegistryPolicyGate, RUST_EVENT_TYPES, TS_COMMAND_TYPES,
    };
    use super::{CommandEnvelope, TsCommand};
    use conduit_security::{CapabilityTokenAuthority, MessageSigner, RateLimitPolicy, RateLimiter};
    use std::fs;
    use std::io::{BufReader, Cursor};
    use std::path::PathBuf;

    fn test_policy_paths() -> (PathBuf, PathBuf, tempfile::TempDir) {
        let temp = tempfile::tempdir().expect("tempdir");
        let constitution = temp.path().join("AGENT-CONSTITUTION.md");
        let guard_registry = temp.path().join("guard_check_registry.json");

        fs::write(&constitution, "Mind Sovereignty Covenant\nRSI Guardrails\n")
            .expect("write constitution");

        fs::write(
            &guard_registry,
            serde_json::json!({
                "merge_guard": {
                    "checks": [
                        {"id":"contract_check"},
                        {"id":"formal_invariant_engine"}
                    ]
                }
            })
            .to_string(),
        )
        .expect("write guard registry");

        (constitution, guard_registry, temp)
    }

    fn test_policy() -> ConduitPolicy {
        let (constitution, guard_registry, temp) = test_policy_paths();
        std::mem::forget(temp);
        ConduitPolicy {
            constitution_path: constitution.to_string_lossy().to_string(),
            guard_registry_path: guard_registry.to_string_lossy().to_string(),
            rate_limit: RateLimitPolicy {
                window_ms: 5_000,
                per_client_max: 10,
                per_client_command_max: 10,
            },
            ..ConduitPolicy::default()
        }
    }

    fn test_security(policy: &ConduitPolicy) -> ConduitSecurityContext {
        ConduitSecurityContext::new(
            MessageSigner::new("msg-k1", "msg-secret"),
            CapabilityTokenAuthority::new("tok-k1", "tok-secret"),
            RateLimiter::new(policy.rate_limit.clone()),
            policy.command_required_capabilities.clone(),
        )
    }

    fn signed_envelope(policy: &ConduitPolicy, command: TsCommand) -> CommandEnvelope {
        let security = test_security(policy);
        let request_id = "req-test";
        let ts_ms = 123;
        let security_metadata =
            security.mint_security_metadata("client-a", request_id, ts_ms, &command, 60_000);
        CommandEnvelope {
            schema_id: super::CONDUIT_SCHEMA_ID.to_string(),
            schema_version: super::CONDUIT_SCHEMA_VERSION.to_string(),
            request_id: request_id.to_string(),
            ts_ms,
            command,
            security: security_metadata,
        }
    }

    #[test]
    fn command_and_event_contract_counts_match_spec() {
        assert_eq!(TS_COMMAND_TYPES.len(), 7);
        assert_eq!(RUST_EVENT_TYPES.len(), 5);
    }

    #[test]
    fn secure_signed_command_passes_and_returns_receipts() {
        let policy = test_policy();
        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);
        let command = signed_envelope(
            &policy,
            TsCommand::StartAgent {
                agent_id: "agent-alpha".to_string(),
            },
        );

        let mut handler = EchoCommandHandler;
        let response = process_command(&command, &gate, &mut security, &mut handler);
        assert!(response.validation.ok);
        assert!(!response.validation.policy_receipt_hash.is_empty());
        assert!(!response.validation.security_receipt_hash.is_empty());
    }

    #[test]
    fn bad_signature_fails_closed() {
        let policy = test_policy();
        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);
        let mut command = signed_envelope(&policy, TsCommand::GetSystemStatus);
        command.security.signature = "deadbeef".to_string();

        let mut handler = EchoCommandHandler;
        let response = process_command(&command, &gate, &mut security, &mut handler);
        assert!(!response.validation.ok);
        assert!(response.validation.fail_closed);
        assert_eq!(response.validation.reason, "message_signature_invalid");
    }

    #[test]
    fn missing_scope_fails_closed() {
        let envelope_policy = test_policy();
        let command = signed_envelope(
            &envelope_policy,
            TsCommand::InstallExtension {
                extension_id: "ext-1".to_string(),
                wasm_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_string(),
                capabilities: vec!["metrics.read".to_string()],
            },
        );

        let mut runtime_policy = envelope_policy.clone();
        runtime_policy.command_required_capabilities.insert(
            "install_extension".to_string(),
            "extension.install.strict".to_string(),
        );

        let gate = RegistryPolicyGate::new(runtime_policy.clone());
        let mut security = test_security(&runtime_policy);

        let mut handler = EchoCommandHandler;
        let response = process_command(&command, &gate, &mut security, &mut handler);
        assert!(!response.validation.ok);
        assert!(response.validation.fail_closed);
        assert!(response
            .validation
            .reason
            .starts_with("capability_token_missing_scope"));
    }

    #[test]
    fn rate_limiting_fails_closed() {
        let mut policy = test_policy();
        policy.rate_limit = RateLimitPolicy {
            window_ms: 10_000,
            per_client_max: 2,
            per_client_command_max: 1,
        };

        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);
        let mut handler = EchoCommandHandler;

        let c1 = signed_envelope(&policy, TsCommand::GetSystemStatus);
        let c2 = signed_envelope(&policy, TsCommand::GetSystemStatus);

        let first = process_command(&c1, &gate, &mut security, &mut handler);
        assert!(first.validation.ok);

        let second = process_command(&c2, &gate, &mut security, &mut handler);
        assert!(!second.validation.ok);
        assert!(second.validation.reason.starts_with("rate_limited:"));
    }

    #[test]
    fn registry_policy_denies_when_constitution_missing_marker() {
        let temp = tempfile::tempdir().expect("tempdir");
        let constitution = temp.path().join("constitution.md");
        fs::write(&constitution, "missing markers").expect("constitution");

        let guard_registry = temp.path().join("guard_registry.json");
        fs::write(
            &guard_registry,
            serde_json::json!({"merge_guard":{"checks":[{"id":"contract_check"}]}}).to_string(),
        )
        .expect("guard registry");

        let policy = ConduitPolicy {
            constitution_path: constitution.to_string_lossy().to_string(),
            guard_registry_path: guard_registry.to_string_lossy().to_string(),
            ..ConduitPolicy::default()
        };
        let gate = RegistryPolicyGate::new(policy);

        let decision = gate.evaluate(&TsCommand::GetSystemStatus);
        assert!(!decision.allow);
        assert!(decision.reason.starts_with("constitution_marker_missing:"));
    }

    #[test]
    fn stdio_roundtrip_returns_json_response() {
        let policy = test_policy();
        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);

        let command = signed_envelope(
            &policy,
            TsCommand::StartAgent {
                agent_id: "agent-alpha".to_string(),
            },
        );

        let mut payload = serde_json::to_string(&command).expect("serialize command");
        payload.push('\n');

        let cursor = Cursor::new(payload.into_bytes());
        let reader = BufReader::new(cursor);
        let mut writer = Vec::new();
        let mut handler = EchoCommandHandler;

        let wrote = run_stdio_once(reader, &mut writer, &gate, &mut security, &mut handler)
            .expect("stdio call should succeed");
        assert!(wrote);

        let text = String::from_utf8(writer).expect("utf8 response");
        let response: super::ResponseEnvelope =
            serde_json::from_str(text.trim()).expect("json response");
        assert!(response.validation.ok);
        assert_eq!(response.request_id, "req-test");
    }
}
