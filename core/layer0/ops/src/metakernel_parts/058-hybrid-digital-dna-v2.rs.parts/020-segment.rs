fn mark_protected_gene(state: &mut HybridDnaState, instance_id: &str, gene_index: usize) {
    let entry = state
        .protected_gene_indexes
        .entry(instance_id.to_string())
        .or_default();
    if !entry.contains(&gene_index) {
        entry.push(gene_index);
        entry.sort_unstable();
        entry.dedup();
    }
}

fn add_hybrid_commit(root: &Path, commit: &HybridCommitRecord) {
    if let Ok(value) = serde_json::to_value(commit) {
        append_jsonl(&hybrid_dna_commits_path(root), &value);
        write_json(&hybrid_dna_latest_commit_path(root), &value);
    }
}

fn normalize_hybrid_region(raw: Option<&String>) -> String {
    let token = normalize_token(
        raw.map(String::as_str)
            .unwrap_or(HYBRID_REGION_ROOT_IDENTITY),
        HYBRID_REGION_ROOT_IDENTITY,
        96,
    );
    match token.as_str() {
        "root" | "root-identity" | "root_identity" => HYBRID_REGION_ROOT_IDENTITY.to_string(),
        "constitutional" | "constitutional-safety-rules" | "constitutional_safety_rules" => {
            HYBRID_REGION_CONSTITUTIONAL_SAFETY_RULES.to_string()
        }
        "lineage" | "lineage-parent-anchor" | "lineage_parent_anchor" => {
            HYBRID_REGION_LINEAGE_PARENT_ANCHOR.to_string()
        }
        "high-stakes" | "high-stakes-receipt" | "high_stakes_receipt" => {
            HYBRID_REGION_HIGH_STAKES_RECEIPT.to_string()
        }
        _ => token,
    }
}

fn validate_worm_region(region: &str) -> bool {
    matches!(
        region,
        HYBRID_REGION_ROOT_IDENTITY
            | HYBRID_REGION_CONSTITUTIONAL_SAFETY_RULES
            | HYBRID_REGION_LINEAGE_PARENT_ANCHOR
            | HYBRID_REGION_HIGH_STAKES_RECEIPT
    )
}

fn run_dna_hybrid_status(root: &Path) -> Value {
    let state = load_hybrid_dna_state(root);
    let commits = read_hybrid_commit_rows(root);
    json!({
        "ok": true,
        "type": "hybrid_dna_status",
        "schema_version": state.schema_version,
        "commit_count": commits.len(),
        "latest_commit_hash": state.latest_commit_hash,
        "worm_region_count": state.worm_regions.len(),
        "protected_gene_count": state.protected_gene_indexes.values().map(Vec::len).sum::<usize>(),
        "latest_receipt": read_json(&hybrid_dna_latest_receipt_path(root))
    })
}

