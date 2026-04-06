pub mod conduit_manager;
pub mod main_nexus;
pub mod policy;
pub mod registry;
pub mod route_lease;
pub mod sub_nexus;
pub mod template;

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

pub(crate) fn deterministic_hash<T: Serialize>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    let mut hasher = Sha256::new();
    hasher.update(&payload);
    format!("{:x}", hasher.finalize())
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
