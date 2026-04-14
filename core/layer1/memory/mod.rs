// SPDX-License-Identifier: Apache-2.0
// Plane ownership: Layer 1 memory primitives (ephemeral scope authority substrate).
// SRS coverage anchor: V6-MEMORY-043

pub mod cleanup;
pub mod ephemeral_scope;
pub mod promotion;

use blake3::Hasher;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedScope {
    Ephemeral,
    Agent(String),
    Swarm(String),
    Core,
    Owner,
}

impl UnifiedScope {
    pub fn label(&self) -> String {
        match self {
            Self::Ephemeral => "ephemeral".to_string(),
            Self::Agent(id) => format!("agent:{id}"),
            Self::Swarm(id) => format!("swarm:{id}"),
            Self::Core => "core".to_string(),
            Self::Owner => "owner".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PermanentScope {
    Agent(String),
    Swarm(String),
    Core,
    Owner,
}

impl PermanentScope {
    pub fn label(&self) -> String {
        match self {
            Self::Agent(id) => format!("agent:{id}"),
            Self::Swarm(id) => format!("swarm:{id}"),
            Self::Core => "core".to_string(),
            Self::Owner => "owner".to_string(),
        }
    }

    pub fn as_unified(&self) -> UnifiedScope {
        match self {
            Self::Agent(id) => UnifiedScope::Agent(id.clone()),
            Self::Swarm(id) => UnifiedScope::Swarm(id.clone()),
            Self::Core => UnifiedScope::Core,
            Self::Owner => UnifiedScope::Owner,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    Public,
    Internal,
    Sensitive,
    Restricted,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrustState {
    Proposed,
    Corroborated,
    Validated,
    Canonical,
    Contested,
    Quarantined,
    Revoked,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminalOutcome {
    Active,
    Promoted,
    Cleaned,
}

impl TerminalOutcome {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Promoted => "promoted",
            Self::Cleaned => "cleaned",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EphemeralObject {
    pub object_id: String,
    pub writer_agent_id: String,
    pub trace_id: String,
    pub scope: UnifiedScope,
    pub classification: Classification,
    pub trust_state: TrustState,
    pub capability: String,
    pub payload: Value,
    pub content_hash: String,
    pub bytes: u64,
    pub written_at: u64,
    pub runtime_epoch: u64,
    pub revision_id: u64,
    pub lease_holder: Option<String>,
    pub lease_expires_at: Option<u64>,
    pub terminal_outcome: TerminalOutcome,
    pub promoted_target_object_id: Option<String>,
    pub cleanup_cycle_id: Option<String>,
    pub cleanup_reason: Option<String>,
    pub canonical: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PromotedObject {
    pub target_object_id: String,
    pub source_object_id: String,
    pub target_scope: PermanentScope,
    pub classification: Classification,
    pub trust_state: TrustState,
    pub capability: String,
    pub payload: Value,
    pub promoted_at: u64,
    pub lineage_refs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MaterializedEntry {
    pub object_id: String,
    pub scope: String,
    pub payload: Value,
    pub redacted: bool,
    pub canonical: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EphemeralWriteReceipt {
    pub receipt_id: String,
    pub object_id: String,
    pub writer_agent_id: String,
    pub trace_id: String,
    pub scope: String,
    pub content_hash: String,
    pub bytes: u64,
    pub written_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EphemeralPromotionReceipt {
    pub receipt_id: String,
    pub source_object_id: String,
    pub target_object_id: String,
    pub target_scope: String,
    pub approved_by: String,
    pub promoted_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EphemeralCleanupReceipt {
    pub receipt_id: String,
    pub object_id: String,
    pub cleanup_cycle_id: String,
    pub cleanup_reason: String,
    pub bytes_reclaimed: u64,
    pub cleaned_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EphemeralConflictReceipt {
    pub receipt_id: String,
    pub object_id: String,
    pub contender: String,
    pub expected_revision: u64,
    pub observed_revision: u64,
    pub resolved_outcome: String,
    pub resolved_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum LineageEvent {
    EphemeralWrite(EphemeralWriteReceipt),
    EphemeralPromotion(EphemeralPromotionReceipt),
    EphemeralCleanup(EphemeralCleanupReceipt),
    EphemeralConflict(EphemeralConflictReceipt),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerityEphemeralPolicy {
    pub max_bytes_per_agent_per_epoch: u64,
    pub max_writes_per_agent_per_epoch: u64,
    pub revoked_agents: BTreeSet<String>,
    pub throttled_agents: BTreeSet<String>,
    pub promotion_approvers: BTreeSet<String>,
    pub debug_principals: BTreeSet<String>,
}

impl Default for VerityEphemeralPolicy {
    fn default() -> Self {
        let mut approvers = BTreeSet::new();
        approvers.insert("verity".to_string());
        Self {
            max_bytes_per_agent_per_epoch: 4 * 1024 * 1024,
            max_writes_per_agent_per_epoch: 1024,
            revoked_agents: BTreeSet::new(),
            throttled_agents: BTreeSet::new(),
            promotion_approvers: approvers,
            debug_principals: BTreeSet::new(),
        }
    }
}

impl VerityEphemeralPolicy {
    pub fn can_write(&self, agent_id: &str) -> Result<(), EphemeralMemoryError> {
        if self.revoked_agents.contains(agent_id) {
            return Err(EphemeralMemoryError::AccessRevoked(agent_id.to_string()));
        }
        if self.throttled_agents.contains(agent_id) {
            return Err(EphemeralMemoryError::AccessThrottled(agent_id.to_string()));
        }
        Ok(())
    }

    pub fn can_approve_promotion(&self, approver: &str) -> bool {
        self.promotion_approvers.contains(approver)
    }

    pub fn can_debug_ephemeral(&self, principal: &str) -> bool {
        self.debug_principals.contains(principal)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentUsage {
    pub bytes_written: u64,
    pub writes: u64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EphemeralMemoryError {
    #[error("ephemeral object not found: {0}")]
    ObjectNotFound(String),
    #[error("capability is required for ephemeral writes")]
    CapabilityRequired,
    #[error("ephemeral access revoked for agent: {0}")]
    AccessRevoked(String),
    #[error("ephemeral access throttled for agent: {0}")]
    AccessThrottled(String),
    #[error("ephemeral write-rate limit exceeded for agent: {0}")]
    WriteRateLimitExceeded(String),
    #[error("ephemeral byte budget exceeded for agent: {0}")]
    ByteBudgetExceeded(String),
    #[error("verity denied promotion approval: {0}")]
    PromotionApprovalDenied(String),
    #[error("lease is required for object: {0}")]
    LeaseRequired(String),
    #[error("lease held by different actor for object: {0}")]
    LeaseHeld(String),
    #[error("lease expired for object: {0}")]
    LeaseExpired(String),
    #[error("compare-and-swap mismatch for object: {0}")]
    CasMismatch(String),
    #[error("object already has terminal outcome: {0}")]
    AlreadyTerminal(String),
    #[error("agent resume blocked by stale ephemeral payload count={0}")]
    ResumeBlockedByStalePayload(usize),
    #[error("serialization failure: {0}")]
    Serialization(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LeaseMode {
    Optional,
    Required,
}

#[derive(Clone, Debug, Default)]
pub struct EphemeralMemoryHeap {
    pub(crate) policy: VerityEphemeralPolicy,
    pub(crate) objects: BTreeMap<String, EphemeralObject>,
    pub(crate) promoted: BTreeMap<String, PromotedObject>,
    pub(crate) lineage_events: Vec<LineageEvent>,
    pub(crate) agent_usage: BTreeMap<String, AgentUsage>,
    pub(crate) receipt_counter: u64,
    pub(crate) cleanup_cycle_counter: u64,
    pub(crate) runtime_epoch: u64,
    pub(crate) resume_blocked: bool,
}

impl EphemeralMemoryHeap {
    pub fn new(policy: VerityEphemeralPolicy) -> Self {
        Self {
            policy,
            runtime_epoch: 1,
            ..Self::default()
        }
    }

    pub fn runtime_epoch(&self) -> u64 {
        self.runtime_epoch
    }

    pub fn lineage_events(&self) -> &[LineageEvent] {
        self.lineage_events.as_slice()
    }

    pub fn ephemeral_object(&self, object_id: &str) -> Option<&EphemeralObject> {
        self.objects.get(object_id)
    }

    pub fn promoted_object(&self, object_id: &str) -> Option<&PromotedObject> {
        self.promoted.get(object_id)
    }

    pub(crate) fn push_lineage(&mut self, event: LineageEvent) {
        self.lineage_events.push(event);
    }

    pub(crate) fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|row| row.as_millis() as u64)
            .unwrap_or(0)
    }

    pub(crate) fn payload_hash_and_size(
        payload: &Value,
    ) -> Result<(String, u64), EphemeralMemoryError> {
        let bytes = serde_json::to_vec(payload)
            .map_err(|err| EphemeralMemoryError::Serialization(err.to_string()))?;
        Ok((blake3_hex(bytes.as_slice()), bytes.len() as u64))
    }

    pub(crate) fn next_receipt_id(&mut self, kind: &str, seed: Value) -> String {
        self.receipt_counter = self.receipt_counter.saturating_add(1);
        let source = json!({
            "kind": kind,
            "counter": self.receipt_counter,
            "runtime_epoch": self.runtime_epoch,
            "seed": seed
        });
        format!(
            "receipt_{}",
            &blake3_hex(source.to_string().as_bytes())[..24]
        )
    }

    pub(crate) fn next_entity_id(&mut self, prefix: &str, kind: &str, seed: Value) -> String {
        let receipt_like = self.next_receipt_id(kind, seed);
        format!("{prefix}{}", &receipt_like["receipt_".len()..])
    }

    pub(crate) fn next_cleanup_cycle_id(&mut self, prefix: &str) -> String {
        self.cleanup_cycle_counter = self.cleanup_cycle_counter.saturating_add(1);
        let seed = format!(
            "{prefix}:{}:{}",
            self.runtime_epoch, self.cleanup_cycle_counter
        );
        format!("cycle_{}", &blake3_hex(seed.as_bytes())[..24])
    }

    pub(crate) fn push_conflict_receipt(
        &mut self,
        object_id: &str,
        contender: &str,
        expected_revision: u64,
        observed_revision: u64,
        resolved_outcome: &str,
    ) {
        let receipt_id = self.next_receipt_id(
            "ephemeral_conflict",
            json!({
                "object_id": object_id,
                "contender": contender,
                "expected_revision": expected_revision,
                "observed_revision": observed_revision,
                "resolved_outcome": resolved_outcome
            }),
        );
        let event = LineageEvent::EphemeralConflict(EphemeralConflictReceipt {
            receipt_id,
            object_id: object_id.to_string(),
            contender: contender.to_string(),
            expected_revision,
            observed_revision,
            resolved_outcome: resolved_outcome.to_string(),
            resolved_at: Self::now_ms(),
        });
        self.push_lineage(event);
    }

    pub(crate) fn validate_mutation_claim(
        &mut self,
        object_id: &str,
        snapshot: &EphemeralObject,
        contender: &str,
        expected_revision: u64,
        lease_mode: LeaseMode,
        now: u64,
        revision_conflict_outcome: &str,
        lease_conflict_outcome: &str,
    ) -> Result<(), EphemeralMemoryError> {
        if snapshot.revision_id != expected_revision {
            self.push_conflict_receipt(
                object_id,
                contender,
                expected_revision,
                snapshot.revision_id,
                revision_conflict_outcome,
            );
            return Err(EphemeralMemoryError::CasMismatch(object_id.to_string()));
        }
        match (snapshot.lease_holder.as_ref(), snapshot.lease_expires_at) {
            (None, _) => {
                if lease_mode == LeaseMode::Required {
                    return Err(EphemeralMemoryError::LeaseRequired(object_id.to_string()));
                }
            }
            (Some(_), None) => {
                return Err(EphemeralMemoryError::LeaseExpired(object_id.to_string()))
            }
            (Some(holder), Some(expires_at)) => {
                if expires_at <= now {
                    return Err(EphemeralMemoryError::LeaseExpired(object_id.to_string()));
                }
                if holder != contender {
                    self.push_conflict_receipt(
                        object_id,
                        contender,
                        expected_revision,
                        snapshot.revision_id,
                        lease_conflict_outcome,
                    );
                    return Err(EphemeralMemoryError::LeaseHeld(object_id.to_string()));
                }
            }
        }
        Ok(())
    }
}

fn blake3_hex(bytes: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests;
