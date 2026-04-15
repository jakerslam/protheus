#[cfg(test)]
mod hybrid_dna_v2_tests {
    use super::*;

    fn create_sample_instance(root: &Path, instance_id: &str, parent: &str) {
        let generation = "0".to_string();
        let schema = "v1".to_string();
        let seed = "hybrid-seed".to_string();
        let _ = run_digital_dna_create(
            root,
            true,
            Some(&instance_id.to_string()),
            Some(&parent.to_string()),
            Some(&schema),
            Some(&generation),
            Some(&seed),
        );
    }

    #[test]
    fn hybrid_gene_merkle_root_vector_exists() {
        let gene = default_gene("vector-seed");
        let root = gene_merkle_root(&gene);
        assert!(!root.is_empty());
    }

    #[test]
    fn hybrid_valid_commit_chain_example() {
        let root = tempfile::tempdir().expect("tempdir");
        create_sample_instance(root.path(), "instance-hybrid-1", "parent-a");
        let first = run_dna_hybrid_commit(
            root.path(),
            true,
            Some(&"instance-hybrid-1".to_string()),
            Some(&"gene_revision_commit".to_string()),
            Some(&"0".to_string()),
            Some(&"1".to_string()),
        );
        assert_eq!(first.get("ok").and_then(Value::as_bool), Some(true));
        let second = run_dna_hybrid_commit(
            root.path(),
            true,
            Some(&"instance-hybrid-1".to_string()),
            Some(&"genome_revision_commit".to_string()),
            None,
            Some(&"0".to_string()),
        );
        assert_eq!(second.get("ok").and_then(Value::as_bool), Some(true));
        let verify =
            run_dna_hybrid_verify(root.path(), true, Some(&"instance-hybrid-1".to_string()));
        assert_eq!(verify.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn hybrid_invalid_commit_chain_example() {
        let root = tempfile::tempdir().expect("tempdir");
        create_sample_instance(root.path(), "instance-hybrid-2", "parent-a");
        let _ = run_dna_hybrid_commit(
            root.path(),
            true,
            Some(&"instance-hybrid-2".to_string()),
            Some(&"gene_revision_commit".to_string()),
            Some(&"0".to_string()),
            Some(&"1".to_string()),
        );
        let mut rows = read_hybrid_commit_rows(root.path());
        assert_eq!(rows.len(), 1);
        rows[0].previous_hash = Some("broken-link".to_string());
        let text = rows
            .iter()
            .map(|row| serde_json::to_string(row).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(hybrid_dna_commits_path(root.path()), format!("{text}\n"))
            .expect("write tampered");
        let verify =
            run_dna_hybrid_verify(root.path(), true, Some(&"instance-hybrid-2".to_string()));
        assert_eq!(verify.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            verify
                .pointer("/judicial_lock/triggered")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn hybrid_mutable_region_repair_example() {
        let root = tempfile::tempdir().expect("tempdir");
        create_sample_instance(root.path(), "instance-hybrid-3", "parent-a");
        let mut dna_state = load_digital_dna_state(root.path());
        let genome = dna_state
            .genomes
            .get_mut("instance-hybrid-3")
            .expect("genome exists");
        genome.genes[0].codons[0].letters[0].verity =
            genome.genes[0].codons[0].letters[0].verity.complement();
        save_digital_dna_state(root.path(), &dna_state);

        let repair = run_dna_hybrid_repair_gene(
            root.path(),
            true,
            Some(&"instance-hybrid-3".to_string()),
            Some(&"0".to_string()),
        );
        assert_eq!(repair.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            repair
                .pointer("/payload/repaired_letters")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0,
            true
        );
    }

    #[test]
    fn hybrid_worm_supersession_example() {
        let root = tempfile::tempdir().expect("tempdir");
        create_sample_instance(root.path(), "instance-hybrid-4", "parent-a");
        let one = run_dna_hybrid_worm_supersede(
            root.path(),
            true,
            Some(&"instance-hybrid-4".to_string()),
            Some(&"lineage_parent_anchor".to_string()),
            Some(&"anchor-1".to_string()),
            Some(&"value-v1".to_string()),
        );
        assert_eq!(one.get("ok").and_then(Value::as_bool), Some(true));
        let two = run_dna_hybrid_worm_supersede(
            root.path(),
            true,
            Some(&"instance-hybrid-4".to_string()),
            Some(&"lineage_parent_anchor".to_string()),
            Some(&"anchor-1".to_string()),
            Some(&"value-v2".to_string()),
        );
        assert_eq!(two.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(two.get("version").and_then(Value::as_u64), Some(2));
    }

    #[test]
    fn hybrid_judicial_lock_invalid_worm_mutation_example() {
        let root = tempfile::tempdir().expect("tempdir");
        create_sample_instance(root.path(), "instance-hybrid-5", "parent-a");
        let out = run_dna_hybrid_worm_mutate_attempt(
            root.path(),
            true,
            Some(&"instance-hybrid-5".to_string()),
            Some(&"root_identity".to_string()),
            Some(&"identity-anchor".to_string()),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.pointer("/judicial_lock/triggered")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn hybrid_protected_lineage_example() {
        let root = tempfile::tempdir().expect("tempdir");
        create_sample_instance(root.path(), "instance-hybrid-6", "parent-a");
        let out = run_dna_hybrid_protected_lineage_check(
            root.path(),
            true,
            Some(&"instance-hybrid-6".to_string()),
            Some(&"parent-b".to_string()),
            Some(&"invoke_agent".to_string()),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.pointer("/judicial_lock/triggered")
                .and_then(Value::as_bool),
            Some(true)
        );
    }
}
