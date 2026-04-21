
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
            let signature =
                compute_blob_manifest_signature(blob_id, &hash, BLOB_VERSION, MANIFEST_SIGNING_KEY);
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
    decode_signed_bincode_blob_manifest_with_adapter(
        bytes,
        96,
        MANIFEST_SIGNING_KEY,
        |entry| BlobManifest {
            id: entry.id,
            hash: entry.hash,
            version: entry.version,
            signature: Some(entry.signature),
        },
        BlobError::ManifestDecodeFailed,
    )
}

pub fn load_embedded_heartbeat() -> Result<String, BlobError> {
    let manifest = decode_manifest(BLOB_MANIFEST)?;
    let hash = manifest_hash_for(&manifest, HEARTBEAT_BLOB_ID)?;
    unfold_blob_typed(HEARTBEAT_BLOB_ID, &hash)
}

pub fn load_embedded_execution_replay() -> Result<EmbeddedExecutionReplay, BlobError> {
    let manifest = decode_manifest(BLOB_MANIFEST)?;
    let hash = manifest_hash_for(&manifest, EXECUTION_REPLAY_BLOB_ID)?;
    unfold_blob_typed(EXECUTION_REPLAY_BLOB_ID, &hash)
}

pub fn load_embedded_vault_policy() -> Result<EmbeddedVaultPolicy, BlobError> {
    let manifest = decode_manifest(BLOB_MANIFEST)?;
    let hash = manifest_hash_for(&manifest, VAULT_POLICY_BLOB_ID)?;
    unfold_blob_typed(VAULT_POLICY_BLOB_ID, &hash)
}

pub fn load_embedded_observability_profile() -> Result<EmbeddedObservabilityProfile, BlobError> {
    let manifest = decode_manifest(BLOB_MANIFEST)?;
    let hash = manifest_hash_for(&manifest, OBSERVABILITY_PROFILE_BLOB_ID)?;
    unfold_blob_typed(OBSERVABILITY_PROFILE_BLOB_ID, &hash)
}

pub fn unfold_blob(blob_id: &str, expected_hash: &str) -> Result<Vec<u8>, BlobError> {
    let manifest = decode_manifest(BLOB_MANIFEST)?;
    let blob_bytes =
        embedded_blob_by_id(blob_id).ok_or_else(|| BlobError::UnknownBlob(blob_id.to_string()))?;
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

    let normalized_expected_hash =
        normalize_sha256_hash(expected_hash).unwrap_or_else(|| expected_hash.to_ascii_lowercase());
    if entry.hash != normalized_expected_hash {
        return Err(BlobError::HashMismatch {
            scope: "expected_vs_manifest",
            expected: entry.hash.clone(),
            actual: normalized_expected_hash,
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
    let folded: FoldedBlob = bincode::deserialize(&decompressed)
        .map_err(|e| BlobError::DeserializeFailed(e.to_string()))?;

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
pub fn default_execution_replay_sample() -> EmbeddedExecutionReplay {
    EmbeddedExecutionReplay {
        engine_version: "execution_core_v0.1.0".to_string(),
        workflow_id: "execution_replay_canary".to_string(),
        deterministic_seed: "phase2_seed".to_string(),
        pause_resume_contract: vec![
            "cursor_monotonic".to_string(),
            "digest_sha256_indexed_events".to_string(),
            "replay_drift_zero".to_string(),
            "pause_requires_explicit_step_flag".to_string(),
        ],
        steps: vec![
            EmbeddedExecutionStep {
                id: "collect".to_string(),
                kind: "task".to_string(),
                action: "collect_data".to_string(),
                command: "collect --source=eyes".to_string(),
                pause_after: false,
            },
            EmbeddedExecutionStep {
                id: "score".to_string(),
                kind: "task".to_string(),
                action: "score".to_string(),
                command: "score --strategy=deterministic".to_string(),
                pause_after: true,
            },
            EmbeddedExecutionStep {
                id: "ship".to_string(),
                kind: "task".to_string(),
                action: "ship".to_string(),
                command: "ship --mode=canary".to_string(),
                pause_after: false,
            },
        ],
        receipt_model: EmbeddedExecutionReceiptModel {
            deterministic: true,
            replayable: true,
            digest_algorithm: "sha256(index:event|)".to_string(),
            status_cycle: vec![
                "running".to_string(),
                "paused".to_string(),
                "completed".to_string(),
            ],
            state_fields: vec![
                "cursor".to_string(),
                "paused".to_string(),
                "completed".to_string(),
                "last_step_id".to_string(),
                "processed_step_ids".to_string(),
                "processed_events".to_string(),
                "digest".to_string(),
            ],
        },
    }
}
