// SPDX-License-Identifier: Apache-2.0
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use infring_types::{
    decode_normalized_blob_manifest, normalize_blob_id as normalize_blob_id_token,
    normalize_sha256_hash,
};

pub const GRAPH_POLICY_BLOB_ID: &str = "graph_runtime_policy";
pub const GRAPH_POLICY_BLOB: &[u8] = include_bytes!("blobs/graph_runtime_policy.blob");
pub const MANIFEST_BLOB: &[u8] = include_bytes!("blobs/manifest.blob");
const MAX_BLOB_ID_LEN: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphRuntimePolicy {
    pub policy_id: String,
    pub max_nodes: usize,
    pub max_edges: usize,
    pub allow_cycles: bool,
    pub deterministic_order: bool,
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
    BlobNotFound(String),
    HashMismatch { expected: String, actual: String },
    DecodeFailed(String),
    EncodeFailed(String),
}

impl Display for BlobError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BlobError::ManifestDecodeFailed(msg) => write!(f, "manifest_decode_failed:{msg}"),
            BlobError::BlobNotFound(id) => write!(f, "blob_not_found:{id}"),
            BlobError::HashMismatch { expected, actual } => {
                write!(f, "blob_hash_mismatch expected={expected} actual={actual}")
            }
            BlobError::DecodeFailed(msg) => write!(f, "blob_decode_failed:{msg}"),
            BlobError::EncodeFailed(msg) => write!(f, "blob_encode_failed:{msg}"),
        }
    }
}

impl std::error::Error for BlobError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}


fn normalize_blob_id(raw: &str) -> Option<String> {
    normalize_blob_id_token(raw, MAX_BLOB_ID_LEN)
}


fn normalize_hash(raw: &str) -> Option<String> {
    normalize_sha256_hash(raw)
}


pub fn fold_blob<T: Serialize>(value: &T, blob_id: &str) -> Result<(Vec<u8>, String), BlobError> {
    if normalize_blob_id(blob_id).is_none() {
        return Err(BlobError::EncodeFailed("blob_id_invalid".to_string()));
    }
    let payload = serde_json::to_vec(value).map_err(|e| BlobError::EncodeFailed(e.to_string()))?;
    let hash = sha256_hex(&payload);
    Ok((payload, hash))
}

pub fn generate_manifest(blobs: &[(&str, &[u8])]) -> Vec<BlobManifest> {
    let mut merged = BTreeMap::<String, BlobManifest>::new();
    for (raw_id, bytes) in blobs {
        let Some(id) = normalize_blob_id(raw_id) else {
            continue;
        };
        merged.insert(
            id.clone(),
            BlobManifest {
                id,
                hash: sha256_hex(bytes),
                version: 1,
            },
        );
    }
    merged.into_values().collect()
}

pub fn decode_manifest(bytes: &[u8]) -> Result<Vec<BlobManifest>, BlobError> {
    let rows = decode_normalized_blob_manifest(bytes, MAX_BLOB_ID_LEN)
        .map_err(BlobError::ManifestDecodeFailed)?;
    Ok(rows
        .into_iter()
        .map(|row| BlobManifest {
            id: row.id,
            hash: row.hash,
            version: row.version,
        })
        .collect())
}


pub fn unfold_blob<T: DeserializeOwned>(bytes: &[u8], expected_hash: &str) -> Result<T, BlobError> {
    let actual = sha256_hex(bytes);
    let expected = normalize_hash(expected_hash)
        .ok_or_else(|| BlobError::DecodeFailed("expected_hash_invalid".to_string()))?;
    if actual != expected {
        return Err(BlobError::HashMismatch {
            expected,
            actual,
        });
    }
    serde_json::from_slice(bytes).map_err(|e| BlobError::DecodeFailed(e.to_string()))
}

pub fn load_embedded_graph_policy() -> Result<GraphRuntimePolicy, BlobError> {
    let manifest = decode_manifest(MANIFEST_BLOB)?;
    let entry = manifest
        .iter()
        .find(|v| v.id == GRAPH_POLICY_BLOB_ID)
        .ok_or_else(|| BlobError::BlobNotFound(GRAPH_POLICY_BLOB_ID.to_string()))?;
    unfold_blob(GRAPH_POLICY_BLOB, &entry.hash)
}
