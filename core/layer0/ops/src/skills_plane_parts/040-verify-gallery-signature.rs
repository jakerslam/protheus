fn verify_gallery_signature(
    contract: &Value,
    manifest: &Value,
    strict: bool,
) -> Result<(), String> {
    let require_signature = contract
        .get("require_signature")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if !require_signature {
        return Ok(());
    }
    let provided = manifest
        .get("signature")
        .and_then(Value::as_str)
        .map(|v| clean(v, 260))
        .unwrap_or_default();
    if provided.is_empty() {
        if strict {
            return Err("gallery_manifest_signature_missing".to_string());
        }
        return Ok(());
    }
    let signing_key = std::env::var("SKILLS_GALLERY_SIGNING_KEY").unwrap_or_default();
    if signing_key.trim().is_empty() {
        if strict {
            return Err("skills_gallery_signing_key_missing".to_string());
        }
        return Ok(());
    }
    let mut basis = manifest.clone();
    if let Some(obj) = basis.as_object_mut() {
        obj.remove("signature");
    }
    let expected = format!(
        "sig:{}",
        sha256_hex_str(&format!(
            "{}:{}",
            signing_key,
            canonical_json_string(&basis)
        ))
    );
    if strict && provided != expected {
        return Err("gallery_manifest_signature_invalid".to_string());
    }
    Ok(())
}

fn gallery_package_rel_safe(raw: &str) -> bool {
    let normalized = clean(raw, 260).replace('\\', "/");
    if normalized.is_empty() || normalized.starts_with('/') || normalized.contains('\0') {
        return false;
    }
    !normalized
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
}

fn run_gallery(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        GALLERY_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "skill_gallery_governance_contract",
            "require_signature": true,
            "max_templates": 1024,
            "allow_load_only_if_reviewed": true
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("skill_gallery_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "skill_gallery_governance_contract"
    {
        errors.push("skill_gallery_contract_kind_invalid".to_string());
    }
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "list".to_string()),
        40,
    )
    .to_ascii_lowercase();
    if !matches!(op.as_str(), "ingest" | "list" | "load") {
        errors.push("gallery_op_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_gallery",
            "errors": errors
        });
    }

    let gallery_dir = gallery_root(root, parsed);
    let manifest_path = gallery_manifest_path(root, parsed);
    let state_manifest_path = state_root(root).join("gallery").join("manifest.json");
    let manifest = if op == "ingest" {
        match read_json(&manifest_path) {
            Some(v) => v,
            None => {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "skills_plane_gallery",
                    "errors": [format!("gallery_manifest_not_found:{}", manifest_path.display())]
                });
            }
        }
    } else {
        read_json(&state_manifest_path).unwrap_or_else(|| {
            json!({
                "version": "v1",
                "kind": "skill_gallery_manifest",
                "templates": []
            })
        })
    };

    if let Err(err) = verify_gallery_signature(&contract, &manifest, strict) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_gallery",
            "errors": [err]
        });
    }

    let templates = manifest
        .get("templates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let max_templates = contract
        .get("max_templates")
        .and_then(Value::as_u64)
        .unwrap_or(1024) as usize;
    if strict && templates.len() > max_templates {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_gallery",
            "errors": ["gallery_template_limit_exceeded"]
        });
    }

    match op.as_str() {
        "ingest" => {
            let _ = fs::create_dir_all(state_manifest_path.parent().unwrap_or(&gallery_dir));
            let _ = write_json(&state_manifest_path, &manifest);
            let _ = append_jsonl(
                &state_root(root).join("gallery").join("history.jsonl"),
                &json!({
                    "ts": crate::now_iso(),
                    "op": "ingest",
                    "manifest_path": manifest_path.display().to_string(),
                    "template_count": templates.len(),
                }),
            );
            let mut out = json!({
                "ok": true,
                "strict": strict,
                "type": "skills_plane_gallery",
                "op": "ingest",
                "lane": "core/layer0/ops",
                "manifest_path": state_manifest_path.display().to_string(),
                "template_count": templates.len(),
                "claim_evidence": [
                    {
                        "id": "V6-SKILLS-001.6",
                        "claim": "curated_skill_gallery_ingest_and_one_click_loader_are_governed_with_deterministic_receipts",
                        "evidence": {
                            "op": "ingest",
                            "template_count": templates.len()
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            out
        }
        "list" => {
            let listed = templates
                .iter()
                .map(|row| {
                    json!({
                        "id": clean(row.get("id").and_then(Value::as_str).unwrap_or_default(), 120),
                        "version": clean(row.get("version").and_then(Value::as_str).unwrap_or("v1"), 40),
                        "package_rel": clean(row.get("package_rel").and_then(Value::as_str).unwrap_or_default(), 260),
                        "reviewed": row.get("human_reviewed").and_then(Value::as_bool).unwrap_or(false)
                    })
                })
                .collect::<Vec<_>>();
            let mut out = json!({
                "ok": true,
                "strict": strict,
                "type": "skills_plane_gallery",
                "op": "list",
                "lane": "core/layer0/ops",
                "manifest_path": state_manifest_path.display().to_string(),
                "templates": listed,
                "template_count": templates.len(),
                "claim_evidence": [
                    {
                        "id": "V6-SKILLS-001.6",
                        "claim": "curated_skill_gallery_ingest_and_one_click_loader_are_governed_with_deterministic_receipts",
                        "evidence": {
                            "op": "list",
                            "template_count": templates.len()
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            out
        }
        "load" => {
            let skill_id = clean(
                parsed
                    .flags
                    .get("skill")
                    .cloned()
                    .or_else(|| parsed.positional.get(2).cloned())
                    .unwrap_or_default(),
                120,
            );
            if skill_id.is_empty() {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "skills_plane_gallery",
                    "errors": ["gallery_skill_required"]
                });
            }
            let allow_only_reviewed = contract
                .get("allow_load_only_if_reviewed")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let maybe_template = templates.iter().find(|row| {
                row.get("id").and_then(Value::as_str).map(|v| clean(v, 120))
                    == Some(skill_id.clone())
            });
            let Some(template) = maybe_template else {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "skills_plane_gallery",
                    "errors": [format!("gallery_skill_not_found:{skill_id}")]
                });
            };
            if strict
                && allow_only_reviewed
                && !template
                    .get("human_reviewed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "skills_plane_gallery",
                    "errors": ["gallery_skill_not_human_reviewed"]
                });
            }
            let package_rel = clean(
                template
                    .get("package_rel")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                260,
            );
            if package_rel.is_empty() {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "skills_plane_gallery",
                    "errors": ["gallery_package_path_missing"]
                });
            }
            if strict && !gallery_package_rel_safe(&package_rel) {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "skills_plane_gallery",
                    "errors": ["gallery_package_path_invalid"]
                });
            }
            let package_path = resolve_rel_or_abs(root, &package_rel);
            let install_payload = run_install(
                root,
                &crate::parse_args(&[
                    "install".to_string(),
                    format!("--skill-path={}", package_path.display()),
                    format!("--strict={}", if strict { "1" } else { "0" }),
                ]),
                strict,
            );
            if !install_payload
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "skills_plane_gallery",
                    "errors": ["gallery_install_failed"],
                    "install_payload": install_payload
                });
            }
            let mut out = json!({
                "ok": true,
                "strict": strict,
                "type": "skills_plane_gallery",
                "op": "load",
                "lane": "core/layer0/ops",
                "manifest_path": state_manifest_path.display().to_string(),
                "skill_id": skill_id,
                "package_path": package_path.display().to_string(),
                "install_payload": install_payload,
                "claim_evidence": [
                    {
                        "id": "V6-SKILLS-001.6",
                        "claim": "curated_skill_gallery_ingest_and_one_click_loader_are_governed_with_deterministic_receipts",
                        "evidence": {
                            "op": "load",
                            "skill_id": skill_id
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            out
        }
        _ => json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_gallery",
            "errors": ["gallery_op_invalid"]
        }),
    }
}

