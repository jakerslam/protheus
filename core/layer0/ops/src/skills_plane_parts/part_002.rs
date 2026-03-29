fn run_chain_validate(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CHAIN_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "skill_chain_contract",
            "required_chain_version": "v1",
            "require_smoke_tests": true
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("skill_chain_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "skill_chain_contract"
    {
        errors.push("skill_chain_contract_kind_invalid".to_string());
    }
    let chain = match parse_chain_input(root, parsed) {
        Ok(v) => v,
        Err(err) => {
            errors.push(err);
            Value::Null
        }
    };
    if chain.is_null() {
        errors.push("chain_required".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_chain_validate",
            "errors": errors
        });
    }

    let chain_version = chain
        .get("version")
        .and_then(Value::as_str)
        .map(|v| clean(v, 20))
        .unwrap_or_default();
    let required_chain_version = clean(
        contract
            .get("required_chain_version")
            .and_then(Value::as_str)
            .unwrap_or("v1"),
        20,
    );
    if strict && chain_version != required_chain_version {
        errors.push("chain_version_invalid".to_string());
    }

    let steps = chain
        .get("skills")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if steps.is_empty() {
        errors.push("chain_skills_required".to_string());
    }
    let require_smoke_tests = contract
        .get("require_smoke_tests")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let root_dir = skills_root(root, parsed);
    let mut test_receipts = Vec::<Value>::new();
    for (idx, step) in steps.iter().enumerate() {
        let id = clean(
            step.get("id").and_then(Value::as_str).unwrap_or_default(),
            120,
        );
        let version = clean(
            step.get("version")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            20,
        );
        if id.is_empty() || version.is_empty() {
            errors.push("chain_skill_id_and_version_required".to_string());
            continue;
        }
        let skill_dir = root_dir.join(&id);
        if strict && !skill_dir.exists() {
            errors.push(format!("chain_skill_missing:{id}"));
        }
        let smoke = skill_dir.join("tests").join("smoke.sh");
        if strict && require_smoke_tests && !smoke.exists() {
            errors.push(format!("chain_skill_smoke_missing:{id}"));
        }
        test_receipts.push(json!({
            "index": idx,
            "id": id,
            "version": version,
            "skill_dir": skill_dir.display().to_string(),
            "smoke_test_present": smoke.exists(),
            "receipt_hash": sha256_hex_str(&format!("{}:{}:{}", id, version, idx))
        }));
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_chain_validate",
            "errors": errors
        });
    }

    let chain_hash = sha256_hex_str(&canonical_json_string(&chain));
    let result = json!({
        "chain_hash": chain_hash,
        "chain": chain,
        "test_receipts": test_receipts
    });
    let artifact_path = state_root(root).join("chain_validate").join("latest.json");
    let _ = write_json(&artifact_path, &result);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "skills_plane_chain_validate",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&result.to_string())
        },
        "result": result,
        "claim_evidence": [
            {
                "id": "V6-SKILLS-001.3",
                "claim": "versioned_composable_skill_chaining_validates_contracts_and_runs_deterministic_chain_test_receipts",
                "evidence": {
                    "steps": steps.len()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn load_registry(path: &Path) -> Value {
    read_json(path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "kind": "skills_registry",
            "installed": {}
        })
    })
}

