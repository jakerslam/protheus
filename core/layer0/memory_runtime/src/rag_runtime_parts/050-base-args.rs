
#[cfg(test)]
mod tests {
    use super::{
        byterover_upgrade_payload, chat_payload, ensure_supported_version, ingest_payload,
        memory_benchmark_ama_payload, memory_causal_retrieve_payload,
        memory_causality_enable_payload, memory_evolve_payload, memory_fuse_payload,
        memory_metacognitive_enable_payload, memory_share_payload, memory_taxonomy_payload,
        merge_vault_payload, nano_chat_payload, nano_fork_payload, nano_train_payload,
        search_payload, stable_status_payload, status_payload,
    };
    use std::collections::HashMap;
    use std::fs;

    fn base_args(root: &str) -> HashMap<String, String> {
        let mut args = HashMap::new();
        args.insert("root".to_string(), root.to_string());
        args.insert("path".to_string(), "docs".to_string());
        args
    }

    #[test]
    fn ingest_search_chat_merge_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let docs = dir.path().join("docs");
        fs::create_dir_all(&docs).expect("mkdir docs");
        fs::create_dir_all(dir.path().join("client/memory")).expect("mkdir memory");
        fs::write(
            docs.join("alpha.md"),
            "# Alpha\nThis document describes local rag indexing and memory retrieval.\n",
        )
        .expect("write alpha");
        fs::write(
            docs.join("beta.txt"),
            "The second document mentions retrieval confidence and citations.",
        )
        .expect("write beta");

