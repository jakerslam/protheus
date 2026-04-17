
#[cfg(test)]
mod tests {
    use super::{
        decrypt_hot_state_envelope, encrypt_value, wrap_hot_state_envelope, DbIndexEntry,
        MemoryDb,
    };
    use tempfile::tempdir;

    fn test_key() -> [u8; 32] {
        [7u8; 32]
    }

    #[test]
    fn aead_round_trip() {
        let key = test_key();
        let payload = r#"{"ok":true,"k":"v"}"#;
        let wrapped = wrap_hot_state_envelope(&key, payload).expect("envelope");
        assert!(wrapped.contains("\"schema_id\":\"organ_state_envelope\""));
        let decrypted = decrypt_hot_state_envelope(&key, &wrapped).expect("decrypt");
        assert_eq!(decrypted, payload);
    }

    #[test]
    fn aead_tamper_fails_closed() {
        let key = test_key();
        let payload = r#"{"ok":true}"#;
        let encrypted = encrypt_value(&key, payload).expect("encrypt");
        let mut chars = encrypted.chars().collect::<Vec<char>>();
        let idx = chars.len().saturating_sub(1);
        chars[idx] = if chars[idx] == 'a' { 'b' } else { 'a' };
        let tampered_cipher = chars.into_iter().collect::<String>();
        let wrapped = serde_json::json!({
            "schema_id": "organ_state_envelope",
            "schema_version": "1.0",
            "organ": "memory",
            "lane": "hot_state",
            "ciphertext": tampered_cipher
        })
        .to_string();
        let decrypted = decrypt_hot_state_envelope(&key, &wrapped);
        assert!(decrypted.is_err(), "tampered payload should fail decrypt");
    }

    #[test]
    fn legacy_cipher_is_rejected_after_retirement() {
        let key = test_key();
        let legacy = "enc-v1:0000000000000001:00";
        let decrypted = decrypt_hot_state_envelope(&key, legacy);
        assert!(decrypted.is_err(), "legacy payload should be rejected");
    }

    #[test]
    fn fragmentation_stats_track_memory_kinds_explicitly() {
        let tmp = tempdir().expect("tempdir");
        let db = MemoryDb::open(tmp.path(), "").expect("open db");
        let stats0 = db.fragmentation_stats().expect("baseline stats");
        assert_eq!(stats0.episodic_rows, 0);
        assert_eq!(stats0.semantic_rows, 0);
        assert_eq!(stats0.procedural_rows, 0);
        drop(db);

        let mut db = MemoryDb::open(tmp.path(), "").expect("reopen db");
        db.replace_index_entries(
            &[
                DbIndexEntry {
                    node_id: "node.episode".to_string(),
                    uid: "UIDEPISODE".to_string(),
                    file_rel: "client/memory/2026-04-13.md".to_string(),
                    summary: "episodic memory".to_string(),
                    tags: vec!["memory".to_string()],
                    kind: "episodic".to_string(),
                },
                DbIndexEntry {
                    node_id: "node.semantic".to_string(),
                    uid: "UIDSEMANTIC".to_string(),
                    file_rel: "client/memory/2026-04-13.md".to_string(),
                    summary: "semantic memory".to_string(),
                    tags: vec!["fact".to_string()],
                    kind: "semantic".to_string(),
                },
                DbIndexEntry {
                    node_id: "node.procedural".to_string(),
                    uid: "UIDPROCEDURAL".to_string(),
                    file_rel: "client/memory/2026-04-13.md".to_string(),
                    summary: "procedural memory".to_string(),
                    tags: vec!["procedure".to_string()],
                    kind: "procedural".to_string(),
                },
            ],
            "test_kind_split",
        )
        .expect("write rows");
        let stats = db.fragmentation_stats().expect("kind stats");
        assert_eq!(stats.episodic_rows, 1);
        assert_eq!(stats.semantic_rows, 1);
        assert_eq!(stats.procedural_rows, 1);
    }
}
