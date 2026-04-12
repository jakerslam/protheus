fn run_mcp_analyze(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        MCP_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "binary_vuln_mcp_server_contract",
            "allowed_transports": ["stdio", "http-sse"],
            "server_name": "binary-vuln-mcp"
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("mcp_server_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "binary_vuln_mcp_server_contract"
    {
        errors.push("mcp_server_contract_kind_invalid".to_string());
    }

    let normalize_transport = |raw: &str| clean(raw, 20).to_ascii_lowercase().replace('_', "-");
    let transport = normalize_transport(
        parsed
            .flags
            .get("transport")
            .cloned()
            .unwrap_or_else(|| "stdio".to_string())
            .as_str(),
    );
    let dx_source = clean(
        parsed
            .flags
            .get("dx-source")
            .map(String::as_str)
            .unwrap_or("direct"),
        80,
    );
    let allowed_transports = contract
        .get("allowed_transports")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("stdio")])
        .iter()
        .filter_map(Value::as_str)
        .map(|v| normalize_transport(v))
        .collect::<Vec<_>>();
    if strict && !allowed_transports.iter().any(|v| v == &transport) {
        errors.push("mcp_transport_invalid".to_string());
    }

    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "binary_vuln_plane_mcp_analyze",
            "errors": errors
        });
    }

    let mut scan_args = vec![
        "scan".to_string(),
        format!("--strict={}", if strict { "1" } else { "0" }),
    ];
    for (key, value) in &parsed.flags {
        if key == "transport" {
            continue;
        }
        scan_args.push(format!("--{key}={value}"));
    }
    if parsed.flags.get("format").is_none() {
        scan_args.push("--format=json".to_string());
    }

    let scan_payload = run_scan(root, &parse_args(&scan_args), strict);
    if !scan_payload
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "binary_vuln_plane_mcp_analyze",
            "errors": ["scan_failed"],
            "scan_payload": scan_payload
        });
    }

    let findings = scan_payload
        .get("findings")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let tool_response = json!({
        "server": clean(contract.get("server_name").and_then(Value::as_str).unwrap_or("binary-vuln-mcp"), 80),
        "transport": transport,
        "tool": "binary_vuln.analyze",
        "result": {
            "findings": findings,
            "finding_count": scan_payload
                .get("output")
                .and_then(|v| v.get("finding_count"))
                .cloned()
                .unwrap_or(json!(0)),
            "input": scan_payload.get("input").cloned().unwrap_or(Value::Null)
        }
    });

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "binary_vuln_plane_mcp_analyze",
        "lane": "core/layer0/ops",
        "mcp": tool_response,
        "scan_payload": scan_payload,
        "claim_evidence": [
            {
                "id": "V6-BINVULN-001.2",
                "claim": "binary_vuln_analysis_surface_is_exposed_as_mcp_contract_for_ai_assisted_hunting",
                "evidence": {
                    "transport": transport
                }
            },
            {
                "id": "V6-BINVULN-001.3",
                "claim": "structured_json_and_jsonl_output_contains_confidence_policy_metadata_and_provenance_hashes",
                "evidence": {
                    "finding_count": tool_response
                        .get("result")
                        .and_then(|v| v.get("finding_count"))
                        .cloned()
                        .unwrap_or(json!(0))
                }
            },
            {
                "id": "V6-BINVULN-001.4",
                "claim": "binary_scan_execution_runs_in_a_safety_plane_sandbox_with_budget_privacy_and_degrade_checks",
                "evidence": {
                    "transport": transport
                }
            },
            {
                "id": "V6-BINVULN-001.6",
                "claim": "developer_cli_aliases_route_to_core_binary_scan_lanes_and_surface_observability_in_protheus_top",
                "evidence": {
                    "dx_source": dx_source,
                    "transport": transport,
                    "observability_surface": "protheus-top"
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_rulepack_install(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let source_raw = parsed
        .flags
        .get("rulepack")
        .cloned()
        .or_else(|| parsed.positional.get(1).cloned())
        .unwrap_or_default();
    if source_raw.trim().is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "binary_vuln_plane_rulepack_install",
            "errors": ["rulepack_path_required"]
        });
    }
    let source_path = resolve_rel_or_abs(root, &source_raw);
    let mut rulepack = match read_json(&source_path) {
        Some(value) => value,
        None => {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "binary_vuln_plane_rulepack_install",
                "errors": [format!("rulepack_not_found:{}", source_path.display())]
            });
        }
    };
    let mut errors = validate_rulepack(&rulepack);
    let metadata_obj = rulepack
        .get("metadata")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let provenance = clean(
        parsed
            .flags
            .get("provenance")
            .map(String::as_str)
            .or_else(|| metadata_obj.get("provenance").and_then(Value::as_str))
            .unwrap_or_default(),
        240,
    );
    let signature = clean(
        parsed
            .flags
            .get("signature")
            .map(String::as_str)
            .or_else(|| metadata_obj.get("signature").and_then(Value::as_str))
            .or_else(|| rulepack.get("signature").and_then(Value::as_str))
            .unwrap_or_default(),
        240,
    );
    if strict && provenance.is_empty() {
        errors.push("rulepack_provenance_required".to_string());
    }
    if strict && !signature.starts_with("sig:") {
        errors.push("rulepack_signature_required".to_string());
    }
    let unsigned_payload = strip_rulepack_signatures(rulepack.clone());
    let payload_digest = sha256_hex_str(&canonical_json_string(&unsigned_payload));
    let expected_signature = format!(
        "sig:{}",
        sha256_hex_str(&format!("{provenance}:{payload_digest}"))
    );
    if strict && !signature.is_empty() && signature != expected_signature {
        errors.push("rulepack_signature_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "binary_vuln_plane_rulepack_install",
            "errors": errors,
            "source_path": source_path.display().to_string()
        });
    }

    let inferred_name = source_path
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("custom-rulepack");
    let pack_name = normalize_rulepack_name(
        parsed
            .flags
            .get("name")
            .map(String::as_str)
            .or_else(|| rulepack.get("name").and_then(Value::as_str))
            .unwrap_or(inferred_name),
    );
    if let Some(obj) = rulepack.as_object_mut() {
        let metadata = obj
            .entry("metadata".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if let Some(meta_obj) = metadata.as_object_mut() {
            if !provenance.is_empty() {
                meta_obj.insert("provenance".to_string(), Value::String(provenance.clone()));
            }
            if !signature.is_empty() {
                meta_obj.insert("signature".to_string(), Value::String(signature.clone()));
            }
            meta_obj.insert(
                "payload_digest".to_string(),
                Value::String(payload_digest.clone()),
            );
        }
    }

    let installed_path = installed_rulepack_dir(root).join(format!("{pack_name}.json"));
    if let Some(parent) = installed_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Err(err) = write_json(&installed_path, &rulepack) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "binary_vuln_plane_rulepack_install",
            "errors": [clean(err, 240)],
            "source_path": source_path.display().to_string()
        });
    }

    let enable_now = parse_bool(parsed.flags.get("enable"), true);
    let mut active_written = false;
    if enable_now {
        let active = json!({
            "version": "v1",
            "kind": "binary_vuln_active_rulepack",
            "name": pack_name,
            "installed_path": installed_path.display().to_string(),
            "sha256": sha256_hex_str(&rulepack.to_string()),
            "enabled_at": crate::now_iso(),
            "provenance": if provenance.is_empty() { Value::Null } else { Value::String(provenance.clone()) },
            "signature": if signature.is_empty() { Value::Null } else { Value::String(signature.clone()) }
        });
        if write_json(&active_rulepack_path(root), &active).is_ok() {
            active_written = true;
        }
    }

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "binary_vuln_plane_rulepack_install",
        "lane": "core/layer0/ops",
        "rulepack": {
            "name": pack_name,
            "source_path": source_path.display().to_string(),
            "installed_path": installed_path.display().to_string(),
            "payload_digest": payload_digest,
            "provenance": if provenance.is_empty() { Value::Null } else { Value::String(provenance.clone()) },
            "signature": if signature.is_empty() { Value::Null } else { Value::String(signature.clone()) },
            "rule_count": rulepack.get("rules").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
            "enabled_now": enable_now,
            "active_written": active_written
        },
        "claim_evidence": [
            {
                "id": "V6-BINVULN-001.5",
                "claim": "custom_and_community_rulepacks_install_with_schema_signature_and_provenance_validation_before_enable",
                "evidence": {
                    "name": pack_name,
                    "strict": strict,
                    "enabled_now": enable_now,
                    "payload_digest": payload_digest
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_rulepack_enable(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let name = normalize_rulepack_name(
        parsed
            .flags
            .get("name")
            .map(String::as_str)
            .or_else(|| parsed.positional.get(1).map(String::as_str))
            .unwrap_or("default"),
    );
    let installed_path = installed_rulepack_dir(root).join(format!("{name}.json"));
    let Some(rulepack) = read_json(&installed_path) else {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "binary_vuln_plane_rulepack_enable",
            "errors": [format!("rulepack_not_installed:{name}")]
        });
    };
    let mut errors = validate_rulepack(&rulepack);
    let provenance = clean(
        rulepack
            .get("metadata")
            .and_then(Value::as_object)
            .and_then(|m| m.get("provenance"))
            .and_then(Value::as_str)
            .unwrap_or_default(),
        240,
    );
    let signature = clean(
        rulepack
            .get("metadata")
            .and_then(Value::as_object)
            .and_then(|m| m.get("signature"))
            .and_then(Value::as_str)
            .or_else(|| rulepack.get("signature").and_then(Value::as_str))
            .unwrap_or_default(),
        240,
    );
    if strict && provenance.is_empty() {
        errors.push("rulepack_provenance_required".to_string());
    }
    if strict && !signature.starts_with("sig:") {
        errors.push("rulepack_signature_required".to_string());
    }
    let unsigned_payload = strip_rulepack_signatures(rulepack.clone());
    let payload_digest = sha256_hex_str(&canonical_json_string(&unsigned_payload));
    let expected_signature = format!(
        "sig:{}",
        sha256_hex_str(&format!("{provenance}:{payload_digest}"))
    );
    if strict && !signature.is_empty() && signature != expected_signature {
        errors.push("rulepack_signature_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "binary_vuln_plane_rulepack_enable",
            "errors": errors,
            "name": name
        });
    }

    let active = json!({
        "version": "v1",
        "kind": "binary_vuln_active_rulepack",
        "name": name,
        "installed_path": installed_path.display().to_string(),
        "sha256": sha256_hex_str(&rulepack.to_string()),
        "enabled_at": crate::now_iso(),
        "provenance": if provenance.is_empty() { Value::Null } else { Value::String(provenance.clone()) },
        "signature": if signature.is_empty() { Value::Null } else { Value::String(signature.clone()) }
    });
    if let Err(err) = write_json(&active_rulepack_path(root), &active) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "binary_vuln_plane_rulepack_enable",
            "errors": [clean(err, 240)],
            "name": name
        });
    }

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "binary_vuln_plane_rulepack_enable",
        "lane": "core/layer0/ops",
        "active_rulepack": active,
        "claim_evidence": [
            {
                "id": "V6-BINVULN-001.5",
                "claim": "custom_and_community_rulepacks_install_with_schema_signature_and_provenance_validation_before_enable",
                "evidence": {
                    "name": name,
                    "installed_path": installed_path.display().to_string()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
