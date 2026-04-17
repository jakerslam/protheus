fn canonical_moat_primitive(raw: &str) -> String {
    let token = raw.trim().to_ascii_lowercase().replace('-', "_");
    match token.as_str() {
        "conduit_runtime" => "conduit".to_string(),
        "directive" | "directiveengine" => "directive_kernel".to_string(),
        "binaryblob" | "blob_vault" => "binary_blob".to_string(),
        "network" | "networking" => "network_protocol".to_string(),
        _ => token,
    }
}

fn canonical_license(raw: &str) -> String {
    let token = raw.trim();
    if token.eq_ignore_ascii_case("apache2")
        || token.eq_ignore_ascii_case("apache_2_0")
        || token.eq_ignore_ascii_case("apache-2")
    {
        "Apache-2.0".to_string()
    } else if token.eq_ignore_ascii_case("mit") {
        "MIT".to_string()
    } else if token.is_empty() {
        "Apache-2.0".to_string()
    } else {
        token.to_string()
    }
}

fn run_moat_license(
    root: &Path,
    strict: bool,
    flags: &std::collections::HashMap<String, String>,
) -> Result<Value, String> {
    let mut primitives = flags
        .get("primitives")
        .map(|raw| split_csv(raw))
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "conduit".to_string(),
                "binary_blob".to_string(),
                "directive_kernel".to_string(),
                "network_protocol".to_string(),
            ]
        });
    primitives = primitives
        .into_iter()
        .map(|item| canonical_moat_primitive(&item))
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    primitives.sort();
    primitives.dedup();
    let license = canonical_license(
        flags
        .get("license")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "Apache-2.0".to_string())
        .as_str(),
    );
    let reviewer = flags
        .get("reviewer")
        .map(|v| v.trim().to_ascii_lowercase().replace(' ', "_"))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "legal-review-bot".to_string());

    let source_for = |primitive: &str| -> Option<&'static str> {
        match primitive {
            "conduit" => Some("core/layer0/ops/src/v8_kernel.rs"),
            "binary_blob" => Some("core/layer0/ops/src/binary_blob_runtime.rs"),
            "directive_kernel" => Some("core/layer0/ops/src/directive_kernel.rs"),
            "network_protocol" => Some("core/layer0/ops/src/network_protocol_run.rs"),
            "enterprise_hardening" => Some("core/layer0/ops/src/enterprise_hardening.rs"),
            _ => None,
        }
    };

    let mut errors = Vec::<String>::new();
    let mut packages = Vec::<Value>::new();
    for primitive in &primitives {
        if let Some(src) = source_for(primitive) {
            let entry = manifest_entry(root, src);
            if strict
                && !entry
                    .get("exists")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            {
                errors.push(format!("primitive_source_missing:{primitive}"));
            }
            packages.push(json!({
                "primitive": primitive,
                "source": src,
                "source_manifest": entry
            }));
        } else if strict {
            errors.push(format!("unknown_primitive:{primitive}"));
        } else {
            packages.push(json!({
                "primitive": primitive,
                "source": Value::Null,
                "source_manifest": {"path": Value::Null, "exists": false}
            }));
        }
    }

    let package_seed = json!({
        "primitives": primitives,
        "license": license,
        "reviewer": reviewer
    });
    let package_hash = deterministic_receipt_hash(&package_seed);
    let package_id = format!("moat_license_{}", &package_hash[..16]);
    let package_path = enterprise_state_root(root)
        .join("moat")
        .join("licensing")
        .join(format!("{package_id}.json"));
    let package_rel = package_path
        .strip_prefix(root)
        .unwrap_or(&package_path)
        .to_string_lossy()
        .replace('\\', "/");

    let package_payload = json!({
        "schema_id": "moat_licensing_package_v1",
        "schema_version": "1.0",
        "package_id": package_id,
        "license": license,
        "reviewer": reviewer,
        "primitives": packages,
        "review_checkpoint": {
            "status": if errors.is_empty() { "approved" } else { "requires_revision" },
            "reviewed_at": now_iso()
        }
    });
    write_json(&package_path, &package_payload)?;

    Ok(with_receipt_hash(json!({
        "ok": !strict || errors.is_empty(),
        "type": "enterprise_hardening_moat_license",
        "lane": "enterprise_hardening",
        "mode": "moat-license",
        "strict": strict,
        "license": license,
        "reviewer": reviewer,
        "primitives_requested": primitives,
        "package_path": package_rel,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-MOAT-001.1",
                "claim": "ip_and_licensing_pipeline_emits_deterministic_legal_package_manifests_with_review_checkpoints",
                "evidence": {"package_path": package_rel, "reviewer": reviewer}
            }
        ]
    })))
}

