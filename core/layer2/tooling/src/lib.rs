// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/tooling (authoritative canonical tool/evidence substrate).

pub mod client_adapter;
pub mod evidence_extractor;
pub mod evidence_store;
pub mod schemas;
pub mod tool_broker;
pub mod verifier;

pub use client_adapter::{ClientAdapterRequest, ClientDelegationResult, ThinClientDelegator};
pub use evidence_extractor::EvidenceExtractor;
pub use evidence_store::{
    EvidenceInvalidationRecord, EvidenceRecord, EvidenceStore, InvalidationRelationType,
};
pub use schemas::{
    Claim, ClaimBundle, ClaimStatus, ConfidenceVector, NormalizedToolMetrics, NormalizedToolResult,
    NormalizedToolStatus, WorkerBudgetUsed, WorkerOutput, WorkerTaskStatus,
};
pub use tool_broker::{
    BrokerCaller, BrokerError, ToolBroker, ToolBrokerExecution, ToolCallRequest,
};
pub use verifier::StructuredVerifier;

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
