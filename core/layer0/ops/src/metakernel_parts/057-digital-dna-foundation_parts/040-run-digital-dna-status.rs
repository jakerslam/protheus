
fn run_digital_dna_status(root: &Path) -> Value {
    let state = load_digital_dna_state(root);
    let latest_receipt = read_json(&digital_dna_latest_receipt_path(root));
    // TODO-NORMATIVE: Qubit wrapper semantics remain DEFERRED-V1; this structure is a placeholder for source-bound superposition metadata.
    let qubit_wrapper_example = QubitWrapper::<Value> {
        layer: "quark".to_string(),
        superposed: vec![
            json!({"value": -1}),
            json!({"value": 0}),
            json!({"value": 1}),
        ],
        collapsed_index: None,
        semantics: "DEFERRED-V1".to_string(),
    };
    json!({
        "ok": true,
        "type": "digital_dna_status",
        "schema_version": state.schema_version,
        "genome_count": state.genomes.len(),
        "instance_ids": state.genomes.keys().cloned().collect::<Vec<_>>(),
        "latest_receipt": latest_receipt,
        "deferred": {
            "qubit_wrapper": qubit_wrapper_example,
            "subservience_full_chain_rules": "DEFERRED-V1"
        }
    })
}

#[cfg(test)]
mod digital_dna_tests {
    use super::*;

    fn sample_letter() -> Letter {
        Letter::new(
            Baryon::from_trits([1, 0, -1]).expect("valid baryon"),
            Baryon::from_trits([0, 1, 1]).expect("valid baryon"),
            Baryon::from_trits([-1, 0, 1]).expect("valid baryon"),
        )
    }

    #[test]
    fn letter_validation_rejects_invalid_verity() {
        let mut letter = sample_letter();
        assert!(letter.is_valid());
        letter.verity = letter.verity.complement();
        assert!(!letter.is_valid());
    }

    #[test]
    fn codon_new_rejects_invalid_letter() {
        let mut invalid = sample_letter();
        invalid.verity = invalid.verity.complement();
        let valid = sample_letter();
        let out = Codon::new([invalid, valid.clone(), valid.clone(), valid.clone()]);
        assert!(out.is_err());
    }

    #[test]
    fn genome_create_emits_receipt_with_instance_reference() {
        let root = tempfile::tempdir().expect("tempdir");
        let instance_id = "instance-alpha".to_string();
        let parent = "root-parent".to_string();
        let schema = "v1".to_string();
        let generation = "3".to_string();
        let seed = "seed-alpha".to_string();
        let out = run_digital_dna_create(
            root.path(),
            true,
            Some(&instance_id),
            Some(&parent),
            Some(&schema),
            Some(&generation),
            Some(&seed),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("instance_dna_ref").and_then(Value::as_str),
            Some("instance-alpha")
        );
        let latest =
            read_json(&digital_dna_latest_receipt_path(root.path())).expect("latest receipt");
        assert_eq!(
            latest.get("instance_dna_ref").and_then(Value::as_str),
            Some("instance-alpha")
        );
    }

    #[test]
    fn subservience_mismatch_triggers_judicial_lock() {
        let root = tempfile::tempdir().expect("tempdir");
        let instance_id = "instance-beta".to_string();
        let parent = "parent-a".to_string();
        let schema = "v1".to_string();
        let generation = "0".to_string();
        let seed = "seed-beta".to_string();
        let _ = run_digital_dna_create(
            root.path(),
            true,
            Some(&instance_id),
            Some(&parent),
            Some(&schema),
            Some(&generation),
            Some(&seed),
        );

        let wrong_parent = "parent-b".to_string();
        let action = "invoke_agent".to_string();
        let out = run_digital_dna_subservience(
            root.path(),
            true,
            Some(&instance_id),
            Some(&wrong_parent),
            Some(&action),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.pointer("/judicial_lock/triggered")
                .and_then(Value::as_bool),
            Some(true)
        );
    }
}
