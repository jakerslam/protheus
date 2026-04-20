
fn run_create_shadow(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        TEMPLATE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "agency_personality_template_pack_contract",
            "templates": {
                "frontend-wizard": {"specialty": "ui/ux", "default_model": "creative"},
                "security-engineer": {"specialty": "threat-analysis", "default_model": "strict"},
                "research-strategist": {"specialty": "research", "default_model": "balanced"}
            }
        }),
    );

    let mut errors = Vec::<String>::new();
    validate_contract(
        &contract,
        "agency_personality_template_pack_contract",
        "agency_template_contract_version_must_be_v1",
        "agency_template_contract_kind_invalid",
        &mut errors,
    );

    let template = clean(
        parsed
            .flags
            .get("template")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        80,
    );
    if template.is_empty() {
        errors.push("agency_template_required".to_string());
    }

    let templates = contract
        .get("templates")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let template_cfg = templates.get(&template).cloned().unwrap_or(Value::Null);
    if strict && template_cfg.is_null() {
        errors.push("agency_template_not_found".to_string());
    }

    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "agency_plane_create_shadow",
            "errors": errors
        });
    }

    let name = clean(
        parsed
            .flags
            .get("name")
            .cloned()
            .unwrap_or_else(|| format!("{}-shadow", template)),
        120,
    );
    let shadow_id = format!(
        "{}_{}",
        template.replace(' ', "-").to_ascii_lowercase(),
        &sha256_hex_str(&format!("{}:{}", template, name))[..10]
    );
    let activation = json!({
        "shadow_id": shadow_id,
        "template": template,
        "name": name,
        "activated_at": crate::now_iso(),
        "activation_receipt_hash": sha256_hex_str(&format!("{}:{}", shadow_id, template))
    });
    let artifact_path = state_root(root)
        .join("shadows")
        .join(format!("{}.json", shadow_id));
    let _ = write_json(
        &artifact_path,
        &json!({
            "version": "v1",
            "shadow_id": shadow_id,
            "template": template,
            "template_config": template_cfg,
            "activation": activation
        }),
    );

    let out = json!({
        "ok": true,
        "strict": strict,
        "type": "agency_plane_create_shadow",
        "lane": "core/layer0/ops",
        "shadow": {
            "id": shadow_id,
            "template": template,
            "name": name
        },
        "activation": activation,
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&activation.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-AGENCY-001.1",
                "claim": "personality_template_pack_supports_one_command_shadow_creation_with_activation_receipts",
                "evidence": {
                    "template": template,
                    "shadow_id": shadow_id
                }
            }
        ]
    });
    out.with_receipt_hash()
}
