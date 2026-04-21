
#[cfg(not(target_arch = "wasm32"))]
pub fn default_vault_policy_sample() -> EmbeddedVaultPolicy {
    EmbeddedVaultPolicy {
        policy_id: "vault_policy_primary".to_string(),
        version: 1,
        key_domain: "protheus_runtime_vault".to_string(),
        cryptographic_profile: "fhe_bfv+zkp_groth16".to_string(),
        attestation_chain: vec![
            "hsm_root_attestation".to_string(),
            "runtime_measurement_attestation".to_string(),
            "operator_dual_control_attestation".to_string(),
        ],
        auto_rotate: EmbeddedVaultAutoRotatePolicy {
            enabled: true,
            rotate_after_hours: 24,
            max_key_age_hours: 72,
            grace_window_minutes: 20,
            quorum_required: 2,
            emergency_rotate_on_tamper: true,
        },
        rules: vec![
            EmbeddedVaultPolicyRule {
                id: "vault.zk.required".to_string(),
                objective: "Every seal/unseal request carries non-interactive zero-knowledge proof."
                    .to_string(),
                zk_requirement: "proof_required_for_key_open".to_string(),
                fhe_requirement: "ciphertext_only_in_compute_lane".to_string(),
                severity: "critical".to_string(),
                fail_closed: true,
            },
            EmbeddedVaultPolicyRule {
                id: "vault.fhe.policy".to_string(),
                objective: "Homomorphic operations remain bounded and deterministic."
                    .to_string(),
                zk_requirement: "proof_links_ciphertext_to_policy".to_string(),
                fhe_requirement: "noise_budget_min_threshold".to_string(),
                severity: "high".to_string(),
                fail_closed: true,
            },
            EmbeddedVaultPolicyRule {
                id: "vault.rotation.window".to_string(),
                objective:
                    "Automatic key rotation executes before max age or immediately after tamper signal."
                        .to_string(),
                zk_requirement: "proof_of_previous_key_revocation".to_string(),
                fhe_requirement: "reencrypt_on_rotate".to_string(),
                severity: "critical".to_string(),
                fail_closed: true,
            },
            EmbeddedVaultPolicyRule {
                id: "vault.audit.trace".to_string(),
                objective: "Every key event emits signed immutable receipt.".to_string(),
                zk_requirement: "proof_receipt_binding".to_string(),
                fhe_requirement: "receipt_contains_ciphertext_digest".to_string(),
                severity: "medium".to_string(),
                fail_closed: true,
            },
        ],
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn default_observability_profile_sample() -> EmbeddedObservabilityProfile {
    EmbeddedObservabilityProfile {
        profile_id: "observability_profile_primary".to_string(),
        version: 1,
        red_legion_trace_channels: vec![
            "runtime.guardrails".to_string(),
            "lane.integrity".to_string(),
            "chaos.replay".to_string(),
            "sovereignty.index".to_string(),
        ],
        allowed_emitters: vec![
            "client/runtime/systems/observability".to_string(),
            "client/runtime/systems/red_legion".to_string(),
            "client/runtime/systems/security".to_string(),
            "core/layer1/observability".to_string(),
        ],
        stream_policy: EmbeddedTraceStreamPolicy {
            trace_window_ms: 1000,
            max_events_per_window: 1024,
            min_sampling_rate_pct: 25,
            redact_fields: vec![
                "secret".to_string(),
                "token".to_string(),
                "private_key".to_string(),
                "api_key".to_string(),
            ],
            require_signature: true,
        },
        sovereignty_scorer: EmbeddedSovereigntyScorer {
            integrity_weight_pct: 45,
            continuity_weight_pct: 25,
            reliability_weight_pct: 20,
            chaos_penalty_pct: 10,
            fail_closed_threshold_pct: 60,
        },
        chaos_hooks: vec![
            EmbeddedChaosHook {
                id: "hook.fail_closed.on_tamper".to_string(),
                condition: "event.severity == critical && event.tag == tamper".to_string(),
                action: "trip_fail_closed".to_string(),
                severity: "critical".to_string(),
                enabled: true,
            },
            EmbeddedChaosHook {
                id: "hook.rate_limit.on_storm".to_string(),
                condition: "window.events > max_events_per_window".to_string(),
                action: "drop_low_priority".to_string(),
                severity: "high".to_string(),
                enabled: true,
            },
            EmbeddedChaosHook {
                id: "hook.score_penalty.on_drift".to_string(),
                condition: "replay.drift > 0".to_string(),
                action: "apply_chaos_penalty".to_string(),
                severity: "medium".to_string(),
                enabled: true,
            },
        ],
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_embedded_blob_assets(heartbeat_sample: &str) -> Result<BlobPackReport, BlobError> {
    let heartbeat_payload = if heartbeat_sample.trim().is_empty() {
        default_heartbeat_sample()
    } else {
        heartbeat_sample.to_string()
    };
    let execution_payload = default_execution_replay_sample();
    let vault_policy_payload = default_vault_policy_sample();
    let observability_payload = default_observability_profile_sample();

    let (heartbeat_blob, heartbeat_hash) = fold_blob(&heartbeat_payload, HEARTBEAT_BLOB_ID)?;
    let (execution_blob, execution_hash) = fold_blob(&execution_payload, EXECUTION_REPLAY_BLOB_ID)?;
    let (vault_policy_blob, vault_policy_hash) =
        fold_blob(&vault_policy_payload, VAULT_POLICY_BLOB_ID)?;
    let (observability_blob, observability_hash) =
        fold_blob(&observability_payload, OBSERVABILITY_PROFILE_BLOB_ID)?;
    let manifest = generate_manifest(&[
        (HEARTBEAT_BLOB_ID, heartbeat_blob.as_slice()),
        (EXECUTION_REPLAY_BLOB_ID, execution_blob.as_slice()),
        (VAULT_POLICY_BLOB_ID, vault_policy_blob.as_slice()),
        (OBSERVABILITY_PROFILE_BLOB_ID, observability_blob.as_slice()),
    ]);
    let manifest_bytes = encode_manifest(&manifest)?;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let heartbeat_path = root.join("src/blobs/heartbeat_sample.blob");
    let execution_path = root.join("src/blobs/execution_replay.blob");
    let vault_policy_path = root.join("src/blobs/vault_policy.blob");
    let observability_path = root.join("src/blobs/observability_profile.blob");
    let manifest_path = root.join("src/blobs/manifest.blob");

    if let Some(parent) = heartbeat_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| BlobError::IoFailed(e.to_string()))?;
    }

    std::fs::write(&heartbeat_path, &heartbeat_blob)
        .map_err(|e| BlobError::IoFailed(e.to_string()))?;
    std::fs::write(&execution_path, &execution_blob)
        .map_err(|e| BlobError::IoFailed(e.to_string()))?;
    std::fs::write(&vault_policy_path, &vault_policy_blob)
        .map_err(|e| BlobError::IoFailed(e.to_string()))?;
    std::fs::write(&observability_path, &observability_blob)
        .map_err(|e| BlobError::IoFailed(e.to_string()))?;
    std::fs::write(&manifest_path, &manifest_bytes)
        .map_err(|e| BlobError::IoFailed(e.to_string()))?;

    Ok(BlobPackReport {
        manifest_path: manifest_path.display().to_string(),
        manifest_bytes: manifest_bytes.len(),
        artifacts: vec![
            BlobArtifactDigest {
                id: HEARTBEAT_BLOB_ID.to_string(),
                path: heartbeat_path.display().to_string(),
                bytes: heartbeat_blob.len(),
                hash: heartbeat_hash,
            },
            BlobArtifactDigest {
                id: EXECUTION_REPLAY_BLOB_ID.to_string(),
                path: execution_path.display().to_string(),
                bytes: execution_blob.len(),
                hash: execution_hash,
            },
            BlobArtifactDigest {
                id: VAULT_POLICY_BLOB_ID.to_string(),
                path: vault_policy_path.display().to_string(),
                bytes: vault_policy_blob.len(),
                hash: vault_policy_hash,
            },
            BlobArtifactDigest {
                id: OBSERVABILITY_PROFILE_BLOB_ID.to_string(),
                path: observability_path.display().to_string(),
                bytes: observability_blob.len(),
                hash: observability_hash,
            },
        ],
    })
}

#[cfg(target_arch = "wasm32")]
pub fn write_embedded_blob_assets(_heartbeat_sample: &str) -> Result<BlobPackReport, BlobError> {
    Err(BlobError::IoFailed(
        "write_embedded_blob_assets_unavailable_on_wasm".to_string(),
    ))
}

fn embedded_blob_by_id(blob_id: &str) -> Option<&'static [u8]> {
    match blob_id {
        HEARTBEAT_BLOB_ID => Some(HEARTBEAT_BLOB),
        EXECUTION_REPLAY_BLOB_ID => Some(EXECUTION_REPLAY_BLOB),
        VAULT_POLICY_BLOB_ID => Some(VAULT_POLICY_BLOB),
        OBSERVABILITY_PROFILE_BLOB_ID => Some(OBSERVABILITY_PROFILE_BLOB),
        _ => None,
    }
}

fn manifest_hash_for(manifest: &[BlobManifest], blob_id: &str) -> Result<String, BlobError> {
    manifest
        .iter()
        .find(|entry| entry.id == blob_id)
        .map(|entry| entry.hash.clone())
        .ok_or_else(|| BlobError::MissingManifestEntry(blob_id.to_string()))
}

fn verify_manifest_entry(entry: &BlobManifest) -> Result<(), BlobError> {
    let actual = entry
        .signature
        .as_ref()
        .ok_or_else(|| BlobError::MissingSignature(entry.id.clone()))?;
    let expected = compute_blob_manifest_signature(
        &entry.id,
        &entry.hash,
        entry.version,
        MANIFEST_SIGNING_KEY,
    );
    if !actual.eq_ignore_ascii_case(&expected) {
        return Err(BlobError::SignatureMismatch {
            id: entry.id.clone(),
            expected,
            actual: actual.clone(),
        });
    }
    Ok(())
}
