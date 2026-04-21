// SPDX-License-Identifier: Apache-2.0
use infring_types::{
    decode_normalized_blob_manifest, normalize_blob_id as normalize_blob_id_token,
    normalize_sha256_hash,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{Display, Formatter};

pub const MOBILE_PROFILE_BLOB_ID: &str = "mobile_runtime_profile";
pub const MOBILE_PROFILE_BLOB: &[u8] = include_bytes!("blobs/mobile_runtime_profile.blob");
pub const MANIFEST_BLOB: &[u8] = include_bytes!("blobs/manifest.blob");
const MAX_BLOB_ID_LEN: usize = 128;
const MAX_MANIFEST_ENTRIES: usize = 1024;
const MAX_MOBILE_CYCLES: u32 = 2_000_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MobileRuntimeProfile {
    pub profile_id: String,
    pub battery_budget_pct_24h: f64,
    pub max_cycles: u32,
    pub enable_background_swarm: bool,
    pub enforce_fail_closed: bool,
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

pub fn fold_blob<T: Serialize>(value: &T, blob_id: &str) -> Result<(Vec<u8>, String), BlobError> {
    if normalize_blob_id_token(blob_id, MAX_BLOB_ID_LEN).is_none() {
        return Err(BlobError::EncodeFailed("blob_id_invalid".to_string()));
    }
    let payload = serde_json::to_vec(value).map_err(|e| BlobError::EncodeFailed(e.to_string()))?;
    let hash = sha256_hex(&payload);
    Ok((payload, hash))
}

pub fn decode_manifest(bytes: &[u8]) -> Result<Vec<BlobManifest>, BlobError> {
    let rows = decode_normalized_blob_manifest(bytes, MAX_BLOB_ID_LEN)
        .map_err(BlobError::ManifestDecodeFailed)?;
    if rows.is_empty() {
        return Err(BlobError::ManifestDecodeFailed(
            "manifest_empty".to_string(),
        ));
    }
    if rows.len() > MAX_MANIFEST_ENTRIES {
        return Err(BlobError::ManifestDecodeFailed(
            "manifest_entry_count_exceeded".to_string(),
        ));
    }
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
    let expected = normalize_sha256_hash(expected_hash)
        .ok_or_else(|| BlobError::DecodeFailed("expected_hash_invalid".to_string()))?;
    if actual != expected {
        return Err(BlobError::HashMismatch { expected, actual });
    }
    serde_json::from_slice(bytes).map_err(|e| BlobError::DecodeFailed(e.to_string()))
}

pub fn load_embedded_mobile_profile() -> Result<MobileRuntimeProfile, BlobError> {
    let manifest = decode_manifest(MANIFEST_BLOB)?;
    let entry = manifest
        .iter()
        .find(|v| v.id == MOBILE_PROFILE_BLOB_ID)
        .ok_or_else(|| BlobError::BlobNotFound(MOBILE_PROFILE_BLOB_ID.to_string()))?;
    let profile: MobileRuntimeProfile = unfold_blob(MOBILE_PROFILE_BLOB, &entry.hash)?;
    if profile.profile_id.trim().is_empty() {
        return Err(BlobError::DecodeFailed("profile_id_invalid".to_string()));
    }
    if !profile.battery_budget_pct_24h.is_finite()
        || profile.battery_budget_pct_24h <= 0.0
        || profile.battery_budget_pct_24h > 100.0
    {
        return Err(BlobError::DecodeFailed(
            "battery_budget_pct_invalid".to_string(),
        ));
    }
    if profile.max_cycles == 0 || profile.max_cycles > MAX_MOBILE_CYCLES {
        return Err(BlobError::DecodeFailed("max_cycles_invalid".to_string()));
    }
    if profile.enable_background_swarm && !profile.enforce_fail_closed {
        return Err(BlobError::DecodeFailed(
            "background_swarm_requires_fail_closed".to_string(),
        ));
    }
    Ok(profile)
}
