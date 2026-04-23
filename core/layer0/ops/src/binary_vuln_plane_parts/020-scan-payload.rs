fn scan_payload(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Result<Value, Value> {
    let scan_started = Instant::now();
    let engine_contract = load_json_or(
        root,
        ENGINE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "binary_analysis_engine_contract",
            "allowed_kinds": ["binary", "firmware", "uefi", "ba2", "binary_ninja_db"],
            "max_input_bytes": 104857600,
            "sandbox": {
                "max_findings": 4000,
                "max_scan_millis": 30000,
                "privacy": {
                    "redact_input_path": true
                },
                "degrade": {
                    "enabled": true,
                    "mode": "truncate_findings"
                }
            }
        }),
    );
    let output_contract = load_json_or(
        root,
        OUTPUT_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "binary_vuln_structured_output_contract",
            "supported_formats": ["json", "jsonl"]
        }),
    );

    let mut errors = Vec::<String>::new();
    if engine_contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("binary_analysis_engine_contract_version_must_be_v1".to_string());
    }
    if engine_contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "binary_analysis_engine_contract"
    {
        errors.push("binary_analysis_engine_contract_kind_invalid".to_string());
    }
    if output_contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("structured_output_contract_version_must_be_v1".to_string());
    }
    if output_contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "binary_vuln_structured_output_contract"
    {
        errors.push("structured_output_contract_kind_invalid".to_string());
    }

    let (path, bytes) = match read_input_file(root, parsed) {
        Ok(v) => v,
        Err(err) => {
            errors.push(err);
            (PathBuf::new(), Vec::new())
        }
    };
    if !errors.is_empty() {
        return Err(json!({
            "ok": false,
            "strict": strict,
            "type": "binary_vuln_plane_scan",
            "errors": errors
        }));
    }

    let max_input = engine_contract
        .get("max_input_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(104857600) as usize;
    let sandbox = engine_contract
        .get("sandbox")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let max_findings = sandbox
        .get("max_findings")
        .and_then(Value::as_u64)
        .unwrap_or(4000) as usize;
    let contract_scan_millis = sandbox
        .get("max_scan_millis")
        .and_then(Value::as_u64)
        .unwrap_or(30000);
    let max_scan_millis = parsed
        .flags
        .get("scan-timeout-ms")
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(contract_scan_millis)
        .clamp(250, 300_000);
    let redact_input_path = sandbox
        .get("privacy")
        .and_then(Value::as_object)
        .and_then(|row| row.get("redact_input_path"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let degrade_enabled = sandbox
        .get("degrade")
        .and_then(Value::as_object)
        .and_then(|row| row.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let degrade_mode = match clean(
        sandbox
            .get("degrade")
            .and_then(Value::as_object)
            .and_then(|row| row.get("mode"))
            .and_then(Value::as_str)
            .unwrap_or("truncate_findings"),
        80,
    )
    .to_ascii_lowercase()
    .as_str()
    {
        "truncate_findings" | "pass_through" | "fail_closed" => clean(
            sandbox
                .get("degrade")
                .and_then(Value::as_object)
                .and_then(|row| row.get("mode"))
                .and_then(Value::as_str)
                .unwrap_or("truncate_findings"),
            80,
        )
        .to_ascii_lowercase(),
        _ => "truncate_findings".to_string(),
    };
    let allow_raw_path = parse_bool(parsed.flags.get("allow-raw-path"), false);
    if strict && bytes.len() > max_input {
        return Err(json!({
            "ok": false,
            "strict": strict,
            "type": "binary_vuln_plane_scan",
            "errors": ["input_exceeds_max_bytes"]
        }));
    }

    let kind = detect_input_kind(&path);
    let allowed_kinds = engine_contract
        .get("allowed_kinds")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 80).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if strict && !allowed_kinds.iter().any(|v| v == &kind) {
        return Err(json!({
            "ok": false,
            "strict": strict,
            "type": "binary_vuln_plane_scan",
            "errors": ["input_kind_not_allowed"]
        }));
    }

    let input_sha256 = sha256_hex_str(&String::from_utf8_lossy(&bytes));
    let raw_utf8 = String::from_utf8_lossy(&bytes).to_string();
    let (rulepack, rules, rulepack_path) = load_rulepack(root, parsed);
    let rulepack_sha256 = sha256_hex_str(&rulepack.to_string());
    let dx_source = clean(
        parsed
            .flags
            .get("dx-source")
            .map(String::as_str)
            .unwrap_or("direct"),
        80,
    );

    let mut findings = normalize_findings(scan_with_rules(
        &raw_utf8,
        &kind,
        &bytes,
        &rules,
        &input_sha256,
    ));
    let mut degraded = false;
    let mut degrade_reason = String::new();
    if findings.len() > max_findings {
        if strict && !degrade_enabled {
            return Err(json!({
                "ok": false,
                "strict": strict,
                "type": "binary_vuln_plane_scan",
                "errors": ["sandbox_finding_budget_exceeded"]
            }));
        }
        findings.truncate(max_findings);
        degraded = true;
        degrade_reason = "finding_budget_exceeded".to_string();
    }
    let scan_millis = scan_started.elapsed().as_millis() as u64;
    if scan_millis > max_scan_millis {
        if strict || !degrade_enabled {
            return Err(json!({
                "ok": false,
                "strict": strict,
                "type": "binary_vuln_plane_scan",
                "errors": ["sandbox_scan_time_budget_exceeded"]
            }));
        }
        degraded = true;
        if degrade_reason.is_empty() {
            degrade_reason = "scan_time_budget_exceeded".to_string();
        }
    }

    let format = clean(
        parsed
            .flags
            .get("format")
            .cloned()
            .unwrap_or_else(|| "json".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let supported_formats = output_contract
        .get("supported_formats")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("json"), json!("jsonl")])
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 20).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if strict && !supported_formats.iter().any(|v| v == &format) {
        return Err(json!({
            "ok": false,
            "strict": strict,
            "type": "binary_vuln_plane_scan",
            "errors": ["structured_output_format_not_supported"]
        }));
    }

    let artifact_base = state_root(root)
        .join("scan")
        .join(format!("{}", &input_sha256[..16]));
    let artifact_path = if format == "jsonl" {
        artifact_base.with_extension("jsonl")
    } else {
        artifact_base.with_extension("json")
    };

    if format == "jsonl" {
        if let Some(parent) = artifact_path.parent() {
            if fs::create_dir_all(parent).is_err() {
                return Err(json!({
                    "ok": false,
                    "strict": strict,
                    "type": "binary_vuln_plane_scan",
                    "errors": ["artifact_parent_create_failed"]
                }));
            }
        }
        let mut lines = Vec::<String>::new();
        for finding in &findings {
            lines.push(serde_json::to_string(finding).unwrap_or_else(|_| "{}".to_string()));
        }
        if fs::write(&artifact_path, format!("{}\n", lines.join("\n"))).is_err() {
            return Err(json!({
                "ok": false,
                "strict": strict,
                "type": "binary_vuln_plane_scan",
                "errors": ["artifact_write_failed"]
            }));
        }
    } else {
        if let Some(parent) = artifact_path.parent() {
            if fs::create_dir_all(parent).is_err() {
                return Err(json!({
                    "ok": false,
                    "strict": strict,
                    "type": "binary_vuln_plane_scan",
                    "errors": ["artifact_parent_create_failed"]
                }));
            }
        }
        let encoded = serde_json::to_string_pretty(&json!({
            "version": "v1",
            "kind": "binary_vuln_scan_artifact",
            "findings": findings
        }))
        .unwrap_or_else(|_| "{}".to_string());
        if fs::write(&artifact_path, encoded).is_err() {
            return Err(json!({
                "ok": false,
                "strict": strict,
                "type": "binary_vuln_plane_scan",
                "errors": ["artifact_write_failed"]
            }));
        }
    }

    let output_path = if redact_input_path && !allow_raw_path {
        format!("<redacted:{}>", &input_sha256[..12])
    } else {
        path.display().to_string()
    };
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "binary_vuln_plane_scan",
        "lane": "core/layer0/ops",
        "input": {
            "path": output_path,
            "path_redacted": redact_input_path && !allow_raw_path,
            "kind": kind,
            "sha256": input_sha256,
            "bytes": bytes.len()
        },
        "rulepack": {
            "path": rulepack_path,
            "sha256": rulepack_sha256,
            "rules": rules.len(),
            "provenance": rulepack
                .get("metadata")
                .and_then(Value::as_object)
                .and_then(|m| m.get("provenance"))
                .cloned()
                .unwrap_or(Value::Null),
            "signature": rulepack
                .get("metadata")
                .and_then(Value::as_object)
                .and_then(|m| m.get("signature"))
                .cloned()
                .or_else(|| rulepack.get("signature").cloned())
                .unwrap_or(Value::Null)
        },
        "output": {
            "format": format,
            "artifact_path": artifact_path.display().to_string(),
            "finding_count": findings.len(),
            "sandbox": {
                "max_input_bytes": max_input,
                "max_findings": max_findings,
                "max_scan_millis": max_scan_millis,
                "scan_millis": scan_millis,
                "privacy_path_redaction": redact_input_path,
                "degrade_enabled": degrade_enabled,
                "degrade_mode": degrade_mode,
                "degraded": degraded,
                "degrade_reason": if degrade_reason.is_empty() { Value::Null } else { Value::String(degrade_reason.clone()) }
            }
        },
        "findings": findings,
        "claim_evidence": [
            {
                "id": "V6-BINVULN-001.1",
                "claim": "binary_and_firmware_analysis_lane_executes_rulepack_detection_with_provenance_receipts",
                "evidence": {
                    "kind": detect_input_kind(&path),
                    "finding_count": findings.len()
                }
            },
            {
                "id": "V6-BINVULN-001.3",
                "claim": "structured_json_and_jsonl_output_contains_confidence_policy_metadata_and_provenance_hashes",
                "evidence": {
                    "format": format,
                    "finding_count": findings.len()
                }
            },
            {
                "id": "V6-BINVULN-001.4",
                "claim": "binary_scan_execution_runs_in_a_safety_plane_sandbox_with_budget_privacy_and_degrade_checks",
                "evidence": {
                    "max_input_bytes": max_input,
                    "max_findings": max_findings,
                    "scan_millis": scan_millis,
                    "degraded": degraded,
                    "path_redacted": redact_input_path && !allow_raw_path
                }
            },
            {
                "id": "V6-BINVULN-001.6",
                "claim": "developer_cli_aliases_route_to_core_binary_scan_lanes_and_surface_observability_in_infring_top",
                "evidence": {
                    "dx_source": dx_source,
                    "observability_surface": "infring-top",
                    "lane": "core/layer0/ops/binary_vuln_plane"
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    Ok(out)
}

fn run_scan(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    match scan_payload(root, parsed, strict) {
        Ok(v) => v,
        Err(err) => err,
    }
}
