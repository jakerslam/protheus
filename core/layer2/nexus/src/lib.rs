pub mod conduit_manager;
pub mod main_nexus;
pub mod policy;
pub mod registry;
pub mod route_lease;
pub mod sub_nexus;
pub mod template;

pub use infring_task_fabric_core_v1 as task_fabric;
pub use infring_agent_derive::{infring_agent, infring_tool};
pub use conduit_security::{
    deterministic_hash as conduit_deterministic_hash, CapabilityToken, CapabilityTokenAuthority,
    MessageSigner, RateLimitPolicy, RateLimiter, SecurityError,
};
pub use exotic_wrapper as exotic;
pub use infring_layer1_security as layer1_security;
pub use infring_types::{
    compute_blob_manifest_signature, decode_normalized_blob_manifest,
    decode_signed_bincode_blob_manifest_with_adapter, normalize_blob_id, normalize_sha256_hash,
    NormalizedBlobManifestEntry,
};
pub use infring_layer1_provenance::{
    InMemoryReceiptSink, ProvenanceError, ReceiptDraft, ReceiptEmitter, ReceiptSink,
};
pub use protheus_ops_core_v1 as ops_core;
pub use protheus_spine_core_v1 as spine_core;
pub use protheus_stomach_core_v1 as stomach_core;
pub use protheus_memory_core_v6::{
    load_embedded_vault_policy, EmbeddedVaultPolicy,
    load_embedded_observability_profile as load_embedded_profile_from_memory, EmbeddedChaosHook,
    EmbeddedObservabilityProfile,
};
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
