
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let strict = parse_bool(parsed.flags.get("strict"), true);
    let manifest_path = clean(
        parsed
            .flags
            .get("manifest")
            .cloned()
            .unwrap_or_else(|| CELLBUNDLE_EXAMPLE_PATH.to_string()),
        512,
    );
    let syscall_flag = parsed.flags.get("syscall");
    let session_flag = parsed.flags.get("session");
    let allow_flag = parsed.flags.get("allow");
    let instance_dna_flag = parsed.flags.get("instance-dna");
    let instance_id_flag = parsed.flags.get("instance-id");
    let parent_signature_flag = parsed.flags.get("parent-signature");
    let schema_version_flag = parsed.flags.get("schema-version");
    let generation_flag = parsed.flags.get("generation");
    let seed_flag = parsed.flags.get("seed");
    let mutation_flag = parsed.flags.get("mutation");
    let action_flag = parsed.flags.get("action");
    let boundary_flag = parsed.flags.get("boundary");
    let gene_index_flag = parsed.flags.get("gene-index");
    let critical_flag = parsed.flags.get("critical");
    let region_flag = parsed.flags.get("region");
    let region_key_flag = parsed.flags.get("region-key");
    let value_flag = parsed.flags.get("value");
    let step_flag = parsed.flags.get("step");
    let step_cap_flag = parsed.flags.get("step-cap");
    let drift_flag = parsed.flags.get("drift");
    let drift_threshold_flag = parsed.flags.get("drift-threshold");

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        for line in METAKERNEL_USAGE_LINES {
            println!("{line}");
        }
        return 0;
    }

    let latest = latest_path(root);
    let history = history_path(root);
    if command == "status" {
        let mut out = json!({
            "ok": true,
            "type": "metakernel_status",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "latest": read_json(&latest)
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        print_receipt(&out);
        return 0;
    }

    let payload = match command.as_str() {
        "registry" => run_registry(root, strict),
        "manifest" => run_manifest(root, strict, &manifest_path),
        "worlds" => run_worlds(root, strict, &manifest_path),
        "capability-taxonomy" => run_capability_taxonomy(root, strict, &manifest_path),
        "budget-admission" => run_budget_admission(root, strict, &manifest_path),
        "epistemic-object" => run_epistemic_object(root, strict, &manifest_path),
        "effect-journal" => run_effect_journal(root, strict, &manifest_path),
        "substrate-registry" => run_substrate_registry(root, strict),
        "radix-guard" => run_radix_guard(root, strict),
        "quantum-broker" => run_quantum_broker(root, strict),
        "neural-consent" => run_neural_consent_kernel(root, strict),
        "attestation-graph" => run_attestation_graph(root, strict),
        "degradation-contracts" => run_degradation_contracts(root, strict),
        "execution-profiles" => run_execution_profiles(root, strict),
        "variant-profiles" => run_variant_profiles(root, strict),
        "mpu-compartments" => run_mpu_compartments(root, strict),
        "dna-status" | "digital-dna-status" => run_digital_dna_status(root),
        "dna-create" | "digital-dna-create" => run_digital_dna_create(
            root,
            strict,
            instance_id_flag,
            parent_signature_flag,
            schema_version_flag,
            generation_flag,
            seed_flag,
        ),
        "dna-mutate" | "digital-dna-mutate" => {
            run_digital_dna_mutate(root, strict, instance_id_flag, mutation_flag, seed_flag)
        }
        "dna-enforce-subservience" | "digital-dna-subservience" => run_digital_dna_subservience(
            root,
            strict,
            instance_id_flag,
            parent_signature_flag,
            action_flag,
        ),
        "dna-hybrid-status" => run_dna_hybrid_status(root),
        "dna-hybrid-commit" => run_dna_hybrid_commit(
            root,
            strict,
            instance_id_flag,
            boundary_flag,
            gene_index_flag,
            critical_flag,
        ),
        "dna-hybrid-verify" => run_dna_hybrid_verify(root, strict, instance_id_flag),
        "dna-hybrid-repair-gene" => {
            run_dna_hybrid_repair_gene(root, strict, instance_id_flag, gene_index_flag)
        }
        "dna-hybrid-worm-supersede" => run_dna_hybrid_worm_supersede(
            root,
            strict,
            instance_id_flag,
            region_flag,
            region_key_flag,
            value_flag,
        ),
        "dna-hybrid-worm-mutate" => run_dna_hybrid_worm_mutate_attempt(
            root,
            strict,
            instance_id_flag,
            region_flag,
            region_key_flag,
        ),
        "dna-hybrid-protected-lineage" => run_dna_hybrid_protected_lineage_check(
            root,
            strict,
            instance_id_flag,
            parent_signature_flag,
            action_flag,
        ),
        "microkernel-safety" => {
            let mut out = run_microkernel_safety(
                root,
                strict,
                syscall_flag,
                session_flag,
                allow_flag,
                instance_dna_flag,
                step_flag,
                step_cap_flag,
                drift_flag,
                drift_threshold_flag,
            );
            if let (Some(instance_dna), Some(_)) = (instance_dna_flag, parent_signature_flag) {
                let instance_id = normalize_token(instance_dna, "instance-dna-default", 96);
                let action = normalize_token(
                    syscall_flag.map(String::as_str).unwrap_or("invoke_agent"),
                    "invoke_agent",
                    96,
                );
                let subservience = evaluate_subservience(
                    root,
                    &instance_id,
                    parent_signature_flag,
                    &action,
                    strict,
                );
                let sub_ok = subservience
                    .get("ok")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if strict && !sub_ok {
                    out["ok"] = Value::Bool(false);
                }
                out["subservience"] = subservience;
            }
            out
        }
        "invariants" => run_invariants(root, strict),
        _ => {
            let mut out = json!({
                "ok": false,
                "type": "metakernel_error",
                "lane": "core/layer0/ops",
                "ts": now_iso(),
                "error": "unknown_command",
                "command": command
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            print_receipt(&out);
            return 1;
        }
    };

    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let mut out = json!({
        "ok": ok,
        "type": "metakernel_run",
        "lane": "core/layer0/ops",
        "ts": now_iso(),
        "command": command,
        "payload": payload
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    write_json(&latest, &out);
    append_jsonl(&history, &out);
    print_receipt(&out);
    if ok { 0 } else { 1 }
}