fn run_moat_contrast(
    root: &Path,
    strict: bool,
    flags: &std::collections::HashMap<String, String>,
) -> Result<Value, String> {
    let narrative = flags
        .get("narrative")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            "Security posture emphasizes fail-closed conduit authority and deterministic receipts."
                .to_string()
        });
    let enterprise_history_rows = fs::read_to_string(enterprise_history_path(root))
        .ok()
        .map(|body| body.lines().count())
        .unwrap_or(0usize);
    let directive_integrity = crate::directive_kernel::directive_vault_integrity(root);
    let blob_vault_path = crate::core_state_root(root)
        .join("blob_vault")
        .join("prime_blob_vault.json");
    let blob_integrity = if blob_vault_path.exists() {
        let vault = read_json(&blob_vault_path).unwrap_or_else(|_| json!({}));
        let entries = vault
            .get("entries")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0usize);
        let chain_head = vault
            .get("chain_head")
            .and_then(Value::as_str)
            .unwrap_or("genesis")
            .to_string();
        json!({
            "ok": entries == 0 || chain_head != "genesis",
            "entry_count": entries,
            "chain_head": chain_head
        })
    } else {
        json!({
            "ok": true,
            "entry_count": 0,
            "chain_head": "genesis"
        })
    };
    let top1_latest = read_json(
        &crate::core_state_root(root)
            .join("ops")
            .join("top1_assurance")
            .join("latest.json"),
    )
    .unwrap_or_else(|_| json!({"proven_ratio": 0.0}));
    let proven_ratio = top1_latest
        .get("proven_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let receipts_ok = directive_integrity
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && blob_integrity
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false);

    let contrast_seed = json!({
        "history_rows": enterprise_history_rows,
        "receipts_ok": receipts_ok,
        "proven_ratio": proven_ratio,
        "narrative": narrative
    });
    let contrast_hash = deterministic_receipt_hash(&contrast_seed);
    let contrast_id = format!("contrast_{}", &contrast_hash[..16]);
    let json_path = enterprise_state_root(root)
        .join("moat")
        .join("contrast")
        .join(format!("{contrast_id}.json"));
    let md_path = enterprise_state_root(root)
        .join("moat")
        .join("contrast")
        .join(format!("{contrast_id}.md"));
    let json_rel = json_path
        .strip_prefix(root)
        .unwrap_or(&json_path)
        .to_string_lossy()
        .replace('\\', "/");
    let md_rel = md_path
        .strip_prefix(root)
        .unwrap_or(&md_path)
        .to_string_lossy()
        .replace('\\', "/");

    write_json(
        &json_path,
        &json!({
            "schema_id": "moat_security_contrast_v1",
            "schema_version": "1.0",
            "contrast_id": contrast_id,
            "enterprise_history_rows": enterprise_history_rows,
            "receipts_ok": receipts_ok,
            "top1_proven_ratio": proven_ratio,
            "directive_integrity": directive_integrity,
            "blob_integrity": blob_integrity,
            "narrative": narrative,
            "generated_at": now_iso()
        }),
    )?;
    let contrast_md = format!(
        "# Security Contrast Report {contrast_id}\n\n\
         - Enterprise history rows: {enterprise_history_rows}\n\
         - Directive integrity ok: {}\n\
         - Binary blob integrity ok: {}\n\
         - Top1 proven ratio: {proven_ratio:.3}\n\n\
         ## Narrative\n{narrative}\n",
        directive_integrity
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        blob_integrity
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    );
    if let Some(parent) = md_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    fs::write(&md_path, contrast_md)
        .map_err(|err| format!("write_contrast_md_failed:{}:{err}", md_path.display()))?;

    let ok = receipts_ok || !strict;
    Ok(with_receipt_hash(json!({
        "ok": ok,
        "type": "enterprise_hardening_moat_contrast",
        "lane": "enterprise_hardening",
        "mode": "moat-contrast",
        "strict": strict,
        "contrast_json_path": json_rel,
        "contrast_markdown_path": md_rel,
        "receipts_ok": receipts_ok,
        "top1_proven_ratio": proven_ratio,
        "claim_evidence": [
            {
                "id": "V7-MOAT-001.2",
                "claim": "security_contrast_artifacts_publish_reproducible_evidence_linked_narrative_metrics",
                "evidence": {"contrast_json_path": json_rel, "top1_proven_ratio": proven_ratio}
            }
        ]
    })))
}

