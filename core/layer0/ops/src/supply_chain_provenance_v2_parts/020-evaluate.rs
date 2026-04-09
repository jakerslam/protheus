fn evaluate(root: &Path, policy: &Policy, bundle_path: &Path, vuln_summary_path: &Path) -> Value {
    let bundle = load_json(bundle_path);
    let bundle_map = bundle_artifact_map(&bundle);

    let mut artifact_rows = Vec::<Value>::new();
    let mut artifact_presence_ok = true;
    let mut sbom_presence_ok = true;
    let mut signature_presence_ok = true;
    let mut bundle_contains_required_ok = true;
    let mut hash_match_ok = true;
    let mut sbom_hash_match_ok = true;
    let mut sig_hash_match_ok = true;
    let mut signature_verified_ok = true;

    for req in &policy.required_artifacts {
        let artifact_exists = req.artifact_path.exists();
        let sbom_exists = req.sbom_path.exists();
        let signature_exists = req.signature_path.exists();

        artifact_presence_ok &= artifact_exists;
        sbom_presence_ok &= sbom_exists;
        signature_presence_ok &= signature_exists;

        let bundle_row = bundle_map.get(&req.id);
        let in_bundle = bundle_row.is_some();
        bundle_contains_required_ok &= in_bundle;

        let artifact_sha_actual = if artifact_exists {
            file_sha256(&req.artifact_path).ok()
        } else {
            None
        };
        let sbom_sha_actual = if sbom_exists {
            file_sha256(&req.sbom_path).ok()
        } else {
            None
        };
        let signature_sha_actual = if signature_exists {
            file_sha256(&req.signature_path).ok()
        } else {
            None
        };

        let mut artifact_hash_ok = true;
        let mut sbom_hash_ok = true;
        let mut signature_hash_ok = true;
        let mut signature_verified = false;

        if let Some(row) = bundle_row {
            if let Some(expected) = row.get("artifact_sha256").and_then(Value::as_str) {
                artifact_hash_ok = artifact_sha_actual
                    .as_deref()
                    .map(|actual| actual.eq_ignore_ascii_case(expected))
                    .unwrap_or(false);
            }
            if let Some(expected) = row.get("sbom_sha256").and_then(Value::as_str) {
                sbom_hash_ok = sbom_sha_actual
                    .as_deref()
                    .map(|actual| actual.eq_ignore_ascii_case(expected))
                    .unwrap_or(false);
            }
            if let Some(expected) = row.get("signature_sha256").and_then(Value::as_str) {
                signature_hash_ok = signature_sha_actual
                    .as_deref()
                    .map(|actual| actual.eq_ignore_ascii_case(expected))
                    .unwrap_or(false);
            }
            signature_verified = row
                .get("signature_verified")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            if let Some(bundle_artifact_path) = row.get("artifact_path").and_then(Value::as_str) {
                artifact_hash_ok &= bundle_artifact_path.replace('\\', "/")
                    == normalize_rel(root, &req.artifact_path);
            }
            if let Some(bundle_sbom_path) = row.get("sbom_path").and_then(Value::as_str) {
                sbom_hash_ok &=
                    bundle_sbom_path.replace('\\', "/") == normalize_rel(root, &req.sbom_path);
            }
            if let Some(bundle_signature_path) = row.get("signature_path").and_then(Value::as_str) {
                signature_hash_ok &= bundle_signature_path.replace('\\', "/")
                    == normalize_rel(root, &req.signature_path);
            }
        }

        hash_match_ok &= artifact_hash_ok;
        sbom_hash_match_ok &= sbom_hash_ok;
        sig_hash_match_ok &= signature_hash_ok;
        signature_verified_ok &= signature_verified;

        artifact_rows.push(json!({
            "id": req.id,
            "artifact_exists": artifact_exists,
            "sbom_exists": sbom_exists,
            "signature_exists": signature_exists,
            "in_provenance_bundle": in_bundle,
            "artifact_sha256": artifact_sha_actual,
            "sbom_sha256": sbom_sha_actual,
            "signature_sha256": signature_sha_actual,
            "artifact_hash_ok": artifact_hash_ok,
            "sbom_hash_ok": sbom_hash_ok,
            "signature_hash_ok": signature_hash_ok,
            "signature_verified": signature_verified,
            "artifact_path": normalize_rel(root, &req.artifact_path),
            "sbom_path": normalize_rel(root, &req.sbom_path),
            "signature_path": normalize_rel(root, &req.signature_path)
        }));
    }

    let vuln_summary = load_json(vuln_summary_path);
    let (critical, high, medium) = read_counts(&vuln_summary);
    let age_hours = report_age_hours(&vuln_summary);
    let vulnerability_ok = critical <= policy.vulnerability_sla.max_critical
        && high <= policy.vulnerability_sla.max_high
        && medium <= policy.vulnerability_sla.max_medium
        && age_hours
            .map(|age| age <= policy.vulnerability_sla.max_report_age_hours)
            .unwrap_or(false);

    let rollback_policy_exists = policy.rollback_policy_path.exists();
    let rollback_last_known_good_tag = bundle
        .get("rollback")
        .and_then(|v| v.get("last_known_good_tag"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let rollback_contract_ok = rollback_policy_exists && !rollback_last_known_good_tag.is_empty();

    let provenance_bundle_ok = bundle_path.exists()
        && bundle
            .get("tag")
            .and_then(Value::as_str)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
        && bundle
            .get("generated_at")
            .and_then(Value::as_str)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
        && bundle_map.len() >= policy.required_artifacts.len();

    let mut checks = BTreeMap::<String, Value>::new();
    checks.insert(
        "artifact_presence".to_string(),
        json!({
            "ok": artifact_presence_ok,
            "required": policy.required_artifacts.len(),
            "details": artifact_rows
        }),
    );
    checks.insert(
        "sbom_presence".to_string(),
        json!({
            "ok": sbom_presence_ok,
            "required": policy.required_artifacts.len()
        }),
    );
    checks.insert(
        "signature_presence".to_string(),
        json!({
            "ok": signature_presence_ok,
            "required": policy.required_artifacts.len()
        }),
    );
    checks.insert(
        "provenance_bundle_contract".to_string(),
        json!({
            "ok": provenance_bundle_ok,
            "bundle_path": bundle_path,
            "bundle_artifact_count": bundle_map.len()
        }),
    );
    checks.insert(
        "bundle_contains_required_artifacts".to_string(),
        json!({
            "ok": bundle_contains_required_ok,
            "required_ids": policy.required_artifacts.iter().map(|a| a.id.clone()).collect::<Vec<_>>()
        }),
    );
    checks.insert(
        "artifact_hashes_match_bundle".to_string(),
        json!({
            "ok": hash_match_ok
        }),
    );
    checks.insert(
        "sbom_hashes_match_bundle".to_string(),
        json!({
            "ok": sbom_hash_match_ok
        }),
    );
    checks.insert(
        "signature_hashes_match_bundle".to_string(),
        json!({
            "ok": sig_hash_match_ok
        }),
    );
    checks.insert(
        "signature_verification_status".to_string(),
        json!({
            "ok": signature_verified_ok
        }),
    );
    checks.insert(
        "dependency_vulnerability_sla".to_string(),
        json!({
            "ok": vulnerability_ok,
            "counts": {
                "critical": critical,
                "high": high,
                "medium": medium
            },
            "max": {
                "critical": policy.vulnerability_sla.max_critical,
                "high": policy.vulnerability_sla.max_high,
                "medium": policy.vulnerability_sla.max_medium
            },
            "report_age_hours": age_hours,
            "max_report_age_hours": policy.vulnerability_sla.max_report_age_hours,
            "summary_path": vuln_summary_path
        }),
    );
    checks.insert(
        "rollback_to_last_known_good_policy".to_string(),
        json!({
            "ok": rollback_contract_ok,
            "rollback_policy_path": policy.rollback_policy_path,
            "last_known_good_tag": rollback_last_known_good_tag
        }),
    );

    let blocking_checks = checks
        .iter()
        .filter_map(|(k, v)| {
            if v.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                None
            } else {
                Some(k.clone())
            }
        })
        .collect::<Vec<_>>();

    let ok = blocking_checks.is_empty();

    json!({
        "ok": ok,
        "type": "supply_chain_provenance_v2_run",
        "lane": LANE_ID,
        "schema_id": "supply_chain_provenance_v2",
        "schema_version": "1.0",
        "ts": now_iso(),
        "checks": checks,
        "blocking_checks": blocking_checks,
        "inputs": {
            "bundle_path": bundle_path,
            "vulnerability_summary_path": vuln_summary_path,
            "required_artifact_count": policy.required_artifacts.len()
        },
        "claim_evidence": [
            {
                "id": "release_artifacts_signed_and_verified",
                "claim": "release_artifacts_have_sbom_signature_and_hash_parity_before_deploy",
                "evidence": {
                    "artifact_presence_ok": artifact_presence_ok,
                    "sbom_presence_ok": sbom_presence_ok,
                    "signature_presence_ok": signature_presence_ok,
                    "bundle_contains_required_ok": bundle_contains_required_ok,
                    "signature_verified_ok": signature_verified_ok
                }
            },
            {
                "id": "dependency_vulnerability_sla_gate",
                "claim": "dependency_vulnerability_sla_is_fail_closed_before_release_promotion",
                "evidence": {
                    "critical": critical,
                    "high": high,
                    "medium": medium,
                    "max_critical": policy.vulnerability_sla.max_critical,
                    "max_high": policy.vulnerability_sla.max_high,
                    "max_medium": policy.vulnerability_sla.max_medium,
                    "rollback_contract_ok": rollback_contract_ok
                }
            }
        ]
    })
}

fn run_cmd(
    root: &Path,
    policy: &Policy,
    strict: bool,
    bundle_path: &Path,
    vuln_summary_path: &Path,
) -> Result<(Value, i32), String> {
    let mut payload = evaluate(root, policy, bundle_path, vuln_summary_path);
    payload["strict"] = Value::Bool(strict);
    payload["policy_path"] = Value::String(policy.policy_path.to_string_lossy().to_string());
    payload["receipt_hash"] = Value::String(deterministic_receipt_hash(&payload));

    write_text_atomic(
        &policy.latest_path,
        &format!(
            "{}\n",
            serde_json::to_string_pretty(&payload)
                .map_err(|e| format!("encode_latest_failed:{e}"))?
        ),
    )?;
    append_jsonl(&policy.history_path, &payload)?;

    let code = if strict && !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        1
    } else {
        0
    };

    Ok((payload, code))
}

fn default_release_tag(root: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("rev-parse")
        .arg("--short=12")
        .arg("HEAD")
        .output();
    if let Ok(output) = output {
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !value.is_empty() {
                return format!("local-{value}");
            }
        }
    }
    format!("local-{}", now_iso().replace([':', '.'], "-"))
}

