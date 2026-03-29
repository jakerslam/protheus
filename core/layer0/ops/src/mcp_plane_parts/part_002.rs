fn run_pattern_pack(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        PATTERN_PACK_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "mcp_pattern_pack_contract",
            "allowed_patterns": ["router", "map-reduce", "fanout", "sequential"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("mcp_pattern_pack_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "mcp_pattern_pack_contract"
    {
        errors.push("mcp_pattern_pack_contract_kind_invalid".to_string());
    }
    let pattern = clean(
        parsed
            .flags
            .get("pattern")
            .cloned()
            .unwrap_or_else(|| "router".to_string()),
        40,
    )
    .to_ascii_lowercase();
    let allowed = contract
        .get("allowed_patterns")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 40).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if strict && !allowed.iter().any(|row| row == &pattern) {
        errors.push("pattern_not_allowed".to_string());
    }
    let tasks = parse_csv_flag(&parsed.flags, "tasks", 200);
    let steps = if let Some(raw) = parsed.flags.get("steps-json") {
        serde_json::from_str::<Value>(raw)
            .ok()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(|v| clean(v, 120))
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>()
    } else {
        match pattern.as_str() {
            "map-reduce" => vec![
                "map_discovery".to_string(),
                "map_execution".to_string(),
                "reduce_merge".to_string(),
            ],
            "orchestrator" => vec![
                "orchestrator_plan".to_string(),
                "orchestrator_dispatch".to_string(),
                "orchestrator_collect".to_string(),
                "orchestrator_finalize".to_string(),
            ],
            "evaluator" => vec![
                "candidate_generate".to_string(),
                "candidate_score".to_string(),
                "candidate_select".to_string(),
            ],
            "swarm" => vec![
                "swarm_spawn".to_string(),
                "swarm_consensus".to_string(),
                "swarm_merge".to_string(),
            ],
            "fanout" => vec![
                "fanout_split".to_string(),
                "parallel_execute".to_string(),
                "aggregate_outputs".to_string(),
            ],
            "sequential" => vec![
                "step_one".to_string(),
                "step_two".to_string(),
                "step_three".to_string(),
            ],
            _ => vec![
                "router_select".to_string(),
                "router_dispatch".to_string(),
                "router_merge".to_string(),
            ],
        }
    };
    if steps.is_empty() {
        errors.push("pattern_steps_required".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "mcp_plane_pattern_pack",
            "errors": errors
        });
    }

    let step_receipts = steps
        .iter()
        .enumerate()
        .map(|(idx, step)| {
            json!({
                "index": idx,
                "step": step,
                "step_hash": sha256_hex_str(&format!("{}:{}:{}", pattern, idx, step))
            })
        })
        .collect::<Vec<_>>();
    let result = json!({
        "pattern": pattern,
        "tasks": tasks,
        "steps": steps,
        "step_receipts": step_receipts
    });
    let artifact_path = state_root(root).join("pattern_pack").join("latest.json");
    let _ = write_json(&artifact_path, &result);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "mcp_plane_pattern_pack",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&result.to_string())
        },
        "result": result,
        "claim_evidence": [
            {
                "id": "V6-MCP-001.4",
                "claim": "declarative_composable_workflow_pattern_pack_compiles_to_deterministic_steps_with_receipts",
                "evidence": {
                    "pattern": pattern,
                    "step_count": steps.len()
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
            "kind": "mcp_template_governance_contract",
            "manifest_path": TEMPLATE_MANIFEST_PATH,
            "templates_root": "planes/contracts/mcp/templates",
            "required_mcp_version": "v1",
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
        errors.push("mcp_template_governance_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "mcp_template_governance_contract"
    {
        errors.push("mcp_template_governance_contract_kind_invalid".to_string());
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
        .unwrap_or_else(|| "planes/contracts/mcp/templates".to_string());
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
            "mcp_template_manifest_not_found:{}",
            manifest_path.display()
        ));
    }

    let required_mcp_version = clean(
        contract
            .get("required_mcp_version")
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
            errors.push("mcp_template_manifest_version_must_be_v1".to_string());
        }
        if manifest
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default()
            != "mcp_template_pack_manifest"
        {
            errors.push("mcp_template_manifest_kind_invalid".to_string());
        }
        let templates = manifest
            .get("templates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if templates.is_empty() {
            errors.push("mcp_template_manifest_templates_required".to_string());
        }

        for entry in templates {
            let rel_path = entry
                .get("path")
                .and_then(Value::as_str)
                .map(|v| clean(v, 260))
                .unwrap_or_default();
            if rel_path.is_empty() {
                errors.push("mcp_template_entry_path_required".to_string());
                continue;
            }
            let tpl_path = if Path::new(&rel_path).is_absolute() {
                PathBuf::from(&rel_path)
            } else {
                templates_root.join(&rel_path)
            };
            let raw_res = fs::read_to_string(&tpl_path)
                .map_err(|_| format!("mcp_template_file_missing:{}", tpl_path.display()));
            let raw = match raw_res {
                Ok(v) => v,
                Err(err) => {
                    errors.push(err);
                    continue;
                }
            };
            let expected_sha = entry
                .get("sha256")
                .and_then(Value::as_str)
                .map(|v| clean(v, 128))
                .unwrap_or_default();
            let actual_sha = sha256_hex_str(&raw);
            if expected_sha.is_empty() || expected_sha != actual_sha {
                errors.push(format!("mcp_template_sha_mismatch:{}", rel_path));
            }
            let human_reviewed = entry
                .get("human_reviewed")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if strict && !human_reviewed {
                errors.push(format!("mcp_template_not_human_reviewed:{}", rel_path));
            }
            let review_cadence_days = entry
                .get("review_cadence_days")
                .and_then(Value::as_u64)
                .unwrap_or(max_review_cadence_days + 1);
            if strict && review_cadence_days > max_review_cadence_days {
                errors.push(format!("mcp_template_review_cadence_exceeded:{}", rel_path));
            }
            let mcp_version = entry
                .get("compatibility")
                .and_then(Value::as_object)
                .and_then(|row| row.get("mcp_version"))
                .and_then(Value::as_str)
                .map(|v| clean(v, 32))
                .unwrap_or_default();
            if strict && mcp_version != required_mcp_version {
                errors.push(format!("mcp_template_version_incompatible:{}", rel_path));
            }

            validated.push(json!({
                "path": tpl_path.display().to_string(),
                "sha256": actual_sha,
                "human_reviewed": human_reviewed,
                "review_cadence_days": review_cadence_days,
                "mcp_version": mcp_version
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
        match std::env::var("MCP_TEMPLATE_SIGNING_KEY")
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
                    errors.push("mcp_template_manifest_signature_invalid".to_string());
                }
            }
            None => {
                if strict {
                    errors.push("mcp_template_signing_key_missing".to_string());
                }
            }
        }
    }

    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "mcp_plane_template_governance",
            "errors": errors
        });
    }

    let result = json!({
        "manifest_path": manifest_path.display().to_string(),
        "templates_root": templates_root.display().to_string(),
        "validated_templates": validated,
        "required_mcp_version": required_mcp_version
    });
    let artifact_path = state_root(root)
        .join("template_governance")
        .join("latest.json");
    let _ = write_json(&artifact_path, &result);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "mcp_plane_template_governance",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&result.to_string())
        },
        "result": result,
        "claim_evidence": [
            {
                "id": "V6-MCP-001.5",
                "claim": "signed_mcp_template_library_governance_validates_compatibility_metadata_and_review_cadence",
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

fn curated_templates() -> Vec<Value> {
    let rows = [
        ("router", "routing"),
        ("map-reduce", "aggregation"),
        ("fanout", "routing"),
        ("sequential", "workflow"),
        ("evaluator", "evaluation"),
        ("swarm", "coordination"),
        ("reviewer", "evaluation"),
        ("planner", "workflow"),
        ("critic", "evaluation"),
        ("judge", "evaluation"),
        ("consensus", "coordination"),
        ("scheduler", "workflow"),
        ("pipeline", "workflow"),
        ("reflector", "reasoning"),
        ("verifier", "reasoning"),
        ("incident-triage", "ops"),
        ("coding-loop", "coding"),
        ("retriever", "retrieval"),
        ("memory-sync", "retrieval"),
        ("tool-router", "routing"),
        ("broadcaster", "coordination"),
        ("aggregator", "aggregation"),
        ("handoff", "coordination"),
        ("pause-resume", "workflow"),
        ("oauth-gated-sampler", "policy"),
    ];
    rows.iter()
        .map(|(id, category)| {
            json!({
                "id": id,
                "category": category,
                "receipt_required": true,
                "policy_gates": ["auth.session", "sampling.request", "roots.enumerate"]
            })
        })
        .collect()
}