        let args = base_args(&dir.path().to_string_lossy());
        let ingest = ingest_payload(&args);
        assert!(ingest.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(
            ingest
                .get("chunk_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= 2
        );

        let mut search_args = args.clone();
        search_args.insert("q".to_string(), "retrieval citations".to_string());
        let search = search_payload(&search_args);
        assert!(search.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(
            search
                .get("hit_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= 1
        );

        let chat = chat_payload(&search_args);
        assert!(chat.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(chat
            .get("answer")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("Document-grounded answer"));

        let merge = merge_vault_payload(&args);
        assert!(merge.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(
            merge
                .get("rows_added")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= 1
        );

        let status = status_payload(&args);
        assert!(status.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
    }

    #[test]
    fn incremental_reuses_unchanged_chunks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let docs = dir.path().join("docs");
        fs::create_dir_all(&docs).expect("mkdir docs");
        fs::write(
            docs.join("stable.md"),
            "Stable file for incremental ingest reuse behavior.",
        )
        .expect("write stable");
        let mut args = base_args(&dir.path().to_string_lossy());
        args.insert("incremental".to_string(), "true".to_string());
        let first = ingest_payload(&args);
        assert!(first.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        let second = ingest_payload(&args);
        assert!(second.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(
            second
                .get("reused_chunks")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= 1
        );
    }

    #[test]
    fn stable_api_version_gate_accepts_and_rejects_expected_values() {
        let mut ok_args = HashMap::new();
        ok_args.insert("api-version".to_string(), "1".to_string());
        assert_eq!(
            ensure_supported_version(&ok_args).expect("v1"),
            "v1".to_string()
        );

        let mut bad_args = HashMap::new();
        bad_args.insert("api-version".to_string(), "v9".to_string());
        let err = ensure_supported_version(&bad_args).expect_err("must reject");
        assert_eq!(
            err.get("error").and_then(|v| v.as_str()),
            Some("unsupported_api_version")
        );
    }

    #[test]
    fn stable_api_version_gate_defaults_and_canonicalizes_case() {
        let default_args: HashMap<String, String> = HashMap::new();
        assert_eq!(
            ensure_supported_version(&default_args).expect("defaults to stable"),
            "stable".to_string()
        );

        let mut stable_args = HashMap::new();
        stable_args.insert("api-version".to_string(), "STABLE".to_string());
        assert_eq!(
            ensure_supported_version(&stable_args).expect("stable case canonicalized"),
            "stable".to_string()
        );
    }

    #[test]
    fn stable_api_version_gate_reports_normalized_error_shape() {
        let mut bad_args = HashMap::new();
        bad_args.insert("api-version".to_string(), "V2".to_string());
        let err = ensure_supported_version(&bad_args).expect_err("must reject");
        assert_eq!(
            err.get("type").and_then(|v| v.as_str()),
            Some("memory_stable_api_error")
        );
        assert_eq!(
            err.get("requested_version").and_then(|v| v.as_str()),
            Some("v2")
        );
        assert_eq!(
            err.get("supported_versions")
                .and_then(|v| v.as_array())
                .map(|v| v.len()),
            Some(3)
        );
    }

    #[test]
    fn stable_status_reports_expected_commands() {
        let out = stable_status_payload();
        assert!(out.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        let commands = out
            .get("commands")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(commands
            .iter()
            .any(|v| v.as_str() == Some("stable-rag-search")));
    }

    #[test]
    fn byterover_upgrade_materializes_context_tree() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut args = HashMap::new();
        args.insert("root".to_string(), dir.path().to_string_lossy().to_string());
        let out = byterover_upgrade_payload(&args);
        assert!(out.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(dir.path().join(".brv/context-tree/timeline.md").exists());
        assert!(dir.path().join(".brv/context-tree/manifest.json").exists());
    }

    #[test]
    fn taxonomy_causality_and_ama_benchmark_workflow() {
        let dir = tempfile::tempdir().expect("tempdir");
        let docs = dir.path().join("docs");
        fs::create_dir_all(&docs).expect("mkdir docs");
        fs::write(
            docs.join("2026-03-12-ops.md"),
            "Policy rule updates with deterministic receipts and causal links.",
        )
        .expect("write doc");

        let mut args = base_args(&dir.path().to_string_lossy());
        let ingest = ingest_payload(&args);
        assert!(ingest.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));

        args.insert("q".to_string(), "policy causal receipts".to_string());
        let search = search_payload(&args);
        assert!(search.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        let chat = chat_payload(&args);
        assert!(chat.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));

        let meta = memory_metacognitive_enable_payload(&base_args(&dir.path().to_string_lossy()));
        assert!(meta.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert_eq!(meta.get("enabled").and_then(|v| v.as_bool()), Some(true));
        assert!(meta
            .get("config_digest")
            .and_then(|v| v.as_str())
            .map(|v| !v.is_empty())
            .unwrap_or(false));

        let taxonomy = memory_taxonomy_payload(&base_args(&dir.path().to_string_lossy()));
        assert!(taxonomy
            .get("ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(false));
        assert!(
            taxonomy
                .get("row_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= 1
        );
        let digest_a = taxonomy
            .get("taxonomy_digest")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        assert!(!digest_a.is_empty());
        let taxonomy_repeat = memory_taxonomy_payload(&base_args(&dir.path().to_string_lossy()));
        let digest_b = taxonomy_repeat
            .get("taxonomy_digest")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        assert_eq!(digest_a, digest_b);

        let causal = memory_causality_enable_payload(&base_args(&dir.path().to_string_lossy()));
        assert!(causal.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(
            causal
                .get("node_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= 3
        );

        let ama = memory_benchmark_ama_payload(&base_args(&dir.path().to_string_lossy()));
        assert!(ama.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(ama.get("ama_score").is_some());
    }

    #[test]
    fn nanochat_modes_emit_receipts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut args = HashMap::new();
        args.insert("root".to_string(), dir.path().to_string_lossy().to_string());
        args.insert("q".to_string(), "teach me nanochat".to_string());
        args.insert("depth".to_string(), "12".to_string());

        let chat = nano_chat_payload(&args);
        assert!(chat.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert_eq!(
            chat.get("type").and_then(|v| v.as_str()),
            Some("nano_chat_mode")
        );

        let train = nano_train_payload(&args);
        assert!(train.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert_eq!(
            train.get("type").and_then(|v| v.as_str()),
            Some("nano_train_mode")
        );

        let fork = nano_fork_payload(&args);
        assert!(fork.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert_eq!(
            fork.get("type").and_then(|v| v.as_str()),
            Some("nano_fork_mode")
        );
    }

    #[test]
    fn memory_share_evolve_retrieve_and_fuse_emit_receipts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let docs = dir.path().join("docs");
        fs::create_dir_all(&docs).expect("mkdir docs");
        fs::write(
            docs.join("flow.md"),
            "Step one causes step two; step two updates strategy.",
        )
        .expect("write");
        let mut args = base_args(&dir.path().to_string_lossy());
        let _ = ingest_payload(&args);
        args.insert("q".to_string(), "strategy".to_string());
        let _ = search_payload(&args);
        let _ = chat_payload(&args);
        let _ = memory_causality_enable_payload(&base_args(&dir.path().to_string_lossy()));
        let mut share_args = base_args(&dir.path().to_string_lossy());
        share_args.insert("persona".to_string(), "peer-shadow".to_string());
        share_args.insert("scope".to_string(), "task".to_string());
        share_args.insert("consent".to_string(), "true".to_string());
        let share = memory_share_payload(&share_args);
        assert!(share.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(share
            .get("consent_scope_digest")
            .and_then(|v| v.as_str())
            .map(|v| !v.is_empty())
            .unwrap_or(false));

        let evolve = memory_evolve_payload(&base_args(&dir.path().to_string_lossy()));
        assert!(evolve.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(evolve.get("stability_score").is_some());
        assert!(evolve
            .get("evolution_digest")
            .and_then(|v| v.as_str())
            .map(|v| !v.is_empty())
            .unwrap_or(false));

        let mut retrieve_args = base_args(&dir.path().to_string_lossy());
        retrieve_args.insert("q".to_string(), "strategy".to_string());
        retrieve_args.insert("depth".to_string(), "2".to_string());
        let retrieve = memory_causal_retrieve_payload(&retrieve_args);
        assert!(retrieve
            .get("ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(false));
        assert!(
            retrieve
                .get("trace_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= 1
        );

        let fuse = memory_fuse_payload(&base_args(&dir.path().to_string_lossy()));
        assert!(fuse.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(fuse.get("fusion_score").is_some());
    }
}

