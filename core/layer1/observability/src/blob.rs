// SPDX-License-Identifier: Apache-2.0
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

pub const OBS_RUNTIME_BLOB_ID: &str = "observability_runtime_envelope";
pub const OBS_RUNTIME_BLOB: &[u8] = include_bytes!("blobs/observability_runtime_envelope.blob");
pub const MANIFEST_BLOB: &[u8] = include_bytes!("blobs/manifest.blob");
const MAX_BLOB_ID_LEN: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservabilityRuntimeEnvelope {
    pub envelope_id: String,
    pub max_telemetry_overhead_ms: f64,
    pub max_battery_pct_24h: f64,
    pub max_drift_pct: f64,
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

fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                ch,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .collect()
}

fn normalize_blob_id(raw: &str) -> Option<String> {
    let normalized: String = strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect();
    let normalized = normalized.trim();
    if normalized.is_empty() || normalized.len() > MAX_BLOB_ID_LEN {
        return None;
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/'))
    {
        return None;
    }
    Some(normalized.to_string())
}

fn normalize_hash(raw: &str) -> Option<String> {
    let normalized = strip_invisible_unicode(raw).trim().to_ascii_lowercase();
    if normalized.len() != 64 || !normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    Some(normalized)
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
    let rows: Vec<BlobManifest> =
        serde_json::from_slice(bytes).map_err(|e| BlobError::ManifestDecodeFailed(e.to_string()))?;
    let mut merged = BTreeMap::<String, BlobManifest>::new();
    for row in rows {
        let id = normalize_blob_id(&row.id)
            .ok_or_else(|| BlobError::ManifestDecodeFailed("manifest_blob_id_invalid".to_string()))?;
        let hash = normalize_hash(&row.hash)
            .ok_or_else(|| BlobError::ManifestDecodeFailed("manifest_blob_hash_invalid".to_string()))?;
        let normalized = BlobManifest {
            id: id.clone(),
            hash,
            version: row.version,
        };
        match merged.get(&id) {
            Some(existing) if existing.version >= normalized.version => {}
            _ => {
                merged.insert(id, normalized);
            }
        }
    }
    Ok(merged.into_values().collect())
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

pub fn load_embedded_observability_runtime_envelope(
) -> Result<ObservabilityRuntimeEnvelope, BlobError> {
    let manifest = decode_manifest(MANIFEST_BLOB)?;
    let entry = manifest
        .iter()
        .find(|v| v.id == OBS_RUNTIME_BLOB_ID)
        .ok_or_else(|| BlobError::BlobNotFound(OBS_RUNTIME_BLOB_ID.to_string()))?;
    unfold_blob(OBS_RUNTIME_BLOB, &entry.hash)
}
