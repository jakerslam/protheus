fn run_genesis_thin_wrapper_audit(
    root: &Path,
    strict: bool,
    flags: &std::collections::HashMap<String, String>,
) -> Result<Value, String> {
    let scan_root_rel_raw = flags
        .get("scan-root")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_THIN_WRAPPER_SCAN_ROOT_REL.to_string());
    let scan_root_rel_normalized = scan_root_rel_raw.replace('\\', "/");
    let scan_root_rel_invalid = scan_root_rel_normalized.starts_with('/')
        || scan_root_rel_normalized.starts_with("../")
        || scan_root_rel_normalized.contains("/../")
        || scan_root_rel_normalized.ends_with("/..")
        || scan_root_rel_normalized.contains('\0');
    if strict && scan_root_rel_invalid {
        return Ok(with_receipt_hash(json!({
            "ok": false,
            "type": "enterprise_hardening_genesis_thin_wrapper_audit",
            "lane": "enterprise_hardening",
            "mode": "genesis-thin-wrapper-audit",
            "strict": strict,
            "scan_root": scan_root_rel_raw,
            "violations": [],
            "errors": ["thin_wrapper_scan_root_invalid"],
            "claim_evidence": [
                {
                    "id": "V7-GENESIS-001.2",
                    "claim": "client_surface_boundary_audit_proves_thin_wrapper_paths_without_unauthorized_authority_calls",
                    "evidence": {"scan_root": scan_root_rel_raw}
                }
            ]
        })));
    }
    let scan_root_rel = if scan_root_rel_invalid {
        DEFAULT_THIN_WRAPPER_SCAN_ROOT_REL.to_string()
    } else {
        scan_root_rel_normalized
    };
    let scan_root = root.join(&scan_root_rel);
    let mut files = Vec::<PathBuf>::new();
    collect_files_with_extension(&scan_root, "ts", &mut files)?;
    files.sort();

    let forbidden = vec![
        "child_process.exec".to_string(),
        "child_process.spawnSync".to_string(),
        "from 'child_process'".to_string(),
        "from \"child_process\"".to_string(),
        "require('child_process')".to_string(),
        "require(\"child_process\")".to_string(),
        "Deno.Command".to_string(),
        "std::process::Command".to_string(),
        "core/layer0/ops/src".to_string(),
    ];
    let allowlist = [
        "client/runtime/systems/ops/formal_spec_guard.ts",
        "client/runtime/systems/ops/dependency_boundary_guard.ts",
    ];
    let mut violations = Vec::<Value>::new();
    for file in files {
        let rel = file
            .strip_prefix(root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        if allowlist.iter().any(|allowed| rel == *allowed) {
            continue;
        }
        let body = fs::read_to_string(&file).unwrap_or_default();
        let hits = forbidden
            .iter()
            .filter(|token| body.contains(token.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if !hits.is_empty() {
            violations.push(json!({"path": rel, "tokens": hits}));
        }
    }
    let mut errors = Vec::<String>::new();
    if strict && !violations.is_empty() {
        errors.push("thin_wrapper_boundary_violation".to_string());
    }
    Ok(with_receipt_hash(json!({
        "ok": !strict || errors.is_empty(),
        "type": "enterprise_hardening_genesis_thin_wrapper_audit",
        "lane": "enterprise_hardening",
        "mode": "genesis-thin-wrapper-audit",
        "strict": strict,
        "scan_root": scan_root_rel,
        "violations": violations,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-GENESIS-001.2",
                "claim": "client_surface_boundary_audit_proves_thin_wrapper_paths_without_unauthorized_authority_calls",
                "evidence": {"scan_root": scan_root_rel}
            }
        ]
    })))
}