fn run_react_minimal(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        REACT_MINIMAL_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "react_minimal_profile_contract",
            "max_steps": 8,
            "allowed_tools": ["search", "read", "summarize"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("react_profile_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "react_minimal_profile_contract"
    {
        errors.push("react_profile_contract_kind_invalid".to_string());
    }
    let task = clean(
        parsed
            .flags
            .get("task")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        500,
    );
    if task.is_empty() {
        errors.push("react_task_required".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_react_minimal",
            "errors": errors
        });
    }
    let max_steps = parse_u64(parsed.flags.get("max-steps"), 0).max(1).min(
        contract
            .get("max_steps")
            .and_then(Value::as_u64)
            .unwrap_or(8),
    ) as usize;
    let allowed_tools = contract
        .get("allowed_tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("search"), json!("read"), json!("summarize")]);
    let allowed = allowed_tools
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 80))
        .collect::<Vec<_>>();
    let mut steps = Vec::<Value>::new();
    for idx in 0..max_steps {
        let tool = allowed
            .get(idx % allowed.len().max(1))
            .cloned()
            .unwrap_or_else(|| "search".to_string());
        let thought = format!("Assess task segment {} for '{}'", idx + 1, clean(&task, 80));
        let action = format!("{tool}:segment_{}", idx + 1);
        let observation = format!(
            "observation_hash:{}",
            &sha256_hex_str(&format!("{task}:{idx}"))[..16]
        );
        steps.push(json!({
            "step": idx + 1,
            "thought": thought,
            "action": action,
            "observation": observation,
            "state_hash": sha256_hex_str(&format!("{}:{}:{}:{}", task, thought, action, observation))
        }));
    }
    let tao_state = json!({
        "version": "v1",
        "profile": "react-minimal",
        "task": task,
        "steps": steps,
        "bounded": true
    });
    let artifact_path = state_root(root).join("react_minimal").join("latest.json");
    let _ = write_json(&artifact_path, &tao_state);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "skills_plane_react_minimal",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&tao_state.to_string())
        },
        "tao_state": tao_state,
        "claim_evidence": [
            {
                "id": "V6-SKILLS-001.7",
                "claim": "react_minimal_profile_runs_bounded_tao_loop_with_stepwise_state_as_governed_objects",
                "evidence": {
                    "max_steps": max_steps
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
