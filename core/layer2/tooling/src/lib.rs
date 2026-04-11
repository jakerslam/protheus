// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/tooling (authoritative canonical tool/evidence substrate).

pub mod capability;
pub mod client_adapter;
pub mod evidence_extractor;
pub mod evidence_store;
pub mod schemas;
pub mod tool_broker;
pub mod verifier;

pub use capability::{
    ToolCapability, ToolCapabilityProbe, ToolCapabilityStatus, ToolReasonCode,
};
pub use client_adapter::{ClientAdapterRequest, ClientDelegationResult, ThinClientDelegator};
pub use evidence_extractor::EvidenceExtractor;
pub use evidence_store::{
    EvidenceInvalidationRecord, EvidenceLedgerEvent, EvidenceRecord, EvidenceStore,
    InvalidationRelationType,
};
pub use schemas::{
    published_schema_contract_v1, Claim, ClaimBundle, ClaimStatus, ConfidenceVector, EvidenceCard,
    NormalizedToolMetrics, NormalizedToolResult, NormalizedToolStatus, WorkerBudgetUsed,
    WorkerOutput, WorkerTaskStatus, CLAIM_BUNDLE_FIELDS, CLAIM_FIELDS, EVIDENCE_CARD_FIELDS,
    NORMALIZED_TOOL_RESULT_FIELDS, TOOL_ATTEMPT_RECEIPT_FIELDS, TOOL_CAPABILITY_PROBE_FIELDS,
    WORKER_OUTPUT_FIELDS,
};
pub use tool_broker::{
    BrokerCaller, BrokerError, ToolAttemptEnvelope, ToolAttemptReceipt, ToolAttemptStatus,
    ToolBroker, ToolBrokerExecution, ToolCallRequest,
};
pub use verifier::StructuredVerifier;

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