fn run_list(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        DX_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "skill_dx_contract",
            "max_packages": 2048,
            "dashboard_run_window": 1000
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("skill_dx_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "skill_dx_contract"
    {
        errors.push("skill_dx_contract_kind_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_list",
            "errors": errors
        });
    }

    let max_packages = contract
        .get("max_packages")
        .and_then(Value::as_u64)
        .unwrap_or(2048) as usize;
    let root_dir = skills_root(root, parsed);
    let mut discovered = Vec::<Value>::new();
    let mut truncated = false;

    if root_dir.exists() {
        if let Ok(entries) = fs::read_dir(&root_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if discovered.len() >= max_packages {
                    truncated = true;
                    break;
                }
                let yaml_path = path.join("skill.yaml");
                let parsed_yaml = parse_skill_yaml(&yaml_path);
                let id = clean(
                    parsed_yaml
                        .get("name")
                        .and_then(Value::as_str)
                        .filter(|v| !v.trim().is_empty())
                        .unwrap_or_else(|| {
                            path.file_name()
                                .and_then(|v| v.to_str())
                                .unwrap_or("skill-unknown")
                        }),
                    120,
                );
                discovered.push(json!({
                    "id": id,
                    "version": clean(parsed_yaml.get("version").and_then(Value::as_str).unwrap_or("v1"), 40),
                    "entrypoint": clean(parsed_yaml.get("entrypoint").and_then(Value::as_str).unwrap_or("scripts/run.sh"), 240),
                    "trigger_count": parsed_yaml
                        .get("triggers")
                        .and_then(Value::as_array)
                        .map(|rows| rows.len())
                        .unwrap_or(0),
                    "path": path.display().to_string()
                }));
            }
        }
    }

    discovered.sort_by(|a, b| {
        let left = a.get("id").and_then(Value::as_str).unwrap_or_default();
        let right = b.get("id").and_then(Value::as_str).unwrap_or_default();
        left.cmp(right)
    });

    let registry = load_registry(&state_root(root).join("registry.json"));
    let installed_map = registry
        .get("installed")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let installed_count = installed_map.len();
    let discovered_count = discovered.len();

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "skills_plane_list",
        "lane": "core/layer0/ops",
        "skills_root": root_dir.display().to_string(),
        "skills_root_rel": skills_root_default(parsed),
        "discovered_count": discovered_count,
        "installed_count": installed_count,
        "truncated": truncated,
        "skills": discovered,
        "claim_evidence": [
            {
                "id": "V6-SKILLS-001.5",
                "claim": "developer_and_user_skill_dx_exposes_create_list_run_status_wrappers_with_observability_surface",
                "evidence": {
                    "discovered_count": discovered_count,
                    "installed_count": installed_count
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_dashboard(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        DX_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "skill_dx_contract",
            "dashboard_run_window": 1000
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("skill_dx_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "skill_dx_contract"
    {
        errors.push("skill_dx_contract_kind_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_dashboard",
            "errors": errors
        });
    }

    let list_payload = run_list(root, parsed, strict);
    if !list_payload
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_dashboard",
            "errors": ["skills_list_failed"],
            "list_payload": list_payload
        });
    }

    let run_window = contract
        .get("dashboard_run_window")
        .and_then(Value::as_u64)
        .unwrap_or(1000) as usize;
    let cognition_latest_path = root.join("local/state/ops/assimilation_controller/latest.json");
    let cognition_history_path = root.join("local/state/ops/assimilation_controller/history.jsonl");
    let cognition_latest = read_json(&cognition_latest_path).unwrap_or(Value::Null);
    let cognition_history_events = fs::read_to_string(&cognition_history_path)
        .ok()
        .map(|raw| raw.lines().filter(|row| !row.trim().is_empty()).count())
        .unwrap_or(0usize);
    let run_history_path = state_root(root).join("runs").join("history.jsonl");
    let mut run_rows = load_jsonl(&run_history_path);
    if run_rows.len() > run_window {
        run_rows = run_rows.split_off(run_rows.len().saturating_sub(run_window));
    }
    let mut by_skill = std::collections::BTreeMap::<String, u64>::new();
    for row in &run_rows {
        if let Some(skill) = row.get("skill").and_then(Value::as_str) {
            let entry = by_skill.entry(clean(skill, 120)).or_insert(0);
            *entry = entry.saturating_add(1);
        }
    }
    let run_hotspots = by_skill
        .iter()
        .map(|(skill, count)| json!({"skill": skill, "runs": count}))
        .collect::<Vec<_>>();

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "skills_plane_dashboard",
        "lane": "core/layer0/ops",
        "metrics": {
            "skills_total": list_payload.get("discovered_count").cloned().unwrap_or(json!(0)),
            "skills_installed": list_payload.get("installed_count").cloned().unwrap_or(json!(0)),
            "runs_window": run_rows.len(),
            "last_run_ts": run_rows.last().and_then(|v| v.get("ts")).cloned().unwrap_or(Value::Null),
            "run_hotspots": run_hotspots
        },
        "upstream": {
            "skills_list_latest": list_payload.get("latest_path").cloned().unwrap_or(Value::Null),
            "runs_history_path": run_history_path.display().to_string(),
            "cognition_latest_path": cognition_latest_path.display().to_string(),
            "cognition_history_path": cognition_history_path.display().to_string()
        },
        "cognition": {
            "history_events": cognition_history_events,
            "latest": cognition_latest
        },
        "claim_evidence": [
            {
                "id": "V6-SKILLS-001.5",
                "claim": "developer_and_user_skill_dx_exposes_create_list_run_status_wrappers_with_observability_surface",
                "evidence": {
                    "run_window": run_rows.len()
                }
            },
            {
                "id": "V6-COGNITION-012.5",
                "claim": "skills_dashboard_surfaces_history_and_latest_state_from_core_receipt_ledger",
                "evidence": {
                    "history_events": cognition_history_events,
                    "latest_type": cognition_latest
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn resolve_rel_or_abs(root: &Path, raw: &str) -> PathBuf {
    if Path::new(raw).is_absolute() {
        PathBuf::from(raw)
    } else {
        root.join(raw)
    }
}

fn gallery_root(root: &Path, parsed: &crate::ParsedArgs) -> PathBuf {
    let rel_or_abs = parsed
        .flags
        .get("gallery-root")
        .cloned()
        .unwrap_or_else(|| "client/runtime/systems/skills/gallery".to_string());
    resolve_rel_or_abs(root, &rel_or_abs)
}

fn gallery_manifest_path(root: &Path, parsed: &crate::ParsedArgs) -> PathBuf {
    let rel_or_abs = parsed
        .flags
        .get("manifest")
        .cloned()
        .unwrap_or_else(|| GALLERY_MANIFEST_PATH.to_string());
    resolve_rel_or_abs(root, &rel_or_abs)
}
