// SPDX-License-Identifier: Apache-2.0
use protheus_nexus_core_v1::{
    conduit_deterministic_hash as deterministic_hash, CapabilityToken,
    CapabilityTokenAuthority, MessageSigner, RateLimitPolicy, RateLimiter, SecurityError,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use wait_timeout::ChildExt;

#[path = "../runtime_paths.rs"]

mod runtime_paths;
pub const CONDUIT_SCHEMA_ID: &str = "protheus_conduit";
pub const CONDUIT_SCHEMA_VERSION: &str = "1.0";
pub const MAX_CONDUIT_MESSAGE_TYPES: usize = 10;

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
        plugin_type: Option<String>,
        version: Option<String>,
        wasm_component_path: Option<String>,
        signature: Option<String>,
        provenance: Option<String>,
        recovery_max_attempts: Option<u32>,
        recovery_backoff_ms: Option<u64>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentLifecycleState {
    Started,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RustEvent {
    AgentLifecycle {
        state: AgentLifecycleState,
        agent_id: String,
    },
    ReceiptAdded {
        receipt_hash: String,
    },
    SystemFeedback {
        status: String,
        detail: Value,
        violation_reason: Option<String>,
    },
}

pub const RUST_EVENT_TYPES: [&str; 3] = ["agent_lifecycle", "receipt_added", "system_feedback"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EdgeBridgeMessage {
    EdgeInference {
        prompt: String,
        max_tokens: Option<u32>,
    },
    EdgeStatus {
        probe: Option<String>,
    },
    SpineCommand {
        args: Vec<String>,
        run_context: Option<String>,
    },
    AttentionCommand {
        args: Vec<String>,
    },
    PersonaAmbientCommand {
        args: Vec<String>,
    },
    DopamineAmbientCommand {
        args: Vec<String>,
    },
    MemoryAmbientCommand {
        args: Vec<String>,
    },
    OpsDomainCommand {
        domain: String,
        args: Vec<String>,
        run_context: Option<String>,
    },
}

pub const EDGE_BRIDGE_MESSAGE_TYPES: [&str; 8] = [
    "edge_inference",
    "edge_status",
    "spine_command",
    "attention_command",
    "persona_ambient_command",
    "dopamine_ambient_command",
    "memory_ambient_command",
    "ops_domain_command",
];

fn default_bridge_message_budget_max() -> usize {
    MAX_CONDUIT_MESSAGE_TYPES
}

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
    pub timestamp_drift_ms: i64,
    pub mode: String,
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
    #[serde(default = "default_bridge_message_budget_max")]
    pub bridge_message_budget_max: usize,
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
            constitution_path: "docs/workspace/AGENT-CONSTITUTION.md".to_string(),
            guard_registry_path: "client/runtime/config/guard_check_registry.json".to_string(),
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
            bridge_message_budget_max: MAX_CONDUIT_MESSAGE_TYPES,
        }
    }
}

impl ConduitPolicy {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
        let raw = fs::read_to_string(path)?;
        serde_json::from_str(&raw).map_err(invalid_data)
    }
}

pub fn conduit_message_contract_count() -> usize {
    TS_COMMAND_TYPES.len() + RUST_EVENT_TYPES.len()
}

pub fn validate_conduit_contract_budget(max_budget: usize) -> Result<(), String> {
    if max_budget == 0 {
        return Err("conduit_message_budget_invalid_zero".to_string());
    }
    let count = conduit_message_contract_count();
    if count > max_budget {
        return Err(format!(
            "conduit_message_budget_exceeded:{count}>{max_budget}"
        ));
    }
    Ok(())
}

pub struct RegistryPolicyGate {
    policy: ConduitPolicy,
    bootstrap_error: Option<String>,
}

impl RegistryPolicyGate {
    pub fn new(policy: ConduitPolicy) -> Self {
        let mut gate = Self {
            policy,
            bootstrap_error: None,
        };
        gate.bootstrap();
        gate
    }

    pub fn policy(&self) -> &ConduitPolicy {
        &self.policy
    }

    fn bootstrap(&mut self) {
        if let Err(reason) = validate_conduit_contract_budget(self.policy.bridge_message_budget_max)
        {
            self.bootstrap_error = Some(reason);
            return;
        }

        if self.policy.command_required_capabilities.len() != TS_COMMAND_TYPES.len() {
            self.bootstrap_error =
                Some("command_capability_mapping_cardinality_mismatch".to_string());
            return;
        }
        for command_type in TS_COMMAND_TYPES {
            if !self
                .policy
                .command_required_capabilities
                .contains_key(command_type)
            {
                self.bootstrap_error = Some(format!(
                    "policy_missing_command_capability_mapping:{command_type}"
                ));
                return;
            }
        }

        let constitution_body = match fs::read_to_string(&self.policy.constitution_path) {
            Ok(body) => body,
            Err(_) => {
                self.bootstrap_error = Some("constitution_file_unavailable".to_string());
                return;
            }
        };
        for marker in &self.policy.required_constitution_markers {
            if !constitution_body.contains(marker) {
                self.bootstrap_error = Some(format!("constitution_marker_missing:{marker}"));
                return;
            }
        }

        let registry_body = match fs::read_to_string(&self.policy.guard_registry_path) {
            Ok(body) => body,
            Err(_) => {
                self.bootstrap_error = Some("guard_registry_unavailable".to_string());
                return;
            }
        };
        let checks = match parse_guard_registry_check_ids(&registry_body) {
            Ok(ids) => ids,
            Err(reason) => {
                self.bootstrap_error = Some(reason);
                return;
            }
        };
        for required in &self.policy.required_guard_checks {
            if !checks.contains(required) {
                self.bootstrap_error =
                    Some(format!("guard_registry_required_check_missing:{required}"));
                return;
            }
        }
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
        if let Err(reason) = self.validate_command_mapping(command) {
            return PolicyDecision::deny(reason);
        }
        PolicyDecision::allow()
    }
}

#[derive(Debug, Deserialize)]
struct GuardRegistrySnapshot {
    merge_guard: Option<GuardRegistryMergeGuard>,
}

#[derive(Debug, Deserialize)]
struct GuardRegistryMergeGuard {
    checks: Option<Vec<GuardRegistryCheck>>,
}
