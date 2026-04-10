use crate::conduit_manager::ConduitManager;
use crate::deterministic_hash;
use crate::now_ms;
use crate::policy::{
    DefaultNexusPolicy, NexusFeatureFlags, NexusPolicyGate, PolicyDecisionRef,
    PolicyEvaluationContext, TrustClass, VerityClass,
};
use crate::registry::{ModuleKind, ModuleLifecycleState, NexusRegistry, SubNexusRegistration};
use crate::route_lease::{LeaseAuthorizationInput, RevocationCause, RouteLeaseCapability};
use crate::sub_nexus::SubNexus;
use crate::template::{ConnectionTemplate, TemplateRegistry};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const MAIN_NEXUS_ID: &str = "main_nexus";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseIssueRequest {
    pub source: String,
    pub target: String,
    pub schema_ids: Vec<String>,
    pub verbs: Vec<String>,
    pub required_verity: VerityClass,
    pub trust_class: TrustClass,
    pub requested_ttl_ms: u64,
    pub template_id: Option<String>,
    pub template_version: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryAuthorizationInput {
    pub lease_id: Option<String>,
    pub source: String,
    pub target: String,
    pub schema_id: String,
    pub verb: String,
    pub offered_verity: VerityClass,
    pub now_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectDeliveryAuthorization {
    pub allowed: bool,
    pub reason: String,
    pub local_resolution: bool,
    pub lease_id: Option<String>,
    pub conduit_link_id: Option<String>,
}

impl DirectDeliveryAuthorization {
    pub fn allow(
        reason: impl Into<String>,
        local_resolution: bool,
        lease_id: Option<String>,
        conduit_link_id: Option<String>,
    ) -> Self {
        Self {
            allowed: true,
            reason: reason.into(),
            local_resolution,
            lease_id,
            conduit_link_id,
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: reason.into(),
            local_resolution: false,
            lease_id: None,
            conduit_link_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NexusReceiptKind {
    Registration,
    TemplateInstantiation,
    LeaseIssued,
    LeaseRevoked,
    LifecycleTransition,
    PlasticityEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NexusReceipt {
    pub receipt_id: String,
    pub kind: NexusReceiptKind,
    pub ts_ms: u64,
    pub issuer: String,
    pub source: Option<String>,
    pub target: Option<String>,
    pub schema_ids: Vec<String>,
    pub template_id: Option<String>,
    pub template_version: Option<u32>,
    pub ttl_ms: Option<u64>,
    pub policy_decision_ref: Option<String>,
    pub revocation_cause: Option<RevocationCause>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NexusMetrics {
    pub local_resolution_count: u64,
    pub cross_module_resolution_count: u64,
    pub local_resolution_ratio: f64,
    pub active_lease_count: usize,
    pub revoked_lease_count: usize,
    pub active_conduit_count: usize,
}

impl NexusMetrics {
    pub fn set_resolution_counts(&mut self, local_count: u64, cross_module_count: u64) {
        self.local_resolution_count = local_count;
        self.cross_module_resolution_count = cross_module_count;
        let total = local_count.saturating_add(cross_module_count);
        self.local_resolution_ratio = if total == 0 {
            1.0
        } else {
            local_count as f64 / total as f64
        };
    }
}

pub struct MainNexusControlPlane {
    feature_flags: NexusFeatureFlags,
    policy: DefaultNexusPolicy,
    registry: NexusRegistry,
    template_registry: TemplateRegistry,
    conduit_manager: ConduitManager,
    sub_nexuses: BTreeMap<String, SubNexus>,
    leases: BTreeMap<String, RouteLeaseCapability>,
    receipts: Vec<NexusReceipt>,
    metrics: NexusMetrics,
}
