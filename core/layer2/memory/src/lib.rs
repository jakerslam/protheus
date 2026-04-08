// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/memory (authoritative unified memory heap substrate).

pub mod blob_store;
pub mod context_materializer;
pub mod graph_subsystem;
pub mod heap_interface;
pub mod policy;
pub mod promotion;
pub mod record_store;
pub mod schemas;
pub mod vector_index;
pub mod version_ledger;

pub use blob_store::BlobStore;
pub use context_materializer::{ContextMaterialization, MaterializedMemoryEntry};
pub use graph_subsystem::{GraphEdge, GraphNode, GraphSubsystem, TaskFabricLease};
pub use heap_interface::{NexusRouteContext, UnifiedMemoryHeap, UnifiedMemoryHeapConfig};
pub use policy::{
    CapabilityValidationResult, DefaultVerityMemoryPolicy, MemoryPolicyDecision, MemoryPolicyGate,
    MemoryPolicyRequest, PolicyAction,
};
pub use promotion::{is_valid_trust_transition, rollback_head_from_version};
pub use record_store::RecordStore;
pub use schemas::{
    memory_scope_authority_matrix, owner_export_redaction_matrix, task_fabric_lease_cas_rules,
    trust_state_transition_matrix, CanonicalMemoryRecord, CapabilityAction, CapabilityToken,
    Classification, ContextManifest, ContextManifestEntryRef, MemoryMutationReplayRow,
    MemoryObject, MemoryPurgeRecord, MemoryReceipt, MemoryRetentionPolicy, MemoryScope,
    MemoryVersion, OwnerExportRedactionPolicy, OwnerScopeSettings, PurgeRelationType,
    RetentionPurgeReport, TrustState,
};
pub use vector_index::VectorIndex;
pub use version_ledger::VersionLedger;

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn deterministic_hash<T: Serialize>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    let mut hasher = Sha256::new();
    hasher.update(payload);
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
