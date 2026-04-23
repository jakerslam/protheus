pub mod conduit_manager;
pub mod main_nexus;
pub mod policy;
pub mod registry;
pub mod route_lease;
pub mod sub_nexus;
pub mod template;

pub use burn_oracle_budget_gate;
pub use foundation_hook_enforcer;
pub use infring_task_fabric_core_v1 as task_fabric;
pub use infring_agent_derive::{infring_agent, infring_tool};
pub use llm_runtime;
pub use persona_dispatch_security_gate;
pub use conduit_security::{
    deterministic_hash as conduit_deterministic_hash, CapabilityToken, CapabilityTokenAuthority,
    MessageSigner, RateLimitPolicy, RateLimiter, SecurityError,
};
pub use exotic_wrapper as exotic;
pub use execution_core as execution_core_v1;
pub use ipc;
pub use infring_layer1_security as layer1_security;
pub use isolation;
pub use infring_layer1_memory_runtime::{recall_policy, token_telemetry};
pub use os_extension_wrapper;
pub use infring_types::{
    compute_blob_manifest_signature, decode_normalized_blob_manifest,
    decode_signed_bincode_blob_manifest_with_adapter, normalize_blob_id, normalize_sha256_hash,
    NormalizedBlobManifestEntry,
};
pub use infring_memory_core_v1 as memory_core_v1;
pub use infring_autonomy_core_v1 as autonomy_core;
pub use infring_graph_core_v1 as graph_core_v1;
pub use infring_pinnacle_core_v1 as pinnacle_core_v1;
pub use infring_red_legion_core_v1 as red_legion_core_v1;
pub use infring_swarm_core_v1 as swarm_core_v1;
pub use infring_observability_core_v1::{
    evaluate_trace_window, load_embedded_observability_profile, run_chaos_resilience,
    ChaosResilienceReport, ChaosScenarioRequest, TraceEvent, TraceWindowReport,
};
pub use infring_tiny_runtime as tiny_runtime;
pub use resource;
pub use storage;
pub use task;
pub use update;
pub use infring_vault_core_v1 as vault_core_v1;
pub use infring_layer1_provenance::{
    InMemoryReceiptSink, ProvenanceError, ReceiptDraft, ReceiptEmitter, ReceiptSink,
};
pub use infring_ops_core_v1 as ops_core;
pub use infring_spine_core_v1 as spine_core;
pub use infring_stomach_core_v1 as stomach_core;
pub use infring_tooling_core_v1 as tooling_core;
pub use infring_memory_core_v6::{
    load_embedded_vault_policy, EmbeddedVaultPolicy,
    load_embedded_observability_profile as load_embedded_profile_from_memory, EmbeddedChaosHook,
    EmbeddedObservabilityProfile,
};
pub use infring_memory_core_v6 as memory_core_v6;
pub use infring_types as types_core;
pub use conduit_manager::{ConduitBackedLink, ConduitManager};
pub use main_nexus::{
    DeliveryAuthorizationInput, DirectDeliveryAuthorization, LeaseIssueRequest,
    MainNexusControlPlane, NexusMetrics, NexusReceipt, NexusReceiptKind,
};
pub use policy::{
    DefaultNexusPolicy, NexusFeatureFlags, NexusPolicyGate, PolicyDecisionRef,
    PolicyEvaluationContext, TrustClass, VerityClass,
};
pub use registry::{ModuleKind, NexusRegistry, SubNexusRegistration};
pub use route_lease::{LeaseAuthorizationInput, RevocationCause, RouteLeaseCapability};
pub use sub_nexus::SubNexus;
pub use template::{ConnectionTemplate, TemplateRegistry};

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

fn stable_json_bytes<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec())
}

fn sha256_hex(payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    format!("{:x}", hasher.finalize())
}

pub(crate) fn deterministic_hash<T: Serialize>(value: &T) -> String {
    let payload = stable_json_bytes(value);
    sha256_hex(payload.as_slice())
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