fn run_dna_hybrid_commit(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    boundary_raw: Option<&String>,
    gene_index_raw: Option<&String>,
    critical_raw: Option<&String>,
) -> Value {
    let Some(instance_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_commit",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_raw, "instance", 96);
    let boundary = normalize_hybrid_boundary(boundary_raw);
    if !is_valid_hybrid_boundary(&boundary) {
        return json!({
            "ok": false,
            "type": "hybrid_dna_commit",
            "error": "boundary_invalid",
            "boundary": boundary
        });
    }
    let critical = parse_bool(critical_raw, false);

    let dna_state = load_digital_dna_state(root);
    let Some(genome) = dna_state.genomes.get(&instance_id) else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_commit",
            "error": "instance_dna_not_found",
            "instance_dna_ref": instance_id
        });
    };

    let gene_index = parse_hybrid_index(gene_index_raw, 0, 1_000_000);
    let boundary_key = if boundary == HYBRID_COMMIT_GENE_REVISION {
        format!("gene:{gene_index}")
    } else {
        boundary.clone()
    };

    let gene_merkle = if boundary == HYBRID_COMMIT_GENE_REVISION {
        genome.genes.get(gene_index).map(gene_merkle_root)
    } else {
        None
    };
    if boundary == HYBRID_COMMIT_GENE_REVISION && gene_merkle.is_none() {
        return json!({
            "ok": false,
            "type": "hybrid_dna_commit",
            "error": "gene_index_out_of_range",
            "instance_dna_ref": instance_id,
            "gene_index": gene_index
        });
    }
    let genome_merkle = genome_merkle_root(genome);

    let payload = json!({
        "instance_dna_ref": instance_id,
        "boundary": boundary,
        "boundary_key": boundary_key,
        "gene_index": gene_index,
        "gene_merkle_root": gene_merkle,
        "genome_merkle_root": genome_merkle,
        "critical": critical
    });
    let mut hybrid_state = load_hybrid_dna_state(root);
    let commit = build_commit_record(
        &instance_id,
        &boundary,
        &boundary_key,
        hybrid_state.latest_commit_hash.clone(),
        &payload,
        gene_merkle.clone(),
        genome_merkle.clone(),
        critical,
    );
    add_hybrid_commit(root, &commit);
    hybrid_state.latest_commit_hash = Some(commit.commit_hash.clone());
    if critical && boundary == HYBRID_COMMIT_GENE_REVISION {
        mark_protected_gene(&mut hybrid_state, &instance_id, gene_index);
    }
    save_hybrid_dna_state(root, &hybrid_state);

    let receipt = write_hybrid_receipt(
        root,
        "hybrid_commit",
        &instance_id,
        true,
        &payload,
        Some(&commit.commit_hash),
    );
    json!({
        "ok": true,
        "type": "hybrid_dna_commit",
        "instance_dna_ref": instance_id,
        "commit": commit,
        "receipt": receipt,
        "strict": strict
    })
}

fn run_dna_hybrid_verify(root: &Path, strict: bool, instance_id_raw: Option<&String>) -> Value {
    let requested_instance = instance_id_raw.map(|v| normalize_token(v, "instance", 96));
    let mut errors = Vec::new();
    let mut critical_errors = Vec::new();
    let commits = read_hybrid_commit_rows(root);
    let dna_state = load_digital_dna_state(root);

    for (idx, commit) in commits.iter().enumerate() {
        if let Some(instance_id) = requested_instance.as_ref() {
            if &commit.instance_dna_ref != instance_id {
                continue;
            }
        }
        let previous = if idx == 0 { None } else { commits.get(idx - 1) };
        if !validate_commit_link(commit, previous) {
            errors.push(json!({
                "type": "commit_chain_break",
                "commit_id": commit.commit_id,
                "instance_dna_ref": commit.instance_dna_ref,
                "critical": commit.critical
            }));
            if commit.critical {
                critical_errors.push(HybridCriticalEvent::CriticalCommitChainBreak);
            }
        }
        if commit.boundary == HYBRID_COMMIT_GENE_REVISION {
            let gene_idx = commit
                .boundary_key
                .strip_prefix("gene:")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0);
            if let Some(genome) = dna_state.genomes.get(&commit.instance_dna_ref) {
                if let Some(gene) = genome.genes.get(gene_idx) {
                    let actual = gene_merkle_root(gene);
                    if commit.gene_merkle_root.as_deref() != Some(actual.as_str()) {
                        errors.push(json!({
                            "type": "critical_merkle_mismatch",
                            "commit_id": commit.commit_id,
                            "instance_dna_ref": commit.instance_dna_ref,
                            "gene_index": gene_idx,
                            "expected": commit.gene_merkle_root,
                            "actual": actual
                        }));
                        if commit.critical {
                            critical_errors
                                .push(HybridCriticalEvent::CriticalMerkleMismatchOnProtectedData);
                        }
                    }
                }
            }
        }
    }

    let instance_ref = requested_instance.as_deref().unwrap_or("hybrid-dna-global");
    let lock_triggered = if let Some(event) = critical_errors.first().cloned() {
        lock_on_hybrid_critical_event(root, strict, event, instance_ref, json!({"errors": errors}))
    } else {
        false
    };

    let ok = errors.is_empty();
    let receipt = write_hybrid_receipt(
        root,
        "hybrid_verify",
        instance_ref,
        ok,
        &json!({
            "strict": strict,
            "error_count": errors.len(),
            "errors": errors
        }),
        None,
    );
    json!({
        "ok": if strict { ok } else { true },
        "type": "hybrid_dna_verify",
        "strict": strict,
        "errors": receipt.pointer("/payload/errors").cloned().unwrap_or_else(|| json!([])),
        "error_count": receipt.pointer("/payload/error_count").cloned().unwrap_or_else(|| json!(0)),
        "judicial_lock": { "triggered": lock_triggered },
        "receipt": receipt
    })
}