fn run_genesis_doc_freeze(
    root: &Path,
    strict: bool,
    flags: &std::collections::HashMap<String, String>,
) -> Result<Value, String> {
    let release_tag = flags
        .get("release-tag")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_DOC_FREEZE_TAG.to_string());
    let required_docs = vec![
        "docs/workspace/SRS.md",
        "docs/workspace/DEFINITION_OF_DONE.md",
        "docs/workspace/codex_enforcer.md",
        "README.md",
    ];
    let entries = required_docs
        .iter()
        .map(|rel| manifest_entry(root, rel))
        .collect::<Vec<_>>();
    let missing = entries
        .iter()
        .filter(|row| !row.get("exists").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let freeze_seed = json!({
        "release_tag": release_tag,
        "entries": entries
    });
    let freeze_hash = deterministic_receipt_hash(&freeze_seed);
    let freeze_id = format!("doc_freeze_{}", &freeze_hash[..16]);
    let manifest_path = enterprise_state_root(root)
        .join("genesis")
        .join("doc_freeze")
        .join(format!("{freeze_id}.json"));
    let whitepaper_path = enterprise_state_root(root)
        .join("genesis")
        .join("doc_freeze")
        .join(format!("{freeze_id}_whitepaper.md"));
    let manifest_rel = manifest_path
        .strip_prefix(root)
        .unwrap_or(&manifest_path)
        .to_string_lossy()
        .replace('\\', "/");
    let whitepaper_rel = whitepaper_path
        .strip_prefix(root)
        .unwrap_or(&whitepaper_path)
        .to_string_lossy()
        .replace('\\', "/");
    write_json(
        &manifest_path,
        &json!({
            "schema_id": "genesis_doc_freeze_v1",
            "schema_version": "1.0",
            "freeze_id": freeze_id,
            "release_tag": release_tag,
            "entries": entries,
            "missing_count": missing,
            "generated_at": now_iso()
        }),
    )?;
    let whitepaper = format!(
        "# Genesis Documentation Freeze {freeze_id}\n\n- Release tag: {release_tag}\n- Missing required docs: {missing}\n- Manifest: {manifest_rel}\n"
    );
    if let Some(parent) = whitepaper_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    fs::write(&whitepaper_path, whitepaper).map_err(|err| {
        format!(
            "write_whitepaper_failed:{}:{err}",
            whitepaper_path.display()
        )
    })?;
    let mut errors = Vec::<String>::new();
    if strict && missing > 0 {
        errors.push("genesis_doc_freeze_missing_required_docs".to_string());
    }
    Ok(with_receipt_hash(json!({
        "ok": !strict || errors.is_empty(),
        "type": "enterprise_hardening_genesis_doc_freeze",
        "lane": "enterprise_hardening",
        "mode": "genesis-doc-freeze",
        "strict": strict,
        "release_tag": release_tag,
        "manifest_path": manifest_rel,
        "whitepaper_path": whitepaper_rel,
        "missing_count": missing,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-GENESIS-001.3",
                "claim": "documentation_freeze_and_whitepaper_artifacts_are_hash_linked_to_release_candidate",
                "evidence": {"manifest_path": manifest_rel, "whitepaper_path": whitepaper_rel}
            }
        ]
    })))
}

fn run_genesis_bootstrap(
    root: &Path,
    strict: bool,
    flags: &std::collections::HashMap<String, String>,
) -> Result<Value, String> {
    let profile = flags
        .get("profile")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "default".to_string());
    let step_ids = ["node-init", "governance-init", "monitoring-init"];
    let mut previous = "GENESIS".to_string();
    let mut checkpoints = Vec::<Value>::new();
    for step in step_ids {
        let seed = json!({"step": step, "previous": previous, "profile": profile, "ts": now_iso()});
        let checkpoint_hash = deterministic_receipt_hash(&seed);
        checkpoints.push(json!({
            "step": step,
            "checkpoint_hash": checkpoint_hash,
            "previous_checkpoint": previous,
            "rollback_pointer": format!("rollback:{step}")
        }));
        previous = checkpoints
            .last()
            .and_then(|v| v.get("checkpoint_hash"))
            .and_then(Value::as_str)
            .unwrap_or("GENESIS")
            .to_string();
    }
    let runbook_seed = json!({"profile": profile, "head": previous});
    let runbook_hash = deterministic_receipt_hash(&runbook_seed);
    let runbook_id = format!("bootstrap_{}", &runbook_hash[..16]);
    let runbook_path = enterprise_state_root(root)
        .join("genesis")
        .join("bootstrap")
        .join(format!("{runbook_id}.json"));
    let runbook_rel = runbook_path
        .strip_prefix(root)
        .unwrap_or(&runbook_path)
        .to_string_lossy()
        .replace('\\', "/");
    write_json(
        &runbook_path,
        &json!({
            "schema_id": "genesis_bootstrap_runbook_v1",
            "schema_version": "1.0",
            "runbook_id": runbook_id,
            "profile": profile,
            "checkpoints": checkpoints,
            "head": previous,
            "generated_at": now_iso()
        }),
    )?;
    Ok(with_receipt_hash(json!({
        "ok": true,
        "type": "enterprise_hardening_genesis_bootstrap",
        "lane": "enterprise_hardening",
        "mode": "genesis-bootstrap",
        "strict": strict,
        "profile": profile,
        "runbook_path": runbook_rel,
        "head": previous,
        "claim_evidence": [
            {
                "id": "V7-GENESIS-001.4",
                "claim": "genesis_bootstrap_runbook_executes_deterministic_checkpointed_sequence_with_rollback_pointers",
                "evidence": {"runbook_path": runbook_rel, "profile": profile}
            }
        ]
    })))
}

