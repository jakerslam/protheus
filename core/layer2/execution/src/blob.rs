// SPDX-License-Identifier: Apache-2.0
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
#[cfg(test)]
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt::{Display, Formatter};
use infring_types::decode_normalized_blob_manifest;

pub const EXECUTION_PROFILE_BLOB_ID: &str = "execution_runtime_profile";
pub const EXECUTION_PROFILE_BLOB: &[u8] = include_bytes!("blobs/execution_runtime_profile.blob");
pub const MANIFEST_BLOB: &[u8] = include_bytes!("blobs/manifest.blob");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionRuntimeProfile {
    pub profile_id: String,
    pub max_steps: usize,
    pub allow_resume: bool,
    pub pause_gate_enabled: bool,
    pub deterministic_receipts_only: bool,
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

#[cfg(test)]
pub fn normalize_blob_status(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "ok" | "success" | "succeeded" | "loaded" => "success",
        "timeout" | "timed_out" | "timed-out" => "timeout",
        "throttled" | "rate_limited" | "rate-limited" | "429" => "throttled",
        _ => "error",
    }
}

#[cfg(test)]
fn sanitize_token(raw: &str, max_len: usize) -> String {
    let mut out = String::with_capacity(raw.len().min(max_len));
    let mut prev_underscore = false;
    for ch in raw.trim().to_ascii_lowercase().chars() {
        let keep = ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.');
        if keep {
            out.push(ch);
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
        if out.len() >= max_len {
            break;
        }
    }
    while out.starts_with('_') {
        out.remove(0);
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

#[cfg(test)]
pub fn blob_execution_receipt(blob_id: &str, status: &str, error_kind: Option<&str>) -> Value {
    let normalized_status = normalize_blob_status(status);
    let normalized_blob_id = sanitize_token(blob_id, 80);
    let normalized_error_kind = error_kind.and_then(|raw| {
        let token = sanitize_token(raw, 64);
        if token.is_empty() { None } else { Some(token) }
    });
    let seed = json!({
        "blob_id": normalized_blob_id,
        "status": normalized_status,
        "error_kind": normalized_error_kind
    });
    let digest = sha256_hex(serde_json::to_string(&seed).unwrap_or_default().as_bytes());
    json!({
        "call_id": format!("blob-{}", &digest[..16]),
        "status": normalized_status,
        "error_kind": normalized_error_kind,
        "telemetry": {
            "duration_ms": 0,
            "tokens_used": 0
        }
    })
}

pub fn fold_blob<T: Serialize>(value: &T, _blob_id: &str) -> Result<(Vec<u8>, String), BlobError> {
    let payload = serde_json::to_vec(value).map_err(|e| BlobError::DecodeFailed(e.to_string()))?;
    let hash = sha256_hex(&payload);
    Ok((payload, hash))
}

pub fn decode_manifest(bytes: &[u8]) -> Result<Vec<BlobManifest>, BlobError> {
    decode_normalized_blob_manifest(bytes, 96)
        .map_err(BlobError::ManifestDecodeFailed)
        .map(|rows| {
            rows.into_iter()
                .map(|entry| BlobManifest {
                    id: entry.id,
                    hash: entry.hash,
                    version: entry.version,
                })
                .collect::<Vec<_>>()
        })
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

pub fn load_embedded_execution_profile() -> Result<ExecutionRuntimeProfile, BlobError> {
    let manifest = decode_manifest(MANIFEST_BLOB)?;
    let entry = manifest
        .iter()
        .find(|v| v.id == EXECUTION_PROFILE_BLOB_ID)
        .ok_or_else(|| BlobError::BlobNotFound(EXECUTION_PROFILE_BLOB_ID.to_string()))?;
    unfold_blob(EXECUTION_PROFILE_BLOB, &entry.hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_execution_receipt_status_normalization() {
        assert_eq!(normalize_blob_status("ok"), "success");
        assert_eq!(normalize_blob_status("rate_limited"), "throttled");
        assert_eq!(normalize_blob_status("timed-out"), "timeout");
        assert_eq!(normalize_blob_status("bad_status"), "error");
    }

    #[test]
    fn blob_execution_receipt_is_deterministic_for_same_seed() {
        let left = blob_execution_receipt("Execution Runtime Profile", "ok", Some("Policy Denied"));
        let right = blob_execution_receipt("Execution Runtime Profile", "success", Some("policy_denied"));
        assert_eq!(left.get("call_id"), right.get("call_id"));
        assert_eq!(left.get("status").and_then(Value::as_str), Some("success"));
        assert_eq!(
            left.get("error_kind").and_then(Value::as_str),
            Some("policy_denied")
        );
    }
}
