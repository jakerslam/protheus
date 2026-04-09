fn run_control(root: &Path, control: &serde_json::Map<String, Value>) -> Value {
    let id = control
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let title = control
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("untitled")
        .to_string();
    let kind = control
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("path_exists")
        .to_string();
    let rel_path = control
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    if rel_path.trim().is_empty() {
        return json!({
            "id": id,
            "title": title,
            "ok": false,
            "reason": "missing_path"
        });
    }

    let path = root.join(&rel_path);
    match kind.as_str() {
        "path_exists" => {
            let ok = path.exists();
            json!({
                "id": id,
                "title": title,
                "type": kind,
                "ok": ok,
                "path": rel_path,
                "reason": if ok { Value::Null } else { Value::String("path_missing".to_string()) }
            })
        }
        "file_contains_all" => {
            let required_tokens = control
                .get("required_tokens")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if required_tokens.is_empty() {
                return json!({
                    "id": id,
                    "title": title,
                    "type": kind,
                    "ok": false,
                    "path": rel_path,
                    "reason": "required_tokens_missing"
                });
            }
            match file_contains_all(&path, &required_tokens) {
                Ok(missing) => json!({
                    "id": id,
                    "title": title,
                    "type": kind,
                    "ok": missing.is_empty(),
                    "path": rel_path,
                    "required_tokens": required_tokens.len(),
                    "missing_tokens": missing
                }),
                Err(err) => json!({
                    "id": id,
                    "title": title,
                    "type": kind,
                    "ok": false,
                    "path": rel_path,
                    "reason": err
                }),
            }
        }
        "json_fields" => {
            let required_fields = control
                .get("required_fields")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if required_fields.is_empty() {
                return json!({
                    "id": id,
                    "title": title,
                    "type": kind,
                    "ok": false,
                    "path": rel_path,
                    "reason": "required_fields_missing"
                });
            }
            match read_json(&path) {
                Ok(payload) => {
                    let missing_fields = required_fields
                        .iter()
                        .filter(|field| resolve_json_path(&payload, field).is_none())
                        .cloned()
                        .collect::<Vec<_>>();
                    json!({
                        "id": id,
                        "title": title,
                        "type": kind,
                        "ok": missing_fields.is_empty(),
                        "path": rel_path,
                        "required_fields": required_fields,
                        "missing_fields": missing_fields
                    })
                }
                Err(err) => json!({
                    "id": id,
                    "title": title,
                    "type": kind,
                    "ok": false,
                    "path": rel_path,
                    "reason": err
                }),
            }
        }
        "cron_delivery_integrity" => match check_cron_delivery_integrity(root, &rel_path) {
            Ok((ok, details)) => json!({
                "id": id,
                "title": title,
                "type": kind,
                "ok": ok,
                "path": rel_path,
                "details": details
            }),
            Err(err) => json!({
                "id": id,
                "title": title,
                "type": kind,
                "ok": false,
                "path": rel_path,
                "reason": err
            }),
        },
        _ => json!({
            "id": id,
            "title": title,
            "type": kind,
            "ok": false,
            "path": rel_path,
            "reason": format!("unknown_control_type:{kind}")
        }),
    }
}

