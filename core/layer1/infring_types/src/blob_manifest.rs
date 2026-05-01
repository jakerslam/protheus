use crate::strip_invisible_unicode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn has_parent_segment(raw: &str) -> bool {
    raw.split(['/', '\\']).any(|segment| segment.trim() == "..")
}

fn has_ambiguous_segment(raw: &str) -> bool {
    raw.split('/')
        .any(|segment| matches!(segment.trim(), "" | "."))
}

fn is_absolute_like_blob_id(raw: &str) -> bool {
    raw.starts_with('/')
        || raw.starts_with('\\')
        || raw.starts_with("//")
        || raw.starts_with("\\\\")
        || raw.get(1..3) == Some(":\\")
        || raw.get(1..3) == Some(":/")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedBlobManifestEntry {
    pub id: String,
    pub hash: String,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct RawSignedBlobManifestEntry {
    pub id: String,
    pub hash: String,
    pub version: u32,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedSignedBlobManifestEntry {
    pub id: String,
    pub hash: String,
    pub version: u32,
    pub signature: String,
}

pub fn normalize_blob_id(raw: &str, max_len: usize) -> Option<String> {
    let normalized: String = strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect();
    let normalized = normalized.trim();
    if normalized.is_empty() || normalized.len() > max_len {
        return None;
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/'))
    {
        return None;
    }
    if has_parent_segment(normalized)
        || has_ambiguous_segment(normalized)
        || is_absolute_like_blob_id(normalized)
    {
        return None;
    }
    Some(normalized.to_string())
}

pub fn normalize_sha256_hash(raw: &str) -> Option<String> {
    let normalized = strip_invisible_unicode(raw).trim().to_ascii_lowercase();
    if normalized.len() != 64 || !normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    Some(normalized)
}

pub fn decode_normalized_blob_manifest(
    bytes: &[u8],
    max_blob_id_len: usize,
) -> Result<Vec<NormalizedBlobManifestEntry>, String> {
    let rows: Vec<NormalizedBlobManifestEntry> =
        serde_json::from_slice(bytes).map_err(|err| err.to_string())?;
    let mut merged = BTreeMap::<String, NormalizedBlobManifestEntry>::new();
    for row in rows {
        let id = normalize_blob_id(&row.id, max_blob_id_len)
            .ok_or_else(|| "manifest_blob_id_invalid".to_string())?;
        let hash = normalize_sha256_hash(&row.hash)
            .ok_or_else(|| "manifest_blob_hash_invalid".to_string())?;
        let normalized = NormalizedBlobManifestEntry {
            id: id.clone(),
            hash,
            version: row.version,
        };
        match merged.get(&id) {
            Some(existing)
                if existing.version == normalized.version && existing.hash != normalized.hash =>
            {
                return Err(format!("manifest_conflicting_hash:{id}"));
            }
            Some(existing) if existing.version >= normalized.version => {}
            _ => {
                merged.insert(id, normalized);
            }
        }
    }
    Ok(merged.into_values().collect())
}

pub fn compute_blob_manifest_signature(
    id: &str,
    hash: &str,
    version: u32,
    signing_key: &str,
) -> String {
    let to_sign = format!("{id}:{hash}:{version}:{signing_key}");
    let mut hasher = Sha256::new();
    hasher.update(to_sign.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn decode_normalized_signed_bincode_blob_manifest(
    bytes: &[u8],
    max_blob_id_len: usize,
    signing_key: &str,
) -> Result<Vec<NormalizedSignedBlobManifestEntry>, String> {
    let rows: Vec<RawSignedBlobManifestEntry> =
        bincode::deserialize(bytes).map_err(|err| err.to_string())?;
    let mut normalized = BTreeMap::<String, NormalizedSignedBlobManifestEntry>::new();
    for row in rows {
        let id = normalize_blob_id(&row.id, max_blob_id_len)
            .ok_or_else(|| "manifest_blob_id_invalid".to_string())?;
        let hash = normalize_sha256_hash(&row.hash)
            .ok_or_else(|| "manifest_blob_hash_invalid".to_string())?;
        let signature = row
            .signature
            .as_deref()
            .and_then(normalize_sha256_hash)
            .ok_or_else(|| "manifest_signature_invalid".to_string())?;
        let expected = compute_blob_manifest_signature(&id, &hash, row.version, signing_key);
        if signature != expected {
            return Err(format!("manifest_signature_mismatch:{id}"));
        }
        let next = NormalizedSignedBlobManifestEntry {
            id,
            hash,
            version: row.version,
            signature,
        };
        match normalized.get(&next.id) {
            Some(existing)
                if existing.version == next.version
                    && (existing.hash != next.hash || existing.signature != next.signature) =>
            {
                return Err(format!("manifest_conflicting_signed_entry:{}", next.id));
            }
            Some(existing) if existing.version >= next.version => {}
            _ => {
                normalized.insert(next.id.clone(), next);
            }
        }
    }
    Ok(normalized.into_values().collect())
}

pub fn decode_signed_bincode_blob_manifest_with_adapter<T, E, F, G>(
    bytes: &[u8],
    max_blob_id_len: usize,
    signing_key: &str,
    adapt_entry: F,
    map_error: G,
) -> Result<Vec<T>, E>
where
    F: FnMut(NormalizedSignedBlobManifestEntry) -> T,
    G: FnOnce(String) -> E,
{
    match decode_normalized_signed_bincode_blob_manifest(bytes, max_blob_id_len, signing_key) {
        Ok(rows) => Ok(rows.into_iter().map(adapt_entry).collect()),
        Err(err) => Err(map_error(err)),
    }
}
