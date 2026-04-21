
// TODO-NORMATIVE: Full source-bound subservience chain semantics and governance policy are DEFERRED-V1.
fn run_digital_dna_create(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    parent_signature_raw: Option<&String>,
    schema_version_raw: Option<&String>,
    generation_raw: Option<&String>,
    seed_raw: Option<&String>,
) -> Value {
    let mut state = load_digital_dna_state(root);
    let generated_id = build_generated_instance_id(&state);
    let instance_id = normalize_token(
        instance_id_raw.map(String::as_str).unwrap_or(&generated_id),
        &generated_id,
        96,
    );
    if state.genomes.contains_key(&instance_id) {
        return json!({
            "ok": false,
            "type": "digital_dna_create",
            "error": "instance_id_already_exists",
            "instance_dna_ref": instance_id
        });
    }
    let parent_signature = normalize_token(
        parent_signature_raw
            .map(String::as_str)
            .unwrap_or(DIGITAL_DNA_DEFAULT_PARENT_SIGNATURE),
        DIGITAL_DNA_DEFAULT_PARENT_SIGNATURE,
        128,
    );
    let schema_version = normalize_schema_version(schema_version_raw);
    let birth_generation = parse_u64_clamped(generation_raw, 0, 1_000_000_000);
    let seed = clean(
        seed_raw
            .map(String::as_str)
            .unwrap_or("digital_dna_seed")
            .to_string(),
        256,
    );

    let genome = InstanceDna {
        header: GenomeHeader {
            instance_id: instance_id.clone(),
            parent_signature,
            version: schema_version,
            birth_generation,
        },
        genes: vec![default_gene(&seed)],
    };
    if let Err(err) = validate_instance_dna(&genome) {
        let receipt = write_digital_dna_receipt(
            root,
            "create",
            &instance_id,
            false,
            &json!({"error": err, "strict": strict}),
        );
        return json!({
            "ok": false,
            "type": "digital_dna_create",
            "error": "genome_validation_failed",
            "detail": err,
            "instance_dna_ref": instance_id,
            "receipt": receipt
        });
    }

    state.genomes.insert(instance_id.clone(), genome.clone());
    save_digital_dna_state(root, &state);
    let receipt = write_digital_dna_receipt(
        root,
        "create",
        &instance_id,
        true,
        &json!({"strict": strict, "birth_generation": birth_generation}),
    );

    json!({
        "ok": true,
        "type": "digital_dna_create",
        "instance_dna_ref": instance_id,
        "genome": genome,
        "receipt": receipt
    })
}

fn run_digital_dna_mutate(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    mutation_raw: Option<&String>,
    seed_raw: Option<&String>,
) -> Value {
    let Some(instance_id_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "digital_dna_mutate",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_id_raw, "instance", 96);
    let mutation = normalize_token(
        mutation_raw.map(String::as_str).unwrap_or("repair"),
        "repair",
        64,
    );
    let mut state = load_digital_dna_state(root);
    let Some(genome) = state.genomes.get_mut(&instance_id) else {
        return json!({
            "ok": false,
            "type": "digital_dna_mutate",
            "error": "instance_id_not_found",
            "instance_dna_ref": instance_id
        });
    };

    let mut mutation_report = json!({
        "mutation": mutation,
        "strict": strict
    });

    match mutation.as_str() {
        "repair" => {
            let (repaired_letters, complement_matches) = repair_instance_dna(genome);
            mutation_report["repaired_letters"] = json!(repaired_letters);
            mutation_report["complement_matches"] = json!(complement_matches);
        }
        "append-codon" => {
            let seed = clean(
                seed_raw
                    .map(String::as_str)
                    .unwrap_or("digital_dna_mutation_seed")
                    .to_string(),
                256,
            );
            if genome.genes.is_empty() {
                genome.genes.push(default_gene(&seed));
            } else if let Some(first_gene) = genome.genes.first_mut() {
                let index = first_gene.codons.len();
                first_gene.codons.push(seeded_codon(&seed, index));
            }
            mutation_report["codon_appended"] = json!(true);
        }
        "bump-generation" => {
            genome.header.birth_generation = genome.header.birth_generation.saturating_add(1);
            mutation_report["birth_generation"] = json!(genome.header.birth_generation);
        }
        _ => {
            return json!({
                "ok": false,
                "type": "digital_dna_mutate",
                "error": "unknown_mutation",
                "mutation": mutation,
                "instance_dna_ref": instance_id
            });
        }
    }

    if let Err(err) = validate_instance_dna(genome) {
        let receipt = write_digital_dna_receipt(
            root,
            "mutate",
            &instance_id,
            false,
            &json!({"error": err, "mutation": mutation}),
        );
        return json!({
            "ok": false,
            "type": "digital_dna_mutate",
            "error": "genome_validation_failed",
            "detail": err,
            "instance_dna_ref": instance_id,
            "receipt": receipt
        });
    }

    let genome_snapshot = genome.clone();
    save_digital_dna_state(root, &state);
    let receipt = write_digital_dna_receipt(root, "mutate", &instance_id, true, &mutation_report);
    json!({
        "ok": true,
        "type": "digital_dna_mutate",
        "instance_dna_ref": instance_id,
        "mutation": mutation,
        "genome": genome_snapshot,
        "receipt": receipt
    })
}

fn run_digital_dna_subservience(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    expected_parent_signature_raw: Option<&String>,
    action_raw: Option<&String>,
) -> Value {
    let Some(instance_id_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "digital_dna_subservience",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_id_raw, "instance", 96);
    let action = normalize_token(
        action_raw.map(String::as_str).unwrap_or("invoke_agent"),
        "invoke_agent",
        96,
    );
    let check = evaluate_subservience(
        root,
        &instance_id,
        expected_parent_signature_raw,
        &action,
        strict,
    );
    let ok = check.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let receipt = write_digital_dna_receipt(
        root,
        "subservience_check",
        &instance_id,
        ok,
        &json!({"action": action, "check": check.clone()}),
    );
    json!({
        "ok": if strict { ok } else { true },
        "type": "digital_dna_subservience",
        "instance_dna_ref": instance_id,
        "action": action,
        "check": check,
        "receipt": receipt,
        "judicial_lock": {
            "triggered": !ok && strict
        }
    })
}
