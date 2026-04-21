
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
    fn fold_embed_unfold_execution_replay_parity() {
        let input = default_execution_replay_sample();
        let (blob, hash) =
            fold_blob(&input, EXECUTION_REPLAY_BLOB_ID).expect("fold should succeed");
        let manifest = generate_manifest(&[(EXECUTION_REPLAY_BLOB_ID, blob.as_slice())]);
        let payload = unfold_blob_from_parts(EXECUTION_REPLAY_BLOB_ID, &hash, &blob, &manifest)
            .expect("unfold should succeed");
        let decoded: EmbeddedExecutionReplay =
            bincode::deserialize(&payload).expect("decode should succeed");
        assert_eq!(decoded, input);
    }

    #[test]
    fn fold_embed_unfold_vault_policy_parity() {
        let input = default_vault_policy_sample();
        let (blob, hash) = fold_blob(&input, VAULT_POLICY_BLOB_ID).expect("fold should succeed");
        let manifest = generate_manifest(&[(VAULT_POLICY_BLOB_ID, blob.as_slice())]);
        let payload = unfold_blob_from_parts(VAULT_POLICY_BLOB_ID, &hash, &blob, &manifest)
            .expect("unfold should succeed");
        let decoded: EmbeddedVaultPolicy =
            bincode::deserialize(&payload).expect("decode should succeed");
        assert_eq!(decoded, input);
    }

    #[test]
    fn fold_embed_unfold_observability_profile_parity() {
        let input = default_observability_profile_sample();
        let (blob, hash) =
            fold_blob(&input, OBSERVABILITY_PROFILE_BLOB_ID).expect("fold should succeed");
        let manifest = generate_manifest(&[(OBSERVABILITY_PROFILE_BLOB_ID, blob.as_slice())]);
        let payload =
            unfold_blob_from_parts(OBSERVABILITY_PROFILE_BLOB_ID, &hash, &blob, &manifest)
                .expect("unfold should succeed");
        let decoded: EmbeddedObservabilityProfile =
            bincode::deserialize(&payload).expect("decode should succeed");
        assert_eq!(decoded, input);
    }

    #[test]
    fn embedded_blobs_load() {
        let heartbeat = load_embedded_heartbeat().expect("embedded heartbeat should load");
        assert!(!heartbeat.trim().is_empty());
        let replay =
            load_embedded_execution_replay().expect("embedded execution replay should load");
        assert_eq!(replay.workflow_id, "execution_replay_canary");
        assert!(replay.steps.len() >= 3);
        let vault_policy = load_embedded_vault_policy().expect("embedded vault policy should load");
        assert_eq!(vault_policy.policy_id, "vault_policy_primary");
        assert!(!vault_policy.rules.is_empty());
        let observability = load_embedded_observability_profile()
            .expect("embedded observability profile should load");
        assert_eq!(observability.profile_id, "observability_profile_primary");
        assert!(!observability.chaos_hooks.is_empty());
    }
}
