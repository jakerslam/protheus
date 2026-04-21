fn run_invariants(root: &Path, strict: bool) -> Value {
    let registry = run_registry(root, true);
    let manifest = run_manifest(root, true, CELLBUNDLE_EXAMPLE_PATH);
    let checks = vec![
        json!({
            "id": "MK_INV_001_registry_contract_exists",
            "ok": root.join(REGISTRY_PATH).exists()
        }),
        json!({
            "id": "MK_INV_002_registry_contract_valid",
            "ok": registry.get("registry_ok").and_then(Value::as_bool).unwrap_or(false)
        }),
        json!({
            "id": "MK_INV_003_no_unknown_primitive_usage",
            "ok": registry.get("unknown_primitive_usage_count").and_then(Value::as_u64).unwrap_or(1) == 0
        }),
        json!({
            "id": "MK_INV_004_cellbundle_schema_exists",
            "ok": root.join(CELLBUNDLE_SCHEMA_PATH).exists()
        }),
        json!({
            "id": "MK_INV_005_cellbundle_example_validates",
            "ok": manifest.get("manifest_ok").and_then(Value::as_bool).unwrap_or(false)
        }),
        json!({
            "id": "MK_INV_006_conduit_schema_present",
            "ok": root.join(CONDUIT_SCHEMA_PATH).exists()
        }),
        json!({
            "id": "MK_INV_007_three_plane_tla_present",
            "ok": root.join(TLA_BOUNDARY_PATH).exists()
        }),
        json!({
            "id": "MK_INV_008_core_policy_manifests_present",
            "ok": root.join(DEP_BOUNDARY_MANIFEST).exists() && root.join(RUST_SOURCE_OF_TRUTH_POLICY).exists()
        }),
    ];
    let pass = checks
        .iter()
        .all(|v| v.get("ok").and_then(Value::as_bool).unwrap_or(false));
    json!({
        "ok": if strict { pass } else { true },
        "strict": strict,
        "checks": checks,
        "registry": registry,
        "manifest": manifest
    })
}

const METAKERNEL_USAGE_LINES: &[&str] = &[
    "Usage:",
    "  protheus-ops metakernel status",
    "  protheus-ops metakernel registry [--strict=1|0]",
    "  protheus-ops metakernel manifest [--manifest=<path>] [--strict=1|0]",
    "  protheus-ops metakernel worlds [--manifest=<path>] [--strict=1|0]",
    "  protheus-ops metakernel capability-taxonomy [--manifest=<path>] [--strict=1|0]",
    "  protheus-ops metakernel budget-admission [--manifest=<path>] [--strict=1|0]",
    "  protheus-ops metakernel epistemic-object [--manifest=<path>] [--strict=1|0]",
    "  protheus-ops metakernel effect-journal [--manifest=<path>] [--strict=1|0]",
    "  protheus-ops metakernel substrate-registry [--strict=1|0]",
    "  protheus-ops metakernel radix-guard [--strict=1|0]",
    "  protheus-ops metakernel quantum-broker [--strict=1|0]",
    "  protheus-ops metakernel neural-consent [--strict=1|0]",
    "  protheus-ops metakernel attestation-graph [--strict=1|0]",
    "  protheus-ops metakernel degradation-contracts [--strict=1|0]",
    "  protheus-ops metakernel execution-profiles [--strict=1|0]",
    "  protheus-ops metakernel variant-profiles [--strict=1|0]",
    "  protheus-ops metakernel mpu-compartments [--strict=1|0]",
    "  protheus-ops metakernel dna-status [--strict=1|0]",
    "  protheus-ops metakernel dna-create [--instance-id=<id>] [--parent-signature=<sig>] [--schema-version=<v>] [--generation=<n>] [--seed=<text>] [--strict=1|0]",
    "  protheus-ops metakernel dna-mutate --instance-id=<id> [--mutation=repair|append-codon|bump-generation] [--seed=<text>] [--strict=1|0]",
    "  protheus-ops metakernel dna-enforce-subservience --instance-id=<id> --parent-signature=<sig> [--action=invoke_agent|fork_instance] [--strict=1|0]",
    "  protheus-ops metakernel dna-hybrid-status [--strict=1|0]",
    "  protheus-ops metakernel dna-hybrid-commit --instance-id=<id> [--boundary=gene_revision_commit|genome_revision_commit|critical_receipt_commit|worm_supersession_commit] [--gene-index=<n>] [--critical=1|0] [--strict=1|0]",
    "  protheus-ops metakernel dna-hybrid-verify [--instance-id=<id>] [--strict=1|0]",
    "  protheus-ops metakernel dna-hybrid-repair-gene --instance-id=<id> [--gene-index=<n>] [--strict=1|0]",
    "  protheus-ops metakernel dna-hybrid-worm-supersede --instance-id=<id> --region=<root_identity|constitutional_safety_rules|lineage_parent_anchor|high_stakes_receipt> [--region-key=<id>] --value=<text> [--strict=1|0]",
    "  protheus-ops metakernel dna-hybrid-worm-mutate --instance-id=<id> --region=<...> [--region-key=<id>] [--strict=1|0]",
    "  protheus-ops metakernel dna-hybrid-protected-lineage --instance-id=<id> --parent-signature=<sig> [--action=invoke_agent|fork_instance] [--strict=1|0]",
    "  protheus-ops metakernel microkernel-safety [--syscall=<id>] [--allow=<csv>] [--session=<id>] [--instance-dna=<id>] [--parent-signature=<sig>] [--step=<n>] [--step-cap=<n>] [--drift=<0..1>] [--drift-threshold=<0..1>] [--strict=1|0]",
    "  protheus-ops metakernel invariants [--strict=1|0]",
];
