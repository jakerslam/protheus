pub fn run_template_governance(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let conduit = conduit_enforcement(root, parsed, strict, "template_governance");
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return finalize_receipt(json!({
            "ok": false,
            "strict": strict,
            "type": "research_plane_template_governance",
            "errors": ["conduit_bypass_rejected"],
            "conduit_enforcement": conduit
        }));
    }

    let contract = read_json_or(
        root,
        TEMPLATE_GOVERNANCE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "template_governance_contract",
            "required_human_review": true,
            "required_reviewer": "operator",
            "signature_env": "RESEARCH_TEMPLATE_SIGNING_KEY"
        }),
    );
    let manifest_rel = parsed
        .flags
        .get("manifest")
        .cloned()
        .unwrap_or_else(|| TEMPLATE_MANIFEST_PATH.to_string());
    let manifest = read_json_or(root, &manifest_rel, Value::Null);
    let templates_root = parsed
        .flags
        .get("templates-root")
        .map(|v| {
            if Path::new(v).is_absolute() {
                PathBuf::from(v)
            } else {
                root.join(v)
            }
        })
        .unwrap_or_else(|| {
            root.join("planes")
                .join("contracts")
                .join("research")
                .join("templates")
        });

    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("template_governance_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "template_governance_contract"
    {
        errors.push("template_governance_contract_kind_invalid".to_string());
    }
    if manifest.is_null() {
        errors.push("template_manifest_missing".to_string());
    }
    let required_reviewer = contract
        .get("required_reviewer")
        .and_then(Value::as_str)
        .unwrap_or("operator")
        .to_string();
    let required_human_review = contract
        .get("required_human_review")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let signature_env = contract
        .get("signature_env")
        .and_then(Value::as_str)
        .unwrap_or("RESEARCH_TEMPLATE_SIGNING_KEY")
        .to_string();
    let signature_required = contract
        .get("signature_required")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let signing_key = std::env::var(&signature_env).unwrap_or_default();

    let templates = manifest
        .get("templates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if templates.is_empty() {
        errors.push("template_manifest_entries_required".to_string());
    }

    let mut checks = Vec::<Value>::new();
    for row in templates {
        let rel = row
            .get("path")
            .and_then(Value::as_str)
            .map(|v| clean(v, 400))
            .unwrap_or_default();
        let reviewer = row
            .get("reviewed_by")
            .and_then(Value::as_str)
            .map(|v| clean(v, 120))
            .unwrap_or_default();
        let approved = row
            .get("human_reviewed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let path = templates_root.join(&rel);
        let exists = path.exists();
        let file_hash = fs::read_to_string(&path)
            .ok()
            .map(|raw| sha256_hex_str(&raw))
            .unwrap_or_default();
        let expected_hash = row
            .get("sha256")
            .and_then(Value::as_str)
            .map(|v| clean(v, 128))
            .unwrap_or_default();
        let hash_ok = !expected_hash.is_empty() && file_hash.eq_ignore_ascii_case(&expected_hash);
        if !exists {
            errors.push(format!("missing_template::{rel}"));
        }
        if required_human_review && (!approved || reviewer != required_reviewer) {
            errors.push(format!("review_gate_failed::{rel}"));
        }
        if !hash_ok {
            errors.push(format!("hash_mismatch::{rel}"));
        }
        checks.push(json!({
            "path": rel,
            "exists": exists,
            "hash_ok": hash_ok,
            "approved": approved,
            "reviewed_by": reviewer
        }));
    }

    let signature = manifest
        .get("signature")
        .and_then(Value::as_str)
        .map(|v| clean(v, 300))
        .unwrap_or_default();
    if signature_required && signing_key.is_empty() {
        errors.push("manifest_signature_key_missing".to_string());
    } else if !signing_key.is_empty() {
        let mut basis = manifest.clone();
        if let Some(obj) = basis.as_object_mut() {
            obj.remove("signature");
        }
        let expected = format!(
            "sig:{}",
            sha256_hex_str(&format!("{signing_key}:{}", basis))
        );
        if expected != signature {
            errors.push("manifest_signature_invalid".to_string());
        }
    }

    let ok = errors.is_empty();
    let out = finalize_receipt(json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "research_plane_template_governance",
        "lane": "core/layer0/ops",
        "manifest_path": manifest_rel,
        "templates_root": templates_root.display().to_string(),
        "checks": checks,
        "conduit_enforcement": conduit,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-002.6",
                "claim": "template_pack_updates_are_governed_by_review_provenance_and_conduit_only_boundary_checks",
                "evidence": {
                    "manifest_path": manifest_rel,
                    "conduit": true
                }
            }
        ]
    }));
    out
}

#[cfg(test)]
#[path = "research_batch6_tests.rs"]
mod tests;

