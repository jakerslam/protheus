// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/tooling (authoritative canonical tool/evidence substrate).
// SRS coverage anchor: V6-TOOL-005

pub mod backend_registry;
pub mod capability;
pub mod capability_contract_surface;
#[cfg(test)]
mod capability_tests;
pub mod client_adapter;
pub mod evidence_extractor;
mod evidence_extractor_artifacts;
#[cfg(test)]
mod evidence_extractor_tests;
pub mod evidence_quality;
pub mod evidence_sanitizer;
pub mod evidence_store;
mod request_validation;
#[cfg(test)]
mod request_validation_tests;
pub mod schemas;
#[cfg(test)]
mod schemas_tests;
pub mod tool_broker;
pub mod tool_contracts;
pub mod verifier;

pub use backend_registry::{live_backend_registry, ToolBackendClass, ToolBackendHealth};
pub use capability::{
    ToolCapability, ToolCapabilityCatalogGroup, ToolCapabilityDomain, ToolCapabilityProbe,
    ToolCapabilityStatus, ToolReasonCode,
};
pub use capability_contract_surface::ToolCapabilityContractSurface;
pub use client_adapter::{ClientAdapterRequest, ClientDelegationResult, ThinClientDelegator};
pub use evidence_extractor::EvidenceExtractor;
pub use evidence_quality::{
    classify_tool_result_quality, payload_text, tool_payload_evidence_count,
};
pub use evidence_sanitizer::{
    safety_flags_from_report, sanitize_text_for_evidence, EvidenceSanitizationReport,
    SanitizedEvidenceText,
};
pub use evidence_store::{
    EvidenceInvalidationRecord, EvidenceLedgerEvent, EvidenceRecord, EvidenceStore,
    InvalidationRelationType,
};
pub use request_validation::repair_and_validate_args;
pub use schemas::{
    published_schema_contract_v1, Claim, ClaimBundle, ClaimStatus, ConfidenceVector,
    EvidenceArtifactRef, EvidenceCard, NormalizedToolMetrics, NormalizedToolResult,
    NormalizedToolStatus, WorkerBudgetUsed, WorkerOutput, WorkerTaskStatus, CLAIM_BUNDLE_FIELDS,
    CLAIM_FIELDS, EVIDENCE_CARD_FIELDS, NORMALIZED_TOOL_RESULT_FIELDS, TOOL_ATTEMPT_RECEIPT_FIELDS,
    TOOL_CAPABILITY_PROBE_FIELDS, WORKER_OUTPUT_FIELDS,
};
pub use tool_broker::{
    BrokerCaller, BrokerError, ToolAttemptEnvelope, ToolAttemptReceipt, ToolAttemptStatus,
    ToolBroker, ToolBrokerExecution, ToolCallRequest, ToolExecutionReceipt,
    ToolExecutionReceiptStatus, ToolSubstrateHealthReport,
};
pub use tool_contracts::{
    published_tool_cd_catalog_v1, tool_cd_contract_for, tool_cd_contract_index_v1,
    validate_tool_cd_catalog, ToolCdCatalog, ToolCdContract, ToolEvidencePackagingContract,
    ToolExtractionContract, ToolQualityClassificationContract, ToolReadinessContract,
    ToolResourcePolicyContract, ToolRetrievalContract, ToolSafetyContract,
    ToolSanitizationContract, ToolSessionPolicyContract, ToolVisibilityContract,
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
