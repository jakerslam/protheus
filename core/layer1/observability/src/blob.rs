// SPDX-License-Identifier: Apache-2.0
use protheus_nexus_core_v1::{decode_normalized_blob_manifest, normalize_sha256_hash};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{Display, Formatter};

pub const OBS_RUNTIME_BLOB_ID: &str = "observability_runtime_envelope";
pub const OBS_RUNTIME_BLOB: &[u8] = include_bytes!("blobs/observability_runtime_envelope.blob");
pub const MANIFEST_BLOB: &[u8] = include_bytes!("blobs/manifest.blob");
const MAX_BLOB_ID_LEN: usize = 128;
const MAX_MANIFEST_ENTRIES: usize = 1024;
const MAX_PERCENT_PCT: f64 = 100.0;
const MAX_TELEMETRY_OVERHEAD_MS: f64 = 60_000.0;

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
        }
    }
}

impl std::error::Error for BlobError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
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

pub fn load_embedded_observability_runtime_envelope(
) -> Result<ObservabilityRuntimeEnvelope, BlobError> {
    let manifest = decode_manifest(MANIFEST_BLOB)?;
    let entry = manifest
        .iter()
        .find(|v| v.id == OBS_RUNTIME_BLOB_ID)
        .ok_or_else(|| BlobError::BlobNotFound(OBS_RUNTIME_BLOB_ID.to_string()))?;
    let envelope: ObservabilityRuntimeEnvelope = unfold_blob(OBS_RUNTIME_BLOB, &entry.hash)?;
    if envelope.envelope_id.trim().is_empty() {
        return Err(BlobError::DecodeFailed("envelope_id_invalid".to_string()));
    }
    if !envelope.max_telemetry_overhead_ms.is_finite() || envelope.max_telemetry_overhead_ms <= 0.0
    {
        return Err(BlobError::DecodeFailed(
            "max_telemetry_overhead_invalid".to_string(),
        ));
    }
    if envelope.max_telemetry_overhead_ms > MAX_TELEMETRY_OVERHEAD_MS {
        return Err(BlobError::DecodeFailed(
            "max_telemetry_overhead_out_of_bounds".to_string(),
        ));
    }
    if !envelope.max_battery_pct_24h.is_finite()
        || envelope.max_battery_pct_24h <= 0.0
        || envelope.max_battery_pct_24h > 100.0
    {
        return Err(BlobError::DecodeFailed(
            "max_battery_pct_invalid".to_string(),
        ));
    }
    if !envelope.max_drift_pct.is_finite() || envelope.max_drift_pct < 0.0 {
        return Err(BlobError::DecodeFailed("max_drift_pct_invalid".to_string()));
    }
    if envelope.max_drift_pct > MAX_PERCENT_PCT {
        return Err(BlobError::DecodeFailed(
            "max_drift_pct_out_of_bounds".to_string(),
        ));
    }
    Ok(envelope)
}
