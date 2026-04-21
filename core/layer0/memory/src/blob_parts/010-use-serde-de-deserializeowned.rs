// SPDX-License-Identifier: Apache-2.0
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use snap::raw::{Decoder, Encoder};
use std::fmt::{Display, Formatter};
use infring_types::{
    compute_blob_manifest_signature, decode_signed_bincode_blob_manifest_with_adapter,
    normalize_sha256_hash,
};

#[allow(dead_code)]
pub(crate) fn contains_forbidden_runtime_context_marker(raw: &str) -> bool {
    const FORBIDDEN: [&str; 6] = [
        "You are an expert Python programmer.",
        "[PATCH v2",
        "List Leaves (25",
        "BEGIN_OPENCLAW_INTERNAL_CONTEXT",
        "END_OPENCLAW_INTERNAL_CONTEXT",
        "UNTRUSTED_CHILD_RESULT_DELIMITER",
    ];
    FORBIDDEN.iter().any(|marker| raw.contains(marker))
}

pub const HEARTBEAT_BLOB_ID: &str = "heartbeat_sample";
pub const EXECUTION_REPLAY_BLOB_ID: &str = "execution_replay";
pub const VAULT_POLICY_BLOB_ID: &str = "vault_policy";
pub const OBSERVABILITY_PROFILE_BLOB_ID: &str = "observability_profile";
pub const BLOB_VERSION: u32 = 1;

pub const HEARTBEAT_BLOB: &[u8] = include_bytes!("../blobs/heartbeat_sample.blob");
pub const EXECUTION_REPLAY_BLOB: &[u8] = include_bytes!("../blobs/execution_replay.blob");
pub const VAULT_POLICY_BLOB: &[u8] = include_bytes!("../blobs/vault_policy.blob");
pub const OBSERVABILITY_PROFILE_BLOB: &[u8] =
    include_bytes!("../blobs/observability_profile.blob");
pub const BLOB_MANIFEST: &[u8] = include_bytes!("../blobs/manifest.blob");

const MANIFEST_SIGNING_KEY: &str = "memory-blob-signing-key-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlobManifest {
    pub id: String,
    pub hash: String,
    pub version: u32,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FoldedBlob {
    id: String,
    version: u32,
    payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlobArtifactDigest {
    pub id: String,
    pub path: String,
    pub bytes: usize,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlobPackReport {
    pub manifest_path: String,
    pub manifest_bytes: usize,
    pub artifacts: Vec<BlobArtifactDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddedExecutionStep {
    pub id: String,
    pub kind: String,
    pub action: String,
    pub command: String,
    pub pause_after: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddedExecutionReceiptModel {
    pub deterministic: bool,
    pub replayable: bool,
    pub digest_algorithm: String,
    pub status_cycle: Vec<String>,
    pub state_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddedExecutionReplay {
    pub engine_version: String,
    pub workflow_id: String,
    pub deterministic_seed: String,
    pub pause_resume_contract: Vec<String>,
    pub steps: Vec<EmbeddedExecutionStep>,
    pub receipt_model: EmbeddedExecutionReceiptModel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddedVaultPolicyRule {
    pub id: String,
    pub objective: String,
    pub zk_requirement: String,
    pub fhe_requirement: String,
    pub severity: String,
    pub fail_closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddedVaultAutoRotatePolicy {
    pub enabled: bool,
    pub rotate_after_hours: u32,
    pub max_key_age_hours: u32,
    pub grace_window_minutes: u32,
    pub quorum_required: u8,
    pub emergency_rotate_on_tamper: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddedVaultPolicy {
    pub policy_id: String,
    pub version: u32,
    pub key_domain: String,
    pub cryptographic_profile: String,
    pub attestation_chain: Vec<String>,
    pub auto_rotate: EmbeddedVaultAutoRotatePolicy,
    pub rules: Vec<EmbeddedVaultPolicyRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddedTraceStreamPolicy {
    pub trace_window_ms: u32,
    pub max_events_per_window: u32,
    pub min_sampling_rate_pct: u8,
    pub redact_fields: Vec<String>,
    pub require_signature: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddedSovereigntyScorer {
    pub integrity_weight_pct: u8,
    pub continuity_weight_pct: u8,
    pub reliability_weight_pct: u8,
    pub chaos_penalty_pct: u8,
    pub fail_closed_threshold_pct: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddedChaosHook {
    pub id: String,
    pub condition: String,
    pub action: String,
    pub severity: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddedObservabilityProfile {
    pub profile_id: String,
    pub version: u32,
    pub red_legion_trace_channels: Vec<String>,
    pub allowed_emitters: Vec<String>,
    pub stream_policy: EmbeddedTraceStreamPolicy,
    pub sovereignty_scorer: EmbeddedSovereigntyScorer,
    pub chaos_hooks: Vec<EmbeddedChaosHook>,
}

#[derive(Debug, Clone)]
pub enum BlobError {
    InvalidBlobId,
    UnknownBlob(String),
    MissingManifestEntry(String),
    MissingSignature(String),
    SignatureMismatch {
        id: String,
        expected: String,
        actual: String,
    },
    HashMismatch {
        scope: &'static str,
        expected: String,
        actual: String,
    },
    IdMismatch {
        expected: String,
        actual: String,
    },
    UnsupportedVersion {
        id: String,
        version: u32,
    },
    SerializeFailed(String),
    DeserializeFailed(String),
    CompressFailed(String),
    DecompressFailed(String),
    ManifestEncodeFailed(String),
    ManifestDecodeFailed(String),
    IoFailed(String),
}

impl Display for BlobError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BlobError::InvalidBlobId => write!(f, "blob_id_required"),
            BlobError::UnknownBlob(blob_id) => write!(f, "unknown_blob_id:{blob_id}"),
            BlobError::MissingManifestEntry(blob_id) => {
                write!(f, "manifest_missing_blob:{blob_id}")
            }
            BlobError::MissingSignature(blob_id) => {
                write!(f, "manifest_missing_signature:{blob_id}")
            }
            BlobError::SignatureMismatch {
                id,
                expected,
                actual,
            } => write!(
                f,
                "manifest_signature_mismatch id={id} expected={expected} actual={actual}"
            ),
            BlobError::HashMismatch {
                scope,
                expected,
                actual,
            } => write!(
                f,
                "blob_hash_mismatch scope={scope} expected={expected} actual={actual}"
            ),
            BlobError::IdMismatch { expected, actual } => {
                write!(f, "blob_id_mismatch expected={expected} actual={actual}")
            }
            BlobError::UnsupportedVersion { id, version } => {
                write!(f, "unsupported_blob_version id={id} version={version}")
            }
            BlobError::SerializeFailed(msg) => write!(f, "serialize_failed:{msg}"),
            BlobError::DeserializeFailed(msg) => write!(f, "deserialize_failed:{msg}"),
            BlobError::CompressFailed(msg) => write!(f, "compress_failed:{msg}"),
            BlobError::DecompressFailed(msg) => write!(f, "decompress_failed:{msg}"),
            BlobError::ManifestEncodeFailed(msg) => write!(f, "manifest_encode_failed:{msg}"),
            BlobError::ManifestDecodeFailed(msg) => write!(f, "manifest_decode_failed:{msg}"),
            BlobError::IoFailed(msg) => write!(f, "io_failed:{msg}"),
        }
    }
}

impl std::error::Error for BlobError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
