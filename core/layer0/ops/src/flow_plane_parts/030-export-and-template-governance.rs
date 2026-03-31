fn run_export(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        EXPORT_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "flow_export_compiler_contract",
            "allowed_formats": ["json", "api", "mcp"],
            "default_format": "json",
            "default_package_version": "v1"
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("flow_export_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "flow_export_compiler_contract"
    {
        errors.push("flow_export_contract_kind_invalid".to_string());
    }

    let format = clean(
        parsed
            .flags
            .get("format")
            .cloned()
            .or_else(|| {
                contract
                    .get("default_format")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
            .unwrap_or_else(|| "json".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let allowed_formats = contract
        .get("allowed_formats")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 20).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if strict && !allowed_formats.iter().any(|row| row == &format) {
        errors.push("flow_export_format_not_allowed".to_string());
    }

    let from_rel = parsed.flags.get("from-path").cloned().unwrap_or_else(|| {
        state_root(root)
            .join("compile")
            .join("latest.json")
            .display()
            .to_string()
    });
    let from_path = if Path::new(&from_rel).is_absolute() {
        PathBuf::from(&from_rel)
    } else {
        root.join(&from_rel)
    };
    let compiled = read_json(&from_path).unwrap_or(Value::Null);
    if compiled.is_null() {
        errors.push(format!("compiled_graph_missing:{}", from_path.display()));
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "flow_plane_export",
            "errors": errors
        });
    }

    let package_version = clean(
        parsed
            .flags
            .get("package-version")
            .cloned()
            .or_else(|| {
                contract
                    .get("default_package_version")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
            .unwrap_or_else(|| "v1".to_string()),
        40,
    );
    let package = json!({
        "version": package_version,
        "kind": "flow_export_package",
        "format": format,
        "compiled_graph_path": from_path.display().to_string(),
        "compiled_graph_sha256": sha256_hex_str(&compiled.to_string()),
        "export_payload": match format.as_str() {
            "api" => json!({
                "entrypoint": "/api/flow/run",
                "method": "POST",
                "body_schema": {"graph": "flow_execution_graph"}
            }),
            "mcp" => json!({
                "server": "flow-export",
                "tool": "run_flow",
                "input_schema": {"graph": "flow_execution_graph"}
            }),
            _ => compiled.clone()
        }
    });
    let artifact_path = state_root(root)
        .join("export")
        .join(&format)
        .join("latest.json");
    let _ = write_json(&artifact_path, &package);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "flow_plane_export",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&package.to_string())
        },
        "package": package,
        "claim_evidence": [
            {
                "id": "V6-FLOW-001.4",
                "claim": "one_click_flow_packaging_exports_versioned_json_api_and_mcp_artifacts_with_deterministic_hashes",
                "evidence": {
                    "format": format,
                    "package_version": package_version
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
            "kind": "flow_template_governance_contract",
            "manifest_path": TEMPLATE_MANIFEST_PATH,
            "templates_root": "planes/contracts/flow/templates",
            "required_canvas_version": "v1",
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
        errors.push("flow_template_governance_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "flow_template_governance_contract"
    {
        errors.push("flow_template_governance_contract_kind_invalid".to_string());
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
        .unwrap_or_else(|| "planes/contracts/flow/templates".to_string());
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
            "flow_template_manifest_not_found:{}",
            manifest_path.display()
        ));
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "flow_plane_template_governance",
            "errors": errors
        });
    }

    if strict
        && manifest
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            != "v1"
    {
        errors.push("flow_template_manifest_version_must_be_v1".to_string());
    }
    if strict
        && manifest
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default()
            != "flow_template_pack_manifest"
    {
        errors.push("flow_template_manifest_kind_invalid".to_string());
    }
    let required_canvas_version = clean(
        contract
            .get("required_canvas_version")
            .and_then(Value::as_str)
            .unwrap_or("v1"),
        20,
    );
    let max_review_cadence_days = contract
        .get("max_review_cadence_days")
        .and_then(Value::as_u64)
        .unwrap_or(120);

    let templates = manifest
        .get("templates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if templates.is_empty() {
        errors.push("flow_template_manifest_templates_required".to_string());
    }
    let mut validated = Vec::<Value>::new();
    for entry in templates {
        let rel_path = clean(
            entry
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            260,
        );
        if rel_path.is_empty() {
            errors.push("flow_template_entry_path_required".to_string());
            continue;
        }
        let path = if Path::new(&rel_path).is_absolute() {
            PathBuf::from(&rel_path)
        } else {
            templates_root.join(&rel_path)
        };
        let raw = fs::read_to_string(&path)
            .map_err(|_| format!("flow_template_missing:{}", path.display()));
        let raw = match raw {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let expected_sha = clean(
            entry
                .get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            128,
        );
        let actual_sha = sha256_hex_str(&raw);
        if strict && (expected_sha.is_empty() || expected_sha != actual_sha) {
            errors.push(format!("flow_template_sha_mismatch:{}", rel_path));
        }
        let human_reviewed = entry
            .get("human_reviewed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if strict && !human_reviewed {
            errors.push(format!("flow_template_not_human_reviewed:{}", rel_path));
        }
        let review_cadence_days = entry
            .get("review_cadence_days")
            .and_then(Value::as_u64)
            .unwrap_or(max_review_cadence_days + 1);
        if strict && review_cadence_days > max_review_cadence_days {
            errors.push(format!(
                "flow_template_review_cadence_exceeded:{}",
                rel_path
            ));
        }
        let canvas_version = entry
            .get("compatibility")
            .and_then(Value::as_object)
            .and_then(|row| row.get("canvas_version"))
            .and_then(Value::as_str)
            .map(|v| clean(v, 20))
            .unwrap_or_default();
        if strict && canvas_version != required_canvas_version {
            errors.push(format!(
                "flow_template_canvas_version_incompatible:{}",
                rel_path
            ));
        }
        validated.push(json!({
            "path": path.display().to_string(),
            "sha256": actual_sha,
            "human_reviewed": human_reviewed,
            "review_cadence_days": review_cadence_days,
            "canvas_version": canvas_version
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
    match std::env::var("FLOW_TEMPLATE_SIGNING_KEY")
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
            if strict && signature != expected {
                errors.push("flow_template_manifest_signature_invalid".to_string());
            }
        }
        None => {
            if strict {
                errors.push("flow_template_signing_key_missing".to_string());
            }
        }
    }

    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "flow_plane_template_governance",
            "errors": errors
        });
    }

    let result = json!({
        "manifest_path": manifest_path.display().to_string(),
        "templates_root": templates_root.display().to_string(),
        "validated_templates": validated
    });
    let artifact_path = state_root(root)
        .join("template_governance")
        .join("latest.json");
    let _ = write_json(&artifact_path, &result);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "flow_plane_template_governance",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&result.to_string())
        },
        "result": result,
        "claim_evidence": [
            {
                "id": "V6-FLOW-001.5",
                "claim": "curated_visual_flow_template_library_governance_enforces_signature_review_cadence_and_deterministic_install_receipts",
                "evidence": {
                    "validated_templates": validated.len()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