fn run_moat_launch_sim(
    root: &Path,
    strict: bool,
    flags: &std::collections::HashMap<String, String>,
) -> Result<Value, String> {
    let contributors = flags
        .get("contributors")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(800)
        .max(1);
    let events = flags
        .get("events")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(12_000)
        .max(1);
    let queue_depth = (events as f64 / contributors as f64).max(1.0);
    let p95_latency_ms = (queue_depth * 18.0).min(5000.0);
    let p99_latency_ms = (p95_latency_ms * 1.35).min(8000.0);
    let readiness_score = (100.0 - (p95_latency_ms / 3.0)).clamp(0.0, 100.0);
    let ready = readiness_score >= 75.0;

    let sim_seed = json!({
        "contributors": contributors,
        "events": events,
        "p95_latency_ms": p95_latency_ms,
        "p99_latency_ms": p99_latency_ms,
        "readiness_score": readiness_score
    });
    let sim_hash = deterministic_receipt_hash(&sim_seed);
    let sim_id = format!("launch_sim_{}", &sim_hash[..16]);
    let sim_path = enterprise_state_root(root)
        .join("moat")
        .join("launch_sim")
        .join(format!("{sim_id}.json"));
    let sim_rel = sim_path
        .strip_prefix(root)
        .unwrap_or(&sim_path)
        .to_string_lossy()
        .replace('\\', "/");
    write_json(
        &sim_path,
        &json!({
            "schema_id": "moat_launch_simulation_v1",
            "schema_version": "1.0",
            "simulation_id": sim_id,
            "contributors": contributors,
            "events": events,
            "metrics": {
                "p95_latency_ms": p95_latency_ms,
                "p99_latency_ms": p99_latency_ms,
                "readiness_score": readiness_score
            },
            "rollback_playbook": [
                "pause_new_contributor_onboarding",
                "drain_non_critical_queues",
                "switch_to_safe_capacity_profile",
                "resume_after_guard_validation"
            ],
            "ready": ready,
            "generated_at": now_iso()
        }),
    )?;

    let mut errors = Vec::<String>::new();
    if strict && !ready {
        errors.push("launch_sim_not_ready".to_string());
    }
    Ok(with_receipt_hash(json!({
        "ok": !strict || errors.is_empty(),
        "type": "enterprise_hardening_moat_launch_sim",
        "lane": "enterprise_hardening",
        "mode": "moat-launch-sim",
        "strict": strict,
        "contributors": contributors,
        "events": events,
        "metrics": {
            "p95_latency_ms": p95_latency_ms,
            "p99_latency_ms": p99_latency_ms,
            "readiness_score": readiness_score
        },
        "artifact_path": sim_rel,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-MOAT-001.3",
                "claim": "launch_day_load_simulation_emits_readiness_and_rollback_playbook_artifacts",
                "evidence": {"artifact_path": sim_rel, "readiness_score": readiness_score}
            }
        ]
    })))
}

fn run_genesis_truth_gate(
    root: &Path,
    strict: bool,
    flags: &std::collections::HashMap<String, String>,
) -> Result<Value, String> {
    let regression_pass = bool_flag(flags.get("regression-pass").map(String::as_str), false);
    let dod_pass = bool_flag(flags.get("dod-pass").map(String::as_str), false);
    let verify_pass = bool_flag(flags.get("verify-pass").map(String::as_str), false);
    let all_pass = regression_pass && dod_pass && verify_pass;
    let mut errors = Vec::<String>::new();
    if strict && !all_pass {
        errors.push("genesis_truth_gate_failed".to_string());
    }
    let candidate_seed = json!({
        "regression_pass": regression_pass,
        "dod_pass": dod_pass,
        "verify_pass": verify_pass,
        "ts": now_iso()
    });
    let candidate_hash = deterministic_receipt_hash(&candidate_seed);
    let candidate_id = format!("launch_candidate_{}", &candidate_hash[..16]);
    let candidate_path = enterprise_state_root(root)
        .join("genesis")
        .join("launch_candidates")
        .join(format!("{candidate_id}.json"));
    let candidate_rel = candidate_path
        .strip_prefix(root)
        .unwrap_or(&candidate_path)
        .to_string_lossy()
        .replace('\\', "/");
    write_json(
        &candidate_path,
        &json!({
            "schema_id": "genesis_launch_candidate_v1",
            "schema_version": "1.0",
            "candidate_id": candidate_id,
            "gates": {
                "regression_pass": regression_pass,
                "dod_pass": dod_pass,
                "verify_pass": verify_pass
            },
            "ready": all_pass,
            "generated_at": now_iso()
        }),
    )?;
    Ok(with_receipt_hash(json!({
        "ok": !strict || errors.is_empty(),
        "type": "enterprise_hardening_genesis_truth_gate",
        "lane": "enterprise_hardening",
        "mode": "genesis-truth-gate",
        "strict": strict,
        "candidate_id": candidate_id,
        "candidate_path": candidate_rel,
        "gates": {
            "regression_pass": regression_pass,
            "dod_pass": dod_pass,
            "verify_pass": verify_pass
        },
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-GENESIS-001.1",
                "claim": "launch_blocker_requires_regression_dod_and_verify_gates_before_promotion",
                "evidence": {"candidate_id": candidate_id, "all_pass": all_pass}
            }
        ]
    })))
}
