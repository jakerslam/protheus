use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use snap::raw::{Decoder, Encoder};
use std::fmt::{Display, Formatter};

pub const HEARTBEAT_BLOB_ID: &str = "heartbeat_sample";
pub const BLOB_VERSION: u32 = 1;

pub const HEARTBEAT_BLOB: &[u8] = include_bytes!("blobs/heartbeat_sample.blob");
pub const BLOB_MANIFEST: &[u8] = include_bytes!("blobs/manifest.blob");

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

#[derive(Debug, Clone)]
pub struct BlobPackReport {
    pub blob_path: String,
    pub manifest_path: String,
    pub blob_bytes: usize,
    pub manifest_bytes: usize,
    pub blob_hash: String,
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

pub fn fold_blob<T: Serialize>(data: &T, blob_id: &str) -> Result<(Vec<u8>, String), BlobError> {
    if blob_id.trim().is_empty() {
        return Err(BlobError::InvalidBlobId);
    }

    let payload =
        bincode::serialize(data).map_err(|e| BlobError::SerializeFailed(e.to_string()))?;
    let folded = FoldedBlob {
        id: blob_id.to_string(),
        version: BLOB_VERSION,
        payload,
    };
    let encoded =
        bincode::serialize(&folded).map_err(|e| BlobError::SerializeFailed(e.to_string()))?;
    let compressed = Encoder::new()
        .compress_vec(&encoded)
        .map_err(|e| BlobError::CompressFailed(e.to_string()))?;
    let hash = sha256_hex(&compressed);
    Ok((compressed, hash))
}

pub fn generate_manifest(blobs: &[(&str, &[u8])]) -> Vec<BlobManifest> {
    blobs
        .iter()
        .map(|(blob_id, blob_bytes)| {
            let hash = sha256_hex(blob_bytes);
            let signature = manifest_signature(blob_id, &hash, BLOB_VERSION);
            BlobManifest {
                id: (*blob_id).to_string(),
                hash,
                version: BLOB_VERSION,
                signature: Some(signature),
            }
        })
        .collect()
}

pub fn encode_manifest(entries: &[BlobManifest]) -> Result<Vec<u8>, BlobError> {
    bincode::serialize(entries).map_err(|e| BlobError::ManifestEncodeFailed(e.to_string()))
}

pub fn decode_manifest(bytes: &[u8]) -> Result<Vec<BlobManifest>, BlobError> {
    bincode::deserialize(bytes).map_err(|e| BlobError::ManifestDecodeFailed(e.to_string()))
}

pub fn load_embedded_heartbeat() -> Result<String, BlobError> {
    let manifest = decode_manifest(BLOB_MANIFEST)?;
    let heartbeat_entry = manifest
        .iter()
        .find(|entry| entry.id == HEARTBEAT_BLOB_ID)
        .ok_or_else(|| BlobError::MissingManifestEntry(HEARTBEAT_BLOB_ID.to_string()))?;

    unfold_blob_typed(HEARTBEAT_BLOB_ID, &heartbeat_entry.hash)
}

pub fn unfold_blob(blob_id: &str, expected_hash: &str) -> Result<Vec<u8>, BlobError> {
    let manifest = decode_manifest(BLOB_MANIFEST)?;
    let blob_bytes = embedded_blob_by_id(blob_id)
        .ok_or_else(|| BlobError::UnknownBlob(blob_id.to_string()))?;
    unfold_blob_from_parts(blob_id, expected_hash, blob_bytes, &manifest)
}

pub fn unfold_blob_from_parts(
    blob_id: &str,
    expected_hash: &str,
    blob_bytes: &[u8],
    manifest: &[BlobManifest],
) -> Result<Vec<u8>, BlobError> {
    let entry = manifest
        .iter()
        .find(|entry| entry.id == blob_id)
        .ok_or_else(|| BlobError::MissingManifestEntry(blob_id.to_string()))?;

    verify_manifest_entry(entry)?;

    if !entry.hash.eq_ignore_ascii_case(expected_hash) {
        return Err(BlobError::HashMismatch {
            scope: "expected_vs_manifest",
            expected: entry.hash.clone(),
            actual: expected_hash.to_string(),
        });
    }

    let actual_hash = sha256_hex(blob_bytes);
    if !actual_hash.eq_ignore_ascii_case(&entry.hash) {
        return Err(BlobError::HashMismatch {
            scope: "blob_vs_manifest",
            expected: entry.hash.clone(),
            actual: actual_hash,
        });
    }

    let decompressed = Decoder::new()
        .decompress_vec(blob_bytes)
        .map_err(|e| BlobError::DecompressFailed(e.to_string()))?;
    let folded: FoldedBlob =
        bincode::deserialize(&decompressed).map_err(|e| BlobError::DeserializeFailed(e.to_string()))?;

    if folded.id != blob_id {
        return Err(BlobError::IdMismatch {
            expected: blob_id.to_string(),
            actual: folded.id,
        });
    }
    if folded.version != BLOB_VERSION {
        return Err(BlobError::UnsupportedVersion {
            id: blob_id.to_string(),
            version: folded.version,
        });
    }

    Ok(folded.payload)
}

pub fn unfold_blob_typed<T: DeserializeOwned>(
    blob_id: &str,
    expected_hash: &str,
) -> Result<T, BlobError> {
    let payload = unfold_blob(blob_id, expected_hash)?;
    bincode::deserialize(&payload).map_err(|e| BlobError::DeserializeFailed(e.to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn default_heartbeat_sample() -> String {
    "Read HEARTBEAT.md if it exists. Follow it strictly. If nothing needs attention, reply HEARTBEAT_OK.".to_string()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_embedded_heartbeat_assets(sample: &str) -> Result<BlobPackReport, BlobError> {
    let payload = if sample.trim().is_empty() {
        default_heartbeat_sample()
    } else {
        sample.to_string()
    };

    let (blob, hash) = fold_blob(&payload, HEARTBEAT_BLOB_ID)?;
    let manifest = generate_manifest(&[(HEARTBEAT_BLOB_ID, blob.as_slice())]);
    let manifest_bytes = encode_manifest(&manifest)?;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let blob_path = root.join("src/blobs/heartbeat_sample.blob");
    let manifest_path = root.join("src/blobs/manifest.blob");

    if let Some(parent) = blob_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| BlobError::IoFailed(e.to_string()))?;
    }

    std::fs::write(&blob_path, &blob).map_err(|e| BlobError::IoFailed(e.to_string()))?;
    std::fs::write(&manifest_path, &manifest_bytes).map_err(|e| BlobError::IoFailed(e.to_string()))?;

    Ok(BlobPackReport {
        blob_path: blob_path.display().to_string(),
        manifest_path: manifest_path.display().to_string(),
        blob_bytes: blob.len(),
        manifest_bytes: manifest_bytes.len(),
        blob_hash: hash,
    })
}

#[cfg(target_arch = "wasm32")]
pub fn write_embedded_heartbeat_assets(_sample: &str) -> Result<BlobPackReport, BlobError> {
    Err(BlobError::IoFailed(
        "write_embedded_heartbeat_assets_unavailable_on_wasm".to_string(),
    ))
}

fn embedded_blob_by_id(blob_id: &str) -> Option<&'static [u8]> {
    match blob_id {
        HEARTBEAT_BLOB_ID => Some(HEARTBEAT_BLOB),
        _ => None,
    }
}

fn manifest_signature(id: &str, hash: &str, version: u32) -> String {
    let to_sign = format!("{id}:{hash}:{version}:{MANIFEST_SIGNING_KEY}");
    sha256_hex(to_sign.as_bytes())
}

fn verify_manifest_entry(entry: &BlobManifest) -> Result<(), BlobError> {
    let actual = entry
        .signature
        .as_ref()
        .ok_or_else(|| BlobError::MissingSignature(entry.id.clone()))?;
    let expected = manifest_signature(&entry.id, &entry.hash, entry.version);
    if !actual.eq_ignore_ascii_case(&expected) {
        return Err(BlobError::SignatureMismatch {
            id: entry.id.clone(),
            expected,
            actual: actual.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_embed_unfold_mock_heartbeat_parity() {
        let input = "# HEARTBEAT\\n- check inbox\\n- ship artifacts".to_string();
        let (blob, hash) = fold_blob(&input, HEARTBEAT_BLOB_ID).expect("fold should succeed");
        let manifest = generate_manifest(&[(HEARTBEAT_BLOB_ID, blob.as_slice())]);
        let payload = unfold_blob_from_parts(HEARTBEAT_BLOB_ID, &hash, &blob, &manifest)
            .expect("unfold should succeed");
        let decoded: String = bincode::deserialize(&payload).expect("decode should succeed");
        assert_eq!(decoded, input);
    }

    #[test]
    fn embedded_heartbeat_blob_loads() {
        let heartbeat = load_embedded_heartbeat().expect("embedded heartbeat should load");
        assert!(!heartbeat.trim().is_empty());
        assert!(heartbeat.to_ascii_uppercase().contains("HEARTBEAT"));
    }
}
