
fn normalize_cycle_request_with_contract(
    request: CycleRequest,
    strict_contract: bool,
) -> Result<CycleRequest, SeedError> {
    let normalized = normalize_cycle_request(request.clone());
    if strict_contract && normalized.drift_overrides.len() != request.drift_overrides.len() {
        return Err(SeedError::InvalidRequest(
            "drift_overrides_modified_under_strict_contract".to_string(),
        ));
    }
    Ok(normalized)
}

fn manifest_signature(id: &str, hash: &str, version: u32) -> String {
    let payload = format!("{id}:{hash}:{version}:{MANIFEST_SIGNING_KEY}");
    sha256_hex(payload.as_bytes())
}

fn verify_manifest_entry(entry: &BlobManifestEntry) -> Result<(), SeedError> {
    let actual = entry
        .signature
        .as_ref()
        .ok_or_else(|| SeedError::MissingSignature(entry.id.clone()))?;
    let expected = manifest_signature(&entry.id, &entry.hash, entry.version);
    if !actual.eq_ignore_ascii_case(&expected) {
        return Err(SeedError::SignatureMismatch {
            id: entry.id.clone(),
            expected,
            actual: actual.clone(),
        });
    }
    Ok(())
}

pub fn fold_blob<T: Serialize>(data: &T, blob_id: &str) -> Result<(Vec<u8>, String), SeedError> {
    if blob_id.trim().is_empty() {
        return Err(SeedError::InvalidBlobId);
    }
    let payload =
        bincode::serialize(data).map_err(|err| SeedError::SerializeFailed(err.to_string()))?;
    let folded = FoldedBlob {
        id: blob_id.to_string(),
        version: BLOB_VERSION,
        payload,
    };
    let encoded =
        bincode::serialize(&folded).map_err(|err| SeedError::SerializeFailed(err.to_string()))?;
    let compressed = Encoder::new()
        .compress_vec(&encoded)
        .map_err(|err| SeedError::CompressFailed(err.to_string()))?;
    let hash = sha256_hex(&compressed);
    Ok((compressed, hash))
}

pub fn generate_manifest(blobs: &[(&str, &[u8])]) -> Vec<BlobManifestEntry> {
    blobs
        .iter()
        .map(|(blob_id, blob_bytes)| {
            let hash = sha256_hex(blob_bytes);
            BlobManifestEntry {
                id: (*blob_id).to_string(),
                hash: hash.clone(),
                version: BLOB_VERSION,
                signature: Some(manifest_signature(blob_id, &hash, BLOB_VERSION)),
            }
        })
        .collect()
}

pub fn encode_manifest(entries: &[BlobManifestEntry]) -> Result<Vec<u8>, SeedError> {
    bincode::serialize(entries).map_err(|err| SeedError::ManifestEncodeFailed(err.to_string()))
}

pub fn decode_manifest(bytes: &[u8]) -> Result<Vec<BlobManifestEntry>, SeedError> {
    bincode::deserialize(bytes).map_err(|err| SeedError::ManifestDecodeFailed(err.to_string()))
}

pub fn unfold_blob_typed<T: DeserializeOwned>(
    blob_id: &str,
    expected_hash: &str,
    blob_bytes: &[u8],
    manifest: &[BlobManifestEntry],
) -> Result<T, SeedError> {
    let entry = manifest
        .iter()
        .find(|entry| entry.id == blob_id)
        .ok_or_else(|| SeedError::MissingManifestEntry(blob_id.to_string()))?;

    verify_manifest_entry(entry)?;

    if !entry.hash.eq_ignore_ascii_case(expected_hash) {
        return Err(SeedError::HashMismatch {
            scope: "expected_vs_manifest",
            expected: entry.hash.clone(),
            actual: expected_hash.to_string(),
        });
    }

    let actual_hash = sha256_hex(blob_bytes);
    if !actual_hash.eq_ignore_ascii_case(&entry.hash) {
        return Err(SeedError::HashMismatch {
            scope: "blob_vs_manifest",
            expected: entry.hash.clone(),
            actual: actual_hash,
        });
    }

    let decompressed = Decoder::new()
        .decompress_vec(blob_bytes)
        .map_err(|err| SeedError::DecompressFailed(err.to_string()))?;
    let folded: FoldedBlob = bincode::deserialize(&decompressed)
        .map_err(|err| SeedError::DeserializeFailed(err.to_string()))?;

    if folded.id != blob_id {
        return Err(SeedError::IdMismatch {
            expected: blob_id.to_string(),
            actual: folded.id,
        });
    }

    if folded.version != BLOB_VERSION {
        return Err(SeedError::UnsupportedVersion {
            id: blob_id.to_string(),
            version: folded.version,
        });
    }

    bincode::deserialize(&folded.payload)
        .map_err(|err| SeedError::DeserializeFailed(err.to_string()))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn sanitize_blob_root_override(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_BLOB_ROOT_CHARS {
        return None;
    }
    if trimmed
        .chars()
        .any(|ch| ch == '\0' || ch == '\n' || ch == '\r')
    {
        return None;
    }
    let path = PathBuf::from(trimmed);
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return None;
    }
    Some(path)
}

fn blob_root() -> PathBuf {
    if let Ok(explicit) = std::env::var("INFRING_SINGULARITY_BLOB_DIR") {
        if let Some(safe) = sanitize_blob_root_override(&explicit) {
            return safe;
        }
    }
    repo_root().join("client/runtime/systems/singularity_seed/blobs")
}

fn manifest_path(root: &Path) -> PathBuf {
    root.join("manifest.blob")
}

fn loop_blob_path(root: &Path, loop_id: &str) -> PathBuf {
    root.join(format!("{loop_id}.blob"))
}
