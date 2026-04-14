// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/memory (authoritative unified memory heap substrate).
// V6-MEMORY-042 authoritative memory core substrate.

pub mod blob_store;
pub mod consolidation;
pub mod context_atoms;
pub mod context_budget;
pub mod context_compaction;
pub mod context_fidelity;
pub mod context_materializer;
pub mod context_topology;
pub mod graph_subsystem;
pub mod heap_interface;
pub mod policy;
pub mod promotion;
pub mod recall;
pub mod record_store;
pub mod schemas;
pub mod vector_index;
pub mod version_ledger;

pub use blob_store::BlobStore;
pub use consolidation::{ConsolidationReport, ConsolidatedMemoryDraft};
pub use context_atoms::{ContextAtom, ContextAtomSourceKind};
pub use context_budget::{ContextBudgetReport, ContextBudgetRequest};
pub use context_materializer::{
    ContextFragment, ContextFragmentKind, ContextMaterialization, ContextTopologyMaterialization,
    MaterializedMemoryEntry,
};
pub use context_topology::{
    ContextAppendInput, ContextAppendOutcome, ContextFrontier, ContextPressureState, ContextSpan,
    ContextSpanStatus, ContextTopology, ContextTopologyConfig, ContextTopologyRebuildReport,
};
pub use graph_subsystem::{
    GraphEdge, GraphNode, GraphSubsystem, KnowledgeEntityKind, KnowledgeGraph,
    KnowledgeGraphEdge, KnowledgeGraphNode, KnowledgeRelationKind, TaskFabricLease,
};
pub use heap_interface::{NexusRouteContext, UnifiedMemoryHeap, UnifiedMemoryHeapConfig};
pub use policy::{
    CapabilityValidationResult, DefaultVerityMemoryPolicy, MemoryPolicyDecision, MemoryPolicyGate,
    MemoryPolicyRequest, PolicyAction,
};
pub use promotion::{is_valid_trust_transition, rollback_head_from_version};
pub use recall::{
    MemoryRecallExplanation, MemoryRecallFeedbackSignal, MemoryRecallHit, MemoryRecallQuery,
};
pub use record_store::RecordStore;
pub use schemas::{
    memory_scope_authority_matrix, owner_export_redaction_matrix, task_fabric_lease_cas_rules,
    trust_state_transition_matrix, CanonicalMemoryRecord, CapabilityAction, CapabilityToken,
    Classification, ContextManifest, ContextManifestEntryRef, DerivationKind, MemoryDerivation,
    MemoryInvalidationReason, MemoryInvalidationRecord, MemoryKind, MemoryMutationReplayRow,
    MemoryObject, MemoryPurgeRecord, MemoryReceipt, MemoryRetentionPolicy, MemorySalience,
    MemoryScope, MemoryVersion, OwnerExportRedactionPolicy, OwnerScopeSettings,
    PurgeRelationType, RetentionPurgeReport, TrustState,
};
pub use vector_index::{
    embed_text, InMemoryVectorStore, VectorIndex, VectorMetadata, VectorQueryFilter,
    VectorQueryRow, VectorStoreBackend,
};
pub use version_ledger::VersionLedger;

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
mod context_topology_heap_tests;
#[cfg(test)]
mod tests;
