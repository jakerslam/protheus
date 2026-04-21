
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