fn run_with_policy(
    root: &Path,
    cmd: &str,
    strict: bool,
    policy_path_rel: &str,
) -> Result<Value, String> {
    let policy_path = root.join(policy_path_rel);
    let policy = read_json(&policy_path)?;
    let controls = policy
        .get("controls")
        .and_then(Value::as_array)
        .ok_or_else(|| "enterprise_policy_missing_controls".to_string())?;

    let mut results = Vec::<Value>::new();
    for control in controls {
        let Some(section) = control.as_object() else {
            results.push(json!({
                "id": "unknown",
                "ok": false,
                "reason": "invalid_control_entry"
            }));
            continue;
        };
        results.push(run_control(root, section));
    }

    let passed = results
        .iter()
        .filter(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let failed = results.len().saturating_sub(passed);
    let ok = if strict { failed == 0 } else { true };

    let mut out = json!({
        "ok": ok,
        "type": "enterprise_hardening",
        "lane": "enterprise_hardening",
        "mode": cmd,
        "strict": strict,
        "ts": now_iso(),
        "policy_path": policy_path_rel,
        "controls_total": results.len(),
        "controls_passed": passed,
        "controls_failed": failed,
        "controls": results,
        "claim_evidence": [
            {
                "id": "f100_controls_gate",
                "claim": "fortune_100_control_contract_is_enforced_before_release",
                "evidence": {
                    "controls_total": controls.len(),
                    "strict": strict,
                    "failed": failed
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    Ok(out)
}

fn run_export_compliance(
    root: &Path,
    strict: bool,
    policy_path_rel: &str,
    profile: &str,
) -> Result<Value, String> {
    let profile_clean = profile.trim().to_ascii_lowercase();
    if !matches!(profile_clean.as_str(), "internal" | "customer" | "auditor") {
        return Err("invalid_compliance_profile".to_string());
    }
    let hardening = run_with_policy(root, "run", strict, policy_path_rel)?;
    let controls = hardening
        .get("controls")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let evidence_manifest = controls
        .iter()
        .filter_map(|row| row.get("path").and_then(Value::as_str))
        .map(|path| manifest_entry(root, path))
        .collect::<Vec<_>>();
    let controls_failed = hardening
        .get("controls_failed")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let bundle_seed = json!({
        "profile": profile_clean,
        "controls_total": controls.len(),
        "controls_failed": controls_failed,
        "ts": now_iso()
    });
    let bundle_hash = deterministic_receipt_hash(&bundle_seed);
    let bundle_id = format!("enterprise_bundle_{}", &bundle_hash[..16]);
    let bundle_path = enterprise_state_root(root)
        .join("compliance_exports")
        .join(format!("{bundle_id}.json"));
    let bundle_rel = bundle_path
        .strip_prefix(root)
        .unwrap_or(&bundle_path)
        .to_string_lossy()
        .replace('\\', "/");
    let bundle = json!({
        "schema_id": "enterprise_compliance_bundle",
        "schema_version": "1.0",
        "bundle_id": bundle_id,
        "profile": profile_clean,
        "generated_at": now_iso(),
        "policy_path": policy_path_rel,
        "controls_total": controls.len(),
        "controls_failed": controls_failed,
        "hardening_receipt_hash": hardening.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "evidence_manifest": evidence_manifest
    });
    write_json(&bundle_path, &bundle)?;

    Ok(with_receipt_hash(json!({
        "ok": !strict || controls_failed == 0,
        "type": "enterprise_hardening_compliance_export",
        "lane": "enterprise_hardening",
        "mode": "export-compliance",
        "strict": strict,
        "profile": profile_clean,
        "bundle_path": bundle_rel,
        "controls_total": controls.len(),
        "controls_failed": controls_failed,
        "claim_evidence": [
            {
                "id": "V7-ENTERPRISE-001.1",
                "claim": "one_command_compliance_export_produces_traceable_audit_bundle_artifacts",
                "evidence": {
                    "bundle_path": bundle_rel,
                    "profile": profile_clean,
                    "manifest_entries": controls.len()
                }
            }
        ]
    })))
}

fn run_identity_surface(
    root: &Path,
    strict: bool,
    flags: &std::collections::HashMap<String, String>,
) -> Result<Value, String> {
    let identity_policy_path = flags
        .get("identity-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_IDENTITY_POLICY_REL);
    let access_policy_path = flags
        .get("access-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_ACCESS_POLICY_REL);
    let abac_policy_path = flags
        .get("abac-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_ABAC_POLICY_REL);
    let siem_policy_path = flags
        .get("siem-policy")
        .map(|v| v.as_str())
        .unwrap_or(DEFAULT_SIEM_POLICY_REL);

    let identity_policy = read_json(&root.join(identity_policy_path))?;
    let providers = identity_policy
        .get("providers")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let requested_provider = flags
        .get("provider")
        .map(|v| v.trim().to_ascii_lowercase())
        .or_else(|| providers.keys().next().map(|v| v.to_ascii_lowercase()))
        .unwrap_or_default();
    let provider = providers
        .get(&requested_provider)
        .cloned()
        .unwrap_or(Value::Null);
    let provider_obj = provider.as_object().cloned().unwrap_or_default();
    let issuer_prefix = provider_obj
        .get("issuer_prefix")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let token_issuer = flags
        .get("token-issuer")
        .cloned()
        .unwrap_or_else(|| format!("{issuer_prefix}enterprise"));
    let scopes = split_csv(
        flags
            .get("scopes")
            .map(|v| v.as_str())
            .unwrap_or("openid,profile,protheus.read"),
    );
    let roles = split_csv(flags.get("roles").map(|v| v.as_str()).unwrap_or("operator"));
    let allowed_scopes = provider_obj
        .get("allowed_scopes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.to_ascii_lowercase())
        .collect::<std::collections::BTreeSet<_>>();
    let allowed_roles = provider_obj
        .get("allowed_roles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.to_ascii_lowercase())
        .collect::<std::collections::BTreeSet<_>>();
    let scopes_allowed = scopes.iter().all(|scope| allowed_scopes.contains(scope));
    let roles_allowed = roles.iter().all(|role| allowed_roles.contains(role));
    let issuer_allowed = !issuer_prefix.is_empty() && token_issuer.starts_with(&issuer_prefix);
    let scim_enabled = provider_obj
        .get("scim_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let access_policy = read_json(&root.join(access_policy_path))?;
    let access_ops = access_policy
        .get("operations")
        .and_then(Value::as_object)
        .map(|ops| ops.len())
        .unwrap_or(0);
    let abac_policy = read_json(&root.join(abac_policy_path))?;
    let abac_rules = abac_policy
        .get("rules")
        .or_else(|| abac_policy.get("policies"))
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    let siem_policy = read_json(&root.join(siem_policy_path))?;
    let has_siem_export = siem_policy
        .get("latest_export_path")
        .and_then(Value::as_str)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    let identity_ok = !requested_provider.is_empty()
        && !providers.is_empty()
        && scopes_allowed
        && roles_allowed
        && issuer_allowed
        && access_ops > 0
        && abac_rules > 0
        && has_siem_export;

    Ok(with_receipt_hash(json!({
        "ok": !strict || identity_ok,
        "type": "enterprise_hardening_identity_surface",
        "lane": "enterprise_hardening",
        "mode": "identity-surface",
        "strict": strict,
        "provider": requested_provider,
        "token_issuer": token_issuer,
        "scopes": scopes,
        "roles": roles,
        "surface": {
            "providers": providers.keys().cloned().collect::<Vec<_>>(),
            "scim_enabled_for_provider": scim_enabled,
            "rbac_operations": access_ops,
            "abac_rules": abac_rules,
            "siem_export_configured": has_siem_export
        },
        "checks": {
            "scopes_allowed": scopes_allowed,
            "roles_allowed": roles_allowed,
            "issuer_allowed": issuer_allowed
        },
        "claim_evidence": [
            {
                "id": "V7-ENTERPRISE-001.2",
                "claim": "identity_and_integration_surface_enforces_sso_scim_rbac_abac_with_receipted_authz_checks",
                "evidence": {
                    "provider": requested_provider,
                    "scim_enabled_for_provider": scim_enabled,
                    "rbac_operations": access_ops,
                    "abac_rules": abac_rules,
                    "siem_export_configured": has_siem_export
                }
            }
        ]
    })))
}

fn bool_at(value: &Value, dotted_path: &str) -> bool {
    resolve_json_path(value, dotted_path)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn str_at(value: &Value, dotted_path: &str) -> String {
    resolve_json_path(value, dotted_path)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn usize_array_len_at(value: &Value, dotted_path: &str) -> usize {
    resolve_json_path(value, dotted_path)
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0)
}

fn bool_env_set(name: &str) -> bool {
    std::env::var(name)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

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
    let signed_receipts_ok = signed_algorithms
        .iter()
        .any(|algo| algo == "hmac_sha256")
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

fn percentile(samples: &[f64], p: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = (((sorted.len() - 1) as f64) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}