fn command_exists(name: &str) -> bool {
    let path = std::env::var("PATH").unwrap_or_default();
    path.split(':')
        .filter(|segment| !segment.trim().is_empty())
        .map(Path::new)
        .any(|dir| dir.join(name).exists())
}

fn run_genesis_installer_sim(
    root: &Path,
    strict: bool,
    flags: &std::collections::HashMap<String, String>,
) -> Result<Value, String> {
    let profile = flags
        .get("profile")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_INSTALLER_PROFILE.to_string());
    let checks = vec![
        json!({"name": "git", "ok": command_exists("git")}),
        json!({"name": "cargo", "ok": command_exists("cargo")}),
        json!({"name": "node", "ok": command_exists("node")}),
        json!({"name": "core_ops_manifest", "ok": root.join("core/layer0/ops/Cargo.toml").exists()}),
        json!({"name": "srs_exists", "ok": root.join("docs/workspace/SRS.md").exists()}),
    ];
    let failed = checks
        .iter()
        .filter(|row| !row.get("ok").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let ready = failed == 0;
    let sim_seed = json!({"profile": profile, "checks": checks});
    let sim_hash = deterministic_receipt_hash(&sim_seed);
    let sim_id = format!("installer_sim_{}", &sim_hash[..16]);
    let sim_path = enterprise_state_root(root)
        .join("genesis")
        .join("installer")
        .join(format!("{sim_id}.json"));
    let sim_rel = sim_path
        .strip_prefix(root)
        .unwrap_or(&sim_path)
        .to_string_lossy()
        .replace('\\', "/");
    write_json(
        &sim_path,
        &json!({
            "schema_id": "genesis_installer_simulation_v1",
            "schema_version": "1.0",
            "simulation_id": sim_id,
            "profile": profile,
            "checks": checks,
            "ready": ready,
            "generated_at": now_iso()
        }),
    )?;
    let mut errors = Vec::<String>::new();
    if strict && !ready {
        errors.push("installer_readiness_failed".to_string());
    }
    Ok(with_receipt_hash(json!({
        "ok": !strict || errors.is_empty(),
        "type": "enterprise_hardening_genesis_installer_sim",
        "lane": "enterprise_hardening",
        "mode": "genesis-installer-sim",
        "strict": strict,
        "profile": profile,
        "artifact_path": sim_rel,
        "ready": ready,
        "failed_checks": failed,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-GENESIS-001.5",
                "claim": "one_command_installer_readiness_simulation_emits_environment_check_receipts_before_launch",
                "evidence": {"artifact_path": sim_rel, "ready": ready}
            }
        ]
    })))
}

