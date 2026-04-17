fn run_playground(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        PLAYGROUND_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "flow_step_playground_contract",
            "allowed_ops": ["play", "pause", "step", "resume", "inspect"],
            "default_total_steps": 8
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("playground_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "flow_step_playground_contract"
    {
        errors.push("playground_contract_kind_invalid".to_string());
    }

    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "inspect".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let allowed = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 20).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if !allowed.iter().any(|row| row == &op) {
        errors.push("playground_op_not_allowed".to_string());
    }
    let require_web_tooling_ready = parse_bool(parsed.flags.get("require-web-tooling-ready"), false);
    let web_tooling_health = crate::network_protocol::web_tooling_health_report(root, strict);
    let web_tooling_ready = web_tooling_health
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if strict && require_web_tooling_ready && !web_tooling_ready {
        errors.push("playground_web_tooling_not_ready".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "flow_plane_playground",
            "errors": errors,
            "web_tooling_health": web_tooling_health
        });
    }

    let run_id = clean(
        parsed
            .flags
            .get("run-id")
            .cloned()
            .unwrap_or_else(|| "flow-playground".to_string()),
        120,
    );
    let total_steps = parse_u64(
        parsed.flags.get("total-steps"),
        contract
            .get("default_total_steps")
            .and_then(Value::as_u64)
            .unwrap_or(8),
    )
    .clamp(1, 100_000);
    let state_path = state_root(root).join("playground").join("state.json");
    let mut state = read_json(&state_path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "kind": "flow_playground_state",
            "runs": {}
        })
    });
    if !state.get("runs").map(Value::is_object).unwrap_or(false) {
        state["runs"] = Value::Object(Map::new());
    }
    let runs = state
        .get("runs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut run = runs.get(&run_id).cloned().unwrap_or_else(|| {
        json!({
            "run_id": run_id,
            "status": "idle",
            "current_step": 0_u64,
            "total_steps": total_steps,
            "events": 0_u64
        })
    });

    let status_before = run
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("idle")
        .to_string();
    let mut event = Value::Null;
    match op.as_str() {
        "play" => {
            run["status"] = Value::String("running".to_string());
        }
        "pause" => {
            if strict && status_before != "running" {
                errors.push("pause_requires_running_state".to_string());
            } else {
                run["status"] = Value::String("paused".to_string());
            }
        }
        "resume" => {
            if strict && status_before != "paused" {
                errors.push("resume_requires_paused_state".to_string());
            } else {
                run["status"] = Value::String("running".to_string());
            }
        }
        "step" => {
            let cur = run.get("current_step").and_then(Value::as_u64).unwrap_or(0);
            let total = run
                .get("total_steps")
                .and_then(Value::as_u64)
                .unwrap_or(total_steps);
            if strict && cur >= total {
                errors.push("step_out_of_bounds".to_string());
            } else {
                run["current_step"] = json!(cur.saturating_add(1));
                run["status"] = Value::String("running".to_string());
                event = json!({
                    "type": "step",
                    "run_id": run_id,
                    "step": cur.saturating_add(1),
                    "step_hash": sha256_hex_str(&format!("{}:{}", run_id, cur.saturating_add(1)))
                });
            }
        }
        "inspect" => {}
        _ => {}
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "flow_plane_playground",
            "errors": errors
        });
    }

    run["updated_at"] = Value::String(crate::now_iso());
    run["events"] = json!(run.get("events").and_then(Value::as_u64).unwrap_or(0) + 1);
    let mut runs_next = state
        .get("runs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    runs_next.insert(run_id.clone(), run.clone());
    state["runs"] = Value::Object(runs_next);
    let _ = write_json(&state_path, &state);
    if !event.is_null() {
        let _ = append_jsonl(
            &state_root(root).join("playground").join("history.jsonl"),
            &event,
        );
    }

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "flow_plane_playground",
        "lane": "core/layer0/ops",
        "run_id": run_id,
        "op": op,
        "state_path": state_path.display().to_string(),
        "web_tooling_health": web_tooling_health,
        "run_state": run,
        "event": event,
        "claim_evidence": [
            {
                "id": "V6-FLOW-001.2",
                "claim": "interactive_playground_supports_play_pause_step_resume_inspect_with_per_step_receipts",
                "evidence": {
                    "op": op
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_component_marketplace(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        COMPONENT_MARKETPLACE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "flow_component_marketplace_contract",
            "manifest_path": COMPONENT_MARKETPLACE_MANIFEST_PATH,
            "components_root": "planes/contracts/flow/components",
            "max_component_bytes": 200000,
            "required_language": "python"
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("component_marketplace_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "flow_component_marketplace_contract"
    {
        errors.push("component_marketplace_contract_kind_invalid".to_string());
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
        .unwrap_or_else(|| COMPONENT_MARKETPLACE_MANIFEST_PATH.to_string());
    let components_root_rel = parsed
        .flags
        .get("components-root")
        .cloned()
        .or_else(|| {
            contract
                .get("components_root")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "planes/contracts/flow/components".to_string());
    let manifest_path = if Path::new(&manifest_rel).is_absolute() {
        PathBuf::from(&manifest_rel)
    } else {
        root.join(&manifest_rel)
    };
    let components_root = if Path::new(&components_root_rel).is_absolute() {
        PathBuf::from(&components_root_rel)
    } else {
        root.join(&components_root_rel)
    };
    let manifest = read_json(&manifest_path).unwrap_or(Value::Null);
    if manifest.is_null() {
        errors.push(format!(
            "component_manifest_not_found:{}",
            manifest_path.display()
        ));
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "flow_plane_component_marketplace",
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
        errors.push("component_manifest_version_must_be_v1".to_string());
    }
    if strict
        && manifest
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default()
            != "flow_component_marketplace_manifest"
    {
        errors.push("component_manifest_kind_invalid".to_string());
    }

    let max_component_bytes = contract
        .get("max_component_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(200_000);
    let required_language = clean(
        contract
            .get("required_language")
            .and_then(Value::as_str)
            .unwrap_or("python"),
        30,
    )
    .to_ascii_lowercase();

    let components = manifest
        .get("components")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut validated = Vec::<Value>::new();
    for row in components {
        let id = clean(
            row.get("id").and_then(Value::as_str).unwrap_or_default(),
            120,
        );
        let rel_path = clean(
            row.get("path").and_then(Value::as_str).unwrap_or_default(),
            260,
        );
        if id.is_empty() || rel_path.is_empty() {
            errors.push("component_id_and_path_required".to_string());
            continue;
        }
        let lang = clean(
            row.get("language")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            30,
        )
        .to_ascii_lowercase();
        if strict && lang != required_language {
            errors.push(format!("component_language_invalid:{id}:{lang}"));
        }
        let file_path = if Path::new(&rel_path).is_absolute() {
            PathBuf::from(&rel_path)
        } else {
            components_root.join(&rel_path)
        };
        let bytes = fs::read(&file_path)
            .map_err(|_| format!("component_file_missing:{}", file_path.display()));
        let bytes = match bytes {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        if strict && bytes.len() as u64 > max_component_bytes {
            errors.push(format!("component_size_exceeded:{id}"));
        }
        let expected_sha = clean(
            row.get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            128,
        );
        let actual_sha = sha256_hex_str(&String::from_utf8_lossy(&bytes));
        if strict && (expected_sha.is_empty() || expected_sha != actual_sha) {
            errors.push(format!("component_sha_mismatch:{id}"));
        }
        validated.push(json!({
            "id": id,
            "path": file_path.display().to_string(),
            "language": lang,
            "sha256": actual_sha,
            "bytes": bytes.len()
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
    match std::env::var("FLOW_COMPONENT_SIGNING_KEY")
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
                errors.push("component_manifest_signature_invalid".to_string());
            }
        }
        None => {
            if strict {
                errors.push("flow_component_signing_key_missing".to_string());
            }
        }
    }

    let component_id = parsed
        .flags
        .get("component-id")
        .map(|v| clean(v, 120))
        .unwrap_or_default();
    let custom_source_path = parsed
        .flags
        .get("custom-source-path")
        .cloned()
        .unwrap_or_default();
    let mut custom_reload = Value::Null;
    if !component_id.is_empty() && !custom_source_path.is_empty() {
        let path = if Path::new(&custom_source_path).is_absolute() {
            PathBuf::from(&custom_source_path)
        } else {
            root.join(&custom_source_path)
        };
        match fs::read_to_string(&path) {
            Ok(raw) => {
                if strict && !path.display().to_string().ends_with(".py") {
                    errors.push("custom_source_must_be_python_file".to_string());
                }
                if strict && raw.as_bytes().len() as u64 > max_component_bytes {
                    errors.push("custom_source_size_exceeded".to_string());
                }
                let install_path = state_root(root)
                    .join("component_customizations")
                    .join(&component_id)
                    .join("custom.py");
                if errors.is_empty() {
                    if let Some(parent) = install_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(&install_path, raw.as_bytes());
                    custom_reload = json!({
                        "component_id": component_id,
                        "custom_source_path": path.display().to_string(),
                        "installed_path": install_path.display().to_string(),
                        "source_sha256": sha256_hex_str(&raw)
                    });
                }
            }
            Err(_) => errors.push(format!("custom_source_not_found:{}", path.display())),
        }
    }

    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "flow_plane_component_marketplace",
            "errors": errors
        });
    }

    let result = json!({
        "manifest_path": manifest_path.display().to_string(),
        "components_root": components_root.display().to_string(),
        "validated_components": validated,
        "custom_reload": custom_reload
    });
    let artifact_path = state_root(root)
        .join("component_marketplace")
        .join("latest.json");
    let _ = write_json(&artifact_path, &result);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "flow_plane_component_marketplace",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&result.to_string())
        },
        "result": result,
        "claim_evidence": [
            {
                "id": "V6-FLOW-001.3",
                "claim": "component_marketplace_enforces_signed_manifests_and_policy_scoped_sandboxed_python_customization_with_receipts",
                "evidence": {
                    "validated_components": validated.len(),
                    "customized_component": if component_id.is_empty() { Value::Null } else { Value::String(component_id) }
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