fn run_dna_hybrid_repair_gene(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    gene_index_raw: Option<&String>,
) -> Value {
    let Some(instance_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_repair_gene",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_raw, "instance", 96);
    let gene_index = parse_hybrid_index(gene_index_raw, 0, 1_000_000);

    let mut dna_state = load_digital_dna_state(root);
    let Some(genome) = dna_state.genomes.get_mut(&instance_id) else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_repair_gene",
            "error": "instance_dna_not_found",
            "instance_dna_ref": instance_id
        });
    };
    let Some(gene) = genome.genes.get_mut(gene_index) else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_repair_gene",
            "error": "gene_index_out_of_range",
            "instance_dna_ref": instance_id,
            "gene_index": gene_index
        });
    };

    let mut repaired_letters = 0usize;
    let mut complement_matches = 0usize;
    for codon in &mut gene.codons {
        for letter in &mut codon.letters {
            let (repaired, complement_match) = repair_letter_with_complement_check(letter);
            if repaired {
                repaired_letters += 1;
            }
            if complement_match {
                complement_matches += 1;
            }
        }
    }
    let gene_valid = gene.codons.iter().all(Codon::is_valid);
    let mut hybrid_state = load_hybrid_dna_state(root);
    let protected = is_protected_gene(&hybrid_state, &instance_id, gene_index);
    let failure_key = protected_gene_key(&instance_id, gene_index);
    let mut lock_triggered = false;

    if !gene_valid && protected {
        let failures = hybrid_state
            .protected_repair_failures
            .entry(failure_key.clone())
            .or_insert(0);
        *failures = failures.saturating_add(1);
        if *failures >= HYBRID_PROTECTED_REPAIR_FAILURE_LOCK_THRESHOLD {
            lock_triggered = lock_on_hybrid_critical_event(
                root,
                strict,
                HybridCriticalEvent::RepeatedFailedRepairOnProtectedStructure,
                &instance_id,
                json!({
                    "gene_index": gene_index,
                    "failures": *failures
                }),
            );
        }
    } else if gene_valid {
        hybrid_state.protected_repair_failures.remove(&failure_key);
    }

    save_hybrid_dna_state(root, &hybrid_state);
    save_digital_dna_state(root, &dna_state);

    let payload = json!({
        "gene_index": gene_index,
        "repaired_letters": repaired_letters,
        "complement_matches": complement_matches,
        "gene_valid": gene_valid,
        "protected": protected
    });
    let receipt = write_hybrid_receipt(
        root,
        "repair_mutable_gene",
        &instance_id,
        gene_valid,
        &payload,
        None,
    );
    json!({
        "ok": if strict { gene_valid } else { true },
        "type": "hybrid_dna_repair_gene",
        "instance_dna_ref": instance_id,
        "payload": payload,
        "judicial_lock": { "triggered": lock_triggered },
        "receipt": receipt
    })
}
