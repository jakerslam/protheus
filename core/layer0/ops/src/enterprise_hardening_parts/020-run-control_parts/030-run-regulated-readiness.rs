
fn run_regulated_readiness(
    root: &Path,
    strict: bool,
    flags: &std::collections::HashMap<String, String>,
) -> Result<Value, String> {
    let multi_tenant_path = flags
        .get("multi-tenant-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_MULTI_TENANT_CONTRACT_REL);
    let access_policy_path = flags
        .get("access-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_ACCESS_POLICY_REL);
    let abac_policy_path = flags
        .get("abac-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_ABAC_POLICY_REL);
    let secret_kms_policy_path = flags
        .get("secret-kms-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_SECRET_KMS_POLICY_REL);
    let signed_receipt_policy_path = flags
        .get("signed-receipt-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_SIGNED_RECEIPT_POLICY_REL);
    let retention_pack_policy_path = flags
        .get("retention-pack-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_RETENTION_POLICY_PACK_REL);
    let runtime_retention_policy_path = flags
        .get("runtime-retention-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_RUNTIME_RETENTION_POLICY_REL);
    let compliance_retention_policy_path = flags
        .get("compliance-retention-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_COMPLIANCE_RETENTION_POLICY_REL);
    let audit_export_policy_path = flags
        .get("audit-export-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_AUDIT_EXPORT_POLICY_REL);
    let evidence_audit_policy_path = flags
        .get("evidence-audit-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_EVIDENCE_AUDIT_POLICY_REL);
    let deployment_doc_path = flags
        .get("deployment-doc")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_DEPLOYMENT_PACKAGING_DOC_REL);

    let multi_tenant = read_json(&root.join(multi_tenant_path))?;
    let access_policy = read_json(&root.join(access_policy_path))?;
    let abac_policy = read_json(&root.join(abac_policy_path))?;
    let secret_kms_policy = read_json(&root.join(secret_kms_policy_path))?;
    let signed_receipt_policy = read_json(&root.join(signed_receipt_policy_path))?;
    let retention_pack_policy = read_json(&root.join(retention_pack_policy_path))?;
    let runtime_retention_policy = read_json(&root.join(runtime_retention_policy_path))?;
    let compliance_retention_policy = read_json(&root.join(compliance_retention_policy_path))?;
    let audit_export_policy = read_json(&root.join(audit_export_policy_path))?;
    let evidence_audit_policy = read_json(&root.join(evidence_audit_policy_path))?;

    let zero_trust_profile_path = enterprise_state_root(root).join("f100/zero_trust_profile.json");
    let zero_trust_profile = read_json(&zero_trust_profile_path).ok();

    let multi_tenant_ok = multi_tenant
        .pointer("/invariants/cross_tenant_leaks")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        == 0
        && bool_at(&multi_tenant, "invariants.classification_enforced")
        && bool_at(&multi_tenant, "invariants.delete_export_contract")
        && str_at(&multi_tenant, "namespace.mode").eq_ignore_ascii_case("strict")
        && !str_at(&multi_tenant, "namespace.tenant_id_pattern").is_empty()
        && bool_at(&multi_tenant, "namespace.enforce_per_tenant_state_roots")
        && usize_array_len_at(&multi_tenant, "namespace.per_tenant_state_roots") > 0
        && str_at(&multi_tenant, "namespace.cross_namespace_reads")
            .eq_ignore_ascii_case("deny_by_default");

    let access_ops = access_policy
        .get("operations")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let rbac_ok = !access_ops.is_empty()
        && access_ops.values().all(|op| {
            op.get("tenant_scoped")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && op
                    .get("require_mfa")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        });

    let abac_rules = abac_policy
        .get("rules")
        .or_else(|| abac_policy.get("policies"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let abac_ok = str_at(&abac_policy, "default_effect").eq_ignore_ascii_case("deny")
        && !abac_rules.is_empty()
        && bool_at(&abac_policy, "flight_recorder.immutable")
        && bool_at(&abac_policy, "flight_recorder.hash_chain");

    let secret_kms_ok = str_at(&secret_kms_policy, "handle_mode")
        .eq_ignore_ascii_case("opaque_handle_only")
        && str_at(&secret_kms_policy, "cmek.key_uri").starts_with("kms://")
        && usize_array_len_at(&secret_kms_policy, "backends") > 0;

    let zero_trust_ok = zero_trust_profile
        .as_ref()
        .map(|profile| {
            cross_plane_guard_ok(
                bool_at(profile, "signed_jwt"),
                &str_at(profile, "cmek_key"),
                &str_at(profile, "private_link"),
                &str_at(profile, "egress"),
            )
        })
        .unwrap_or(false);

    let signed_algorithms = signed_receipt_policy
        .get("algorithms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let required_envs = signed_receipt_policy
        .get("required_env")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    let missing_envs = required_envs
        .iter()
        .filter(|name| !bool_env_set(name))
        .cloned()
        .collect::<Vec<_>>();
    let signed_receipts_ok = signed_algorithms.iter().any(|algo| algo == "hmac_sha256")
        && !required_envs.is_empty()
        && missing_envs.is_empty()
        && bool_at(
            &signed_receipt_policy,
            "receipt_chain.hmac_required_for_regulated_exports",
        );

    let retention_pack_ok = bool_at(&retention_pack_policy, "enabled")
        && !str_at(&retention_pack_policy, "runtime_policy_path").is_empty()
        && !str_at(&retention_pack_policy, "compliance_policy_path").is_empty()
        && usize_array_len_at(&runtime_retention_policy, "jsonl_targets") > 0
        && usize_array_len_at(&runtime_retention_policy, "directory_targets") > 0
        && compliance_retention_policy
            .pointer("/tiers/hot_days")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
        && compliance_retention_policy
            .pointer("/tiers/warm_days")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
        && compliance_retention_policy
            .pointer("/tiers/cold_days")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
        && usize_array_len_at(&compliance_retention_policy, "scopes") > 0;

    let audit_export_ok = !str_at(&audit_export_policy, "outputs.latest_path").is_empty()
        && !str_at(&audit_export_policy, "outputs.history_path").is_empty()
        && !str_at(&evidence_audit_policy, "paths.export_json_path").is_empty()
        && !str_at(&evidence_audit_policy, "paths.export_md_path").is_empty()
        && !str_at(&evidence_audit_policy, "paths.receipts_path").is_empty();

    let docs_required_tokens = vec![
        "Multi-Tenant Deployment".to_string(),
        "RBAC/ABAC".to_string(),
        "KMS-backed Secret Handling".to_string(),
        "Signed Receipts".to_string(),
        "Retention Policy Packs".to_string(),
        "Exportable Audit Trails".to_string(),
    ];
    let deployment_doc_abs = root.join(deployment_doc_path);
    let docs_missing_tokens = file_contains_all(&deployment_doc_abs, &docs_required_tokens)
        .unwrap_or_else(|_| docs_required_tokens.clone());
    let docs_ok = docs_missing_tokens.is_empty();

    let controls = vec![
        json!({
            "id": "multi_tenant_namespaces",
            "ok": multi_tenant_ok,
            "path": multi_tenant_path
        }),
        json!({
            "id": "rbac_policy",
            "ok": rbac_ok,
            "path": access_policy_path,
            "operations": access_ops.len()
        }),
        json!({
            "id": "abac_policy",
            "ok": abac_ok,
            "path": abac_policy_path,
            "rules": abac_rules.len()
        }),
        json!({
            "id": "kms_secret_policy",
            "ok": secret_kms_ok && zero_trust_ok,
            "path": secret_kms_policy_path,
            "zero_trust_profile_path": zero_trust_profile_path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| zero_trust_profile_path.to_string_lossy().to_string()),
            "zero_trust_profile_present": zero_trust_profile.is_some()
        }),
        json!({
            "id": "signed_receipts",
            "ok": signed_receipts_ok,
            "path": signed_receipt_policy_path,
            "missing_required_env": missing_envs
        }),
        json!({
            "id": "retention_policy_pack",
            "ok": retention_pack_ok,
            "path": retention_pack_policy_path,
            "runtime_policy_path": runtime_retention_policy_path,
            "compliance_policy_path": compliance_retention_policy_path
        }),
        json!({
            "id": "audit_export",
            "ok": audit_export_ok,
            "audit_export_policy_path": audit_export_policy_path,
            "evidence_audit_policy_path": evidence_audit_policy_path
        }),
        json!({
            "id": "deployment_docs_regulated_surface",
            "ok": docs_ok,
            "path": deployment_doc_path,
            "missing_tokens": docs_missing_tokens
        }),
    ];
    let controls_failed = controls
        .iter()
        .filter(|row| !row.get("ok").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let regulated_ok = controls_failed == 0;

    Ok(with_receipt_hash(json!({
        "ok": !strict || regulated_ok,
        "type": "enterprise_hardening_regulated_readiness",
        "lane": "enterprise_hardening",
        "mode": "regulated-readiness",
        "strict": strict,
        "ts": now_iso(),
        "controls_total": controls.len(),
        "controls_failed": controls_failed,
        "controls": controls,
        "claim_evidence": [
            {
                "id": "V11-ENTERPRISE-008",
                "claim": "enterprise_hardening_validates_multi_tenant_rbac_abac_kms_signed_receipts_retention_and_exportable_audit_contracts",
                "evidence": {
                    "controls_total": 8,
                    "controls_failed": controls_failed,
                    "deployment_doc": deployment_doc_path
                }
            }
        ]
    })))
}
