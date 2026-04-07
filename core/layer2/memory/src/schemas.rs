use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    Public,
    Agent(String),
    Swarm(String),
    Core,
    Owner,
}

impl MemoryScope {
    pub fn label(&self) -> String {
        match self {
            Self::Public => "public".to_string(),
            Self::Agent(id) => format!("agent:{id}"),
            Self::Swarm(id) => format!("swarm:{id}"),
            Self::Core => "core".to_string(),
            Self::Owner => "owner".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    Public,
    Internal,
    Sensitive,
    Restricted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityAction {
    Read,
    Write,
    Promote,
    Canonicalize,
    MaterializeContext,
    TaskFabricMutate,
    ExportOwnerRaw,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityToken {
    pub token_id: String,
    pub principal_id: String,
    pub scopes: Vec<MemoryScope>,
    pub allowed_actions: Vec<CapabilityAction>,
    pub expires_at_ms: u64,
    pub verity_class: String,
    pub receipt_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryObject {
    pub object_id: String,
    pub scope: MemoryScope,
    pub classification: Classification,
    pub namespace: String,
    pub key: String,
    pub payload: Value,
    pub metadata: Value,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryVersion {
    pub version_id: String,
    pub object_id: String,
    pub scope: MemoryScope,
    pub parent_version_id: Option<String>,
    pub lineage_refs: Vec<String>,
    pub receipt_id: String,
    pub trust_state: TrustState,
    pub payload: Value,
    pub payload_hash: String,
    pub timestamp_ms: u64,
    pub proposed_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryReceipt {
    pub receipt_id: String,
    pub event_type: String,
    pub issuer: String,
    pub source: String,
    pub target: String,
    pub schema_id: String,
    pub template_version_id: Option<String>,
    pub ttl_ms: Option<u64>,
    pub policy_decision_ref: String,
    pub revocation_cause: Option<String>,
    pub timestamp_ms: u64,
    pub lineage_refs: Vec<String>,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OwnerExportRedactionPolicy {
    AllowFull,
    AllowRedacted,
    SummarizeOnly,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OwnerConsentMode {
    ExplicitApproval,
    DelegatedSteward,
    Restricted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnerScopeSettings {
    pub consent_mode: OwnerConsentMode,
    pub export_redaction_policy: OwnerExportRedactionPolicy,
}

impl Default for OwnerScopeSettings {
    fn default() -> Self {
        Self {
            consent_mode: OwnerConsentMode::ExplicitApproval,
            export_redaction_policy: OwnerExportRedactionPolicy::AllowRedacted,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextManifestEntryRef {
    pub object_id: String,
    pub version_id: String,
    pub scope: MemoryScope,
    pub trust_state: TrustState,
    pub redacted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextManifest {
    pub context_manifest_id: String,
    pub principal_id: String,
    pub requested_scopes: Vec<MemoryScope>,
    pub redaction_policy: OwnerExportRedactionPolicy,
    pub entries: Vec<ContextManifestEntryRef>,
    pub lineage_refs: Vec<String>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryScopeAuthorityRule {
    pub scope: &'static str,
    pub canonicalize_authority: &'static str,
    pub cross_scope_promotion_rule: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustStateTransitionRule {
    pub from: &'static str,
    pub to: &'static str,
    pub gate: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnerExportRedactionRule {
    pub policy: &'static str,
    pub raw_export: &'static str,
    pub redacted_export: &'static str,
    pub summary_export: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskFabricLeaseCasRule {
    pub operation: &'static str,
    pub lease_required: bool,
    pub cas_required: bool,
    pub authority: &'static str,
}

pub fn memory_scope_authority_matrix() -> &'static [MemoryScopeAuthorityRule] {
    &[
        MemoryScopeAuthorityRule {
            scope: "public",
            canonicalize_authority: "any authorized principal",
            cross_scope_promotion_rule: "explicit + receipt",
        },
        MemoryScopeAuthorityRule {
            scope: "agent:<id>",
            canonicalize_authority: "owning agent",
            cross_scope_promotion_rule: "explicit + receipt + lineage",
        },
        MemoryScopeAuthorityRule {
            scope: "swarm:<id>",
            canonicalize_authority: "swarm steward",
            cross_scope_promotion_rule: "explicit + receipt + lineage",
        },
        MemoryScopeAuthorityRule {
            scope: "core",
            canonicalize_authority: "core organs + verity",
            cross_scope_promotion_rule: "validated + verity approval + receipt",
        },
        MemoryScopeAuthorityRule {
            scope: "owner",
            canonicalize_authority: "owner or delegated steward",
            cross_scope_promotion_rule: "consent mode + explicit approval + receipt",
        },
    ]
}

pub fn trust_state_transition_matrix() -> &'static [TrustStateTransitionRule] {
    &[
        TrustStateTransitionRule {
            from: "proposed",
            to: "corroborated",
            gate: "verity evidence corroboration + receipt",
        },
        TrustStateTransitionRule {
            from: "corroborated",
            to: "validated",
            gate: "verity validation + policy decision reference",
        },
        TrustStateTransitionRule {
            from: "validated",
            to: "canonical",
            gate: "canonicalization authority + receipt lineage",
        },
        TrustStateTransitionRule {
            from: "proposed",
            to: "quarantined",
            gate: "policy violation or contamination signal",
        },
        TrustStateTransitionRule {
            from: "corroborated",
            to: "contested",
            gate: "conflicting evidence branch",
        },
        TrustStateTransitionRule {
            from: "canonical",
            to: "revoked",
            gate: "verity revocation + explicit cause",
        },
    ]
}

pub fn owner_export_redaction_matrix() -> &'static [OwnerExportRedactionRule] {
    &[
        OwnerExportRedactionRule {
            policy: "allow_full",
            raw_export: "allow",
            redacted_export: "allow",
            summary_export: "allow",
        },
        OwnerExportRedactionRule {
            policy: "allow_redacted",
            raw_export: "deny",
            redacted_export: "allow",
            summary_export: "allow",
        },
        OwnerExportRedactionRule {
            policy: "summarize_only",
            raw_export: "deny",
            redacted_export: "deny",
            summary_export: "allow",
        },
        OwnerExportRedactionRule {
            policy: "deny",
            raw_export: "deny",
            redacted_export: "deny",
            summary_export: "deny",
        },
    ]
}

pub fn task_fabric_lease_cas_rules() -> &'static [TaskFabricLeaseCasRule] {
    &[
        TaskFabricLeaseCasRule {
            operation: "create_task_node",
            lease_required: false,
            cas_required: false,
            authority: "task_fabric_mutate capability + verity policy allow",
        },
        TaskFabricLeaseCasRule {
            operation: "mutate_task_node",
            lease_required: true,
            cas_required: true,
            authority: "lease holder only; deny on stale cas",
        },
        TaskFabricLeaseCasRule {
            operation: "add_graph_edge",
            lease_required: true,
            cas_required: true,
            authority: "lease holder only; single shared graph subsystem",
        },
        TaskFabricLeaseCasRule {
            operation: "close_task_node",
            lease_required: true,
            cas_required: true,
            authority: "lease holder + verity policy allow",
        },
    ]
}