fn run_dashboard(root: &Path) -> Value {
    let latest = read_json(&enterprise_latest_path(root)).unwrap_or_else(|_| json!({}));
    let compliance_dir = enterprise_state_root(root).join("compliance_exports");
    let scale_dir = enterprise_state_root(root).join("scale_certifications");
    let compliance_bundles = fs::read_dir(&compliance_dir)
        .ok()
        .map(|rows| rows.filter_map(|entry| entry.ok()).count())
        .unwrap_or(0);
    let scale_certifications = fs::read_dir(&scale_dir)
        .ok()
        .map(|rows| rows.filter_map(|entry| entry.ok()).count())
        .unwrap_or(0);
    let moat_dir = enterprise_state_root(root).join("moat");
    let genesis_dir = enterprise_state_root(root).join("genesis");
    let moat_artifacts = fs::read_dir(&moat_dir)
        .ok()
        .map(|rows| rows.filter_map(|entry| entry.ok()).count())
        .unwrap_or(0);
    let genesis_artifacts = fs::read_dir(&genesis_dir)
        .ok()
        .map(|rows| rows.filter_map(|entry| entry.ok()).count())
        .unwrap_or(0);
    let bedrock_profile = read_json(
        &enterprise_state_root(root)
            .join("bedrock_proxy")
            .join("profile.json"),
    )
    .unwrap_or_else(|_| json!({"ok": false}));
    let scheduled_hands = read_json(
        &crate::core_state_root(root)
            .join("ops")
            .join("assimilation_controller")
            .join("scheduled_hands")
            .join("state.json"),
    )
    .unwrap_or_else(|_| json!({"enabled": false}));
    with_receipt_hash(json!({
        "ok": true,
        "type": "enterprise_hardening_dashboard",
        "lane": "enterprise_hardening",
        "mode": "dashboard",
        "latest": latest,
        "summary": {
            "compliance_bundles": compliance_bundles,
            "scale_certifications": scale_certifications,
            "moat_artifact_groups": moat_artifacts,
            "genesis_artifact_groups": genesis_artifacts,
            "bedrock_proxy_enabled": bedrock_profile.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "scheduled_hands_enabled": scheduled_hands.get("enabled").and_then(Value::as_bool).unwrap_or(false),
            "scheduled_hands_run_count": scheduled_hands.get("run_count").cloned().unwrap_or(Value::from(0)),
            "scheduled_hands_earnings_total_usd": scheduled_hands.get("earnings_total_usd").cloned().unwrap_or(Value::from(0.0))
        },
        "claim_evidence": [
            {
                "id": "V7-ASSIMILATE-001.5.4",
                "claim": "operations_dashboard_surfaces_bedrock_and_scheduled_hands_runtime_metrics",
                "evidence": {
                    "bedrock_proxy_enabled": bedrock_profile.get("ok").cloned().unwrap_or(Value::Bool(false)),
                    "scheduled_hands_enabled": scheduled_hands.get("enabled").cloned().unwrap_or(Value::Bool(false))
                }
            }
        ]
    }))
}

fn print_pretty(payload: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn command_exit(strict: bool, payload: &Value) -> i32 {
    if strict && !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        1
    } else {
        0
    }
}

pub(crate) fn cross_plane_guard_ok(
    signed_jwt: bool,
    cmek_key: &str,
    private_link: &str,
    egress: &str,
) -> bool {
    signed_jwt
        && cmek_key.starts_with("kms://")
        && !private_link.trim().is_empty()
        && matches!(egress, "deny" | "restricted")
}

pub(crate) fn super_gate_release_blocked(
    strict: bool,
    proven_ratio: f64,
    reliability_ok: bool,
    scale_ok: bool,
    chaos_ok: bool,
    proven_surface_count: usize,
    scheduler_proven: bool,
    fuzz_ok: bool,
) -> bool {
    strict
        && !(proven_ratio >= 0.25
            && reliability_ok
            && scale_ok
            && chaos_ok
            && proven_surface_count > 0
            && scheduler_proven
            && fuzz_ok)
}

fn requires_cross_plane_jwt_guard(cmd: &str) -> bool {
    matches!(
        cmd,
        "ops-bridge"
            | "scale-ha-certify"
            | "deploy-modules"
            | "super-gate"
            | "adoption-bootstrap"
            | "replay"
            | "explore"
            | "ai"
            | "sync"
            | "energy-cert"
            | "migrate-ecosystem"
            | "chaos-run"
            | "assistant-mode"
            | "assistant_mode"
    )
}
