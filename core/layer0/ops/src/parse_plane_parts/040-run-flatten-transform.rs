fn run_flatten_transform(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        FLATTEN_TRANSFORM_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "flatten_unnest_transform_contract",
            "default_max_depth": 6,
            "default_format": "dot",
            "preserve_metadata": true
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("flatten_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "flatten_unnest_transform_contract"
    {
        errors.push("flatten_contract_kind_invalid".to_string());
    }
    let format = clean(
        parsed.flags.get("format").cloned().unwrap_or_else(|| {
            contract
                .get("default_format")
                .and_then(Value::as_str)
                .unwrap_or("dot")
                .to_string()
        }),
        20,
    )
    .to_ascii_lowercase();
    if !matches!(format.as_str(), "dot" | "slash") {
        errors.push("flatten_format_invalid".to_string());
    }
    let max_depth = parse_u64(
        parsed.flags.get("max-depth"),
        contract
            .get("default_max_depth")
            .and_then(Value::as_u64)
            .unwrap_or(6),
    )
    .clamp(1, 32) as usize;
    let preserve_metadata = contract
        .get("preserve_metadata")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let (input_hint, input) = match parse_transform_input(root, parsed) {
        Ok(ok) => ok,
        Err(err) => {
            errors.push(err);
            ("".to_string(), Value::Null)
        }
    };
    if input.is_null() {
        errors.push("transform_input_required".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "parse_plane_flatten_transform",
            "errors": errors
        });
    }

    let mut flattened = Map::<String, Value>::new();
    flatten_value("root", &input, 0, max_depth, &format, &mut flattened);
    let mut unnested_rows = Vec::<Value>::new();
    collect_unnested_rows("root", &input, 0, max_depth, &format, &mut unnested_rows);

    let result = json!({
        "input_hint": input_hint,
        "format": format,
        "max_depth": max_depth,
        "flattened": Value::Object(flattened.clone()),
        "unnested_rows": unnested_rows,
        "metadata": if preserve_metadata {
            json!({
                "input_sha256": sha256_hex_str(&canonical_json_string(&input)),
                "flattened_sha256": sha256_hex_str(&canonical_json_string(&Value::Object(flattened.clone()))),
                "preserve_metadata": true
            })
        } else {
            Value::Null
        }
    });
    let artifact_path = state_root(root).join("parse_flatten").join("latest.json");
    let _ = write_json(&artifact_path, &result);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "parse_plane_flatten_transform",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&result.to_string())
        },
        "result": result,
        "claim_evidence": [
            {
                "id": "V6-PARSE-001.4",
                "claim": "governed_flatten_and_unnest_transforms_execute_with_configurable_depth_format_and_provenance_receipts",
                "evidence": {
                    "format": format,
                    "max_depth": max_depth,
                    "flattened_keys": flattened.len()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_template_governance(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        TEMPLATE_GOVERNANCE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "parser_template_governance_contract",
            "manifest_path": TEMPLATE_MANIFEST_PATH,
            "templates_root": "planes/contracts/parse/templates",
            "required_contract_version": "v1",
            "max_review_cadence_days": 120
        }),
    );
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
        != "parser_template_governance_contract"
    {
        errors.push("template_governance_contract_kind_invalid".to_string());
    }

    let manifest_rel = parsed
        .flags
        .get("manifest")
        .cloned()
        .or_else(|| {
            contract
                .get("manifest_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| TEMPLATE_MANIFEST_PATH.to_string());
    let templates_root_rel = parsed
        .flags
        .get("templates-root")
        .cloned()
        .or_else(|| {
            contract
                .get("templates_root")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "planes/contracts/parse/templates".to_string());
    let manifest_path = if Path::new(&manifest_rel).is_absolute() {
        PathBuf::from(&manifest_rel)
    } else {
        root.join(&manifest_rel)
    };
    let templates_root = if Path::new(&templates_root_rel).is_absolute() {
        PathBuf::from(&templates_root_rel)
    } else {
        root.join(&templates_root_rel)
    };

    let manifest = read_json(&manifest_path).unwrap_or(Value::Null);
    if manifest.is_null() {
        errors.push(format!(
            "template_manifest_not_found:{}",
            manifest_path.display()
        ));
    }

    let required_contract_version = clean(
        contract
            .get("required_contract_version")
            .and_then(Value::as_str)
            .unwrap_or("v1"),
        32,
    );
    let max_review_cadence_days = contract
        .get("max_review_cadence_days")
        .and_then(Value::as_u64)
        .unwrap_or(120);

    let mut validated = Vec::<Value>::new();
    if !manifest.is_null() {
        if manifest
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            != "v1"
        {
            errors.push("template_manifest_version_must_be_v1".to_string());
        }
        if manifest
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default()
            != "parser_template_pack_manifest"
        {
            errors.push("template_manifest_kind_invalid".to_string());
        }
        let templates = manifest
            .get("templates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if templates.is_empty() {
            errors.push("template_manifest_templates_required".to_string());
        }
        for entry in templates {
            let rel_path = entry
                .get("path")
                .and_then(Value::as_str)
                .map(|v| clean(v, 260))
                .unwrap_or_default();
            if rel_path.is_empty() {
                errors.push("template_entry_path_required".to_string());
                continue;
            }
            let tpl_path = if Path::new(&rel_path).is_absolute() {
                PathBuf::from(&rel_path)
            } else {
                templates_root.join(&rel_path)
            };
            let raw = fs::read_to_string(&tpl_path)
                .map_err(|_| format!("template_file_missing:{}", tpl_path.display()));
            let Ok(raw) = raw else {
                errors.push(raw.err().unwrap_or_default());
                continue;
            };
            let expected_sha = entry
                .get("sha256")
                .and_then(Value::as_str)
                .map(|v| clean(v, 128))
                .unwrap_or_default();
            let actual_sha = sha256_hex_str(&raw);
            if expected_sha.is_empty() || expected_sha != actual_sha {
                errors.push(format!("template_sha_mismatch:{}", rel_path));
            }
            let human_reviewed = entry
                .get("human_reviewed")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if strict && !human_reviewed {
                errors.push(format!("template_not_human_reviewed:{}", rel_path));
            }
            let review_cadence_days = entry
                .get("review_cadence_days")
                .and_then(Value::as_u64)
                .unwrap_or(max_review_cadence_days + 1);
            if strict && review_cadence_days > max_review_cadence_days {
                errors.push(format!("template_review_cadence_exceeded:{}", rel_path));
            }
            let compatibility = entry
                .get("compatibility")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let mapping_contract_version = compatibility
                .get("mapping_contract_version")
                .and_then(Value::as_str)
                .map(|v| clean(v, 32))
                .unwrap_or_default();
            if strict && mapping_contract_version != required_contract_version {
                errors.push(format!(
                    "template_contract_version_incompatible:{}",
                    rel_path
                ));
            }
            validated.push(json!({
                "path": tpl_path.display().to_string(),
                "sha256": actual_sha,
                "human_reviewed": human_reviewed,
                "review_cadence_days": review_cadence_days,
                "mapping_contract_version": mapping_contract_version
            }));
        }

        let signature = manifest
            .get("signature")
            .and_then(Value::as_str)
            .map(|v| clean(v, 240))
            .unwrap_or_default();
        let mut signature_basis = manifest.clone();
        if let Some(obj) = signature_basis.as_object_mut() {
            obj.remove("signature");
        }
        match std::env::var("PARSER_TEMPLATE_SIGNING_KEY")
            .ok()
            .map(|v| clean(v, 4096))
            .filter(|v| !v.is_empty())
        {
            Some(key) => {
                let expected = format!(
                    "sig:{}",
                    sha256_hex_str(&format!(
                        "{}:{}",
                        key,
                        canonical_json_string(&signature_basis)
                    ))
                );
                if signature != expected {
                    errors.push("template_manifest_signature_invalid".to_string());
                }
            }
            None => {
                if strict {
                    errors.push("parser_template_signing_key_missing".to_string());
                }
            }
        }
    }

    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "parse_plane_template_governance",
            "errors": errors
        });
    }

    let result = json!({
        "manifest_path": manifest_path.display().to_string(),
        "templates_root": templates_root.display().to_string(),
        "validated_templates": validated,
        "required_contract_version": required_contract_version
    });
    let artifact_path = state_root(root).join("parse_templates").join("latest.json");
    let _ = write_json(&artifact_path, &result);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "parse_plane_template_governance",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&result.to_string())
        },
        "result": result,
        "claim_evidence": [
            {
                "id": "V6-PARSE-001.5",
                "claim": "signed_parser_template_mapping_library_governance_validates_compatibility_and_review_metadata",
                "evidence": {
                    "manifest_path": manifest_path.display().to_string(),
                    "validated_templates": validated.len()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

