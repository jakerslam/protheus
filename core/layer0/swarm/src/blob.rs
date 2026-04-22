// SPDX-License-Identifier: Apache-2.0
use infring_types::decode_normalized_blob_manifest;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

pub const SWARM_STRATEGY_BLOB_ID: &str = "swarm_strategy";
pub const SWARM_STRATEGY_BLOB: &[u8] = include_bytes!("blobs/swarm_strategy.blob");
pub const MANIFEST_BLOB: &[u8] = include_bytes!("blobs/manifest.blob");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwarmStrategyProfile {
    pub profile_id: String,
    pub max_tasks_per_agent: u32,
    pub reliability_weight_pct: f64,
    pub fairness_weight_pct: f64,
    pub consensus_floor_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlobManifest {
    pub id: String,
    pub hash: String,
    pub version: u32,
}

#[derive(Debug, Clone)]
pub enum BlobError {
    ManifestDecodeFailed(String),
    DuplicateManifestEntry(String),
    UnsupportedManifestVersion { id: String, version: u32 },
    BlobNotFound(String),
    HashMismatch { expected: String, actual: String },
    DecodeFailed(String),
}

impl Display for BlobError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BlobError::ManifestDecodeFailed(msg) => write!(f, "manifest_decode_failed:{msg}"),
            BlobError::DuplicateManifestEntry(id) => write!(f, "manifest_duplicate_entry:{id}"),
            BlobError::UnsupportedManifestVersion { id, version } => {
                write!(f, "manifest_unsupported_version:{id}:{version}")
            }
            BlobError::BlobNotFound(id) => write!(f, "blob_not_found:{id}"),
            BlobError::HashMismatch { expected, actual } => {
                write!(f, "blob_hash_mismatch expected={expected} actual={actual}")
            }
            BlobError::DecodeFailed(msg) => write!(f, "blob_decode_failed:{msg}"),
        }
    }
}

impl std::error::Error for BlobError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn fold_blob<T: Serialize>(value: &T, _blob_id: &str) -> Result<(Vec<u8>, String), BlobError> {
    let payload = serde_json::to_vec(value).map_err(|e| BlobError::DecodeFailed(e.to_string()))?;
    let hash = sha256_hex(&payload);
    Ok((payload, hash))
}

pub fn generate_manifest(blobs: &[(&str, &[u8])]) -> Vec<BlobManifest> {
    blobs
        .iter()
        .map(|(id, bytes)| BlobManifest {
            id: (*id).to_string(),
            hash: sha256_hex(bytes),
            version: 1,
        })
        .collect()
}

pub fn decode_manifest(bytes: &[u8]) -> Result<Vec<BlobManifest>, BlobError> {
    let raw_manifest: Vec<BlobManifest> =
        serde_json::from_slice(bytes).map_err(|e| BlobError::ManifestDecodeFailed(e.to_string()))?;
    let mut seen = BTreeSet::<String>::new();
    for entry in &raw_manifest {
        if !seen.insert(entry.id.clone()) {
            return Err(BlobError::DuplicateManifestEntry(entry.id.clone()));
        }
    }
    let manifest = decode_normalized_blob_manifest(bytes, 96)
        .map_err(BlobError::ManifestDecodeFailed)?
        .into_iter()
        .map(|entry| BlobManifest {
            id: entry.id,
            hash: entry.hash,
            version: entry.version,
        })
        .collect::<Vec<_>>();
    for entry in &manifest {
        if entry.version != 1 {
            return Err(BlobError::UnsupportedManifestVersion {
                id: entry.id.clone(),
                version: entry.version,
            });
        }
    }
    Ok(manifest)
}

pub fn unfold_blob<T: DeserializeOwned>(bytes: &[u8], expected_hash: &str) -> Result<T, BlobError> {
    let actual = sha256_hex(bytes);
    if !actual.eq_ignore_ascii_case(expected_hash) {
        return Err(BlobError::HashMismatch {
            expected: expected_hash.to_string(),
            actual,
        });
    }
    serde_json::from_slice(bytes).map_err(|e| BlobError::DecodeFailed(e.to_string()))
}

pub fn load_embedded_swarm_strategy() -> Result<SwarmStrategyProfile, BlobError> {
    let manifest = decode_manifest(MANIFEST_BLOB)?;
    let entry = manifest
        .iter()
        .find(|v| v.id == SWARM_STRATEGY_BLOB_ID)
        .ok_or_else(|| BlobError::BlobNotFound(SWARM_STRATEGY_BLOB_ID.to_string()))?;
    unfold_blob(SWARM_STRATEGY_BLOB, &entry.hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_manifest_entries_fail_closed() {
        let manifest = serde_json::to_vec(&vec![
            BlobManifest {
                id: "swarm_strategy".to_string(),
                hash: "a".repeat(64),
                version: 1,
            },
            BlobManifest {
                id: "swarm_strategy".to_string(),
                hash: "b".repeat(64),
                version: 1,
            },
        ])
        .expect("manifest");
        let err = decode_manifest(&manifest).expect_err("duplicate entries must fail");
        assert!(matches!(
            err,
            BlobError::DuplicateManifestEntry(id) if id == "swarm_strategy"
        ));
    }

    #[test]
    fn unsupported_manifest_versions_fail_closed() {
        let manifest = serde_json::to_vec(&vec![BlobManifest {
            id: "swarm_strategy".to_string(),
            hash: "a".repeat(64),
            version: 2,
        }])
        .expect("manifest");
        let err = decode_manifest(&manifest).expect_err("unsupported versions must fail");
        assert!(matches!(
            err,
            BlobError::UnsupportedManifestVersion { id, version } if id == "swarm_strategy" && version == 2
        ));
    }
}
