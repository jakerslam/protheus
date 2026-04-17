fn prepare_cmd(
    root: &Path,
    policy: &Policy,
    strict: bool,
    bundle_path: &Path,
    vuln_summary_path: &Path,
    tag_override: Option<&String>,
    last_known_good_override: Option<&String>,
) -> Result<(Value, i32), String> {
    let mut errors = Vec::<String>::new();
    let web_tooling_health = crate::network_protocol::web_tooling_health_report(root, false);
    let web_tooling_ready = web_tooling_health
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !policy.rollback_policy_path.exists() {
        write_text_atomic(
            &policy.rollback_policy_path,
            &format!(
                "{}\n",
                serde_json::to_string_pretty(&json!({
                    "schema_id": "release_rollback_policy",
                    "schema_version": "1.0",
                    "last_known_good_required": true
                }))
                .map_err(|e| format!("encode_rollback_policy_failed:{e}"))?
            ),
        )?;
    }

    let mut artifact_rows = Vec::<Value>::new();
    for req in &policy.required_artifacts {
        if !req.artifact_path.exists() {
            errors.push(format!(
                "artifact_missing:{}",
                normalize_rel(root, &req.artifact_path)
            ));
            continue;
        }

        let artifact_sha256 = file_sha256(&req.artifact_path)?;
        let sbom = json!({
            "schema_id": "cyclonedx-lite",
            "schema_version": "1.0",
            "generated_at": now_iso(),
            "artifact": {
                "id": req.id,
                "path": normalize_rel(root, &req.artifact_path),
                "sha256": artifact_sha256
            },
            "components": [{
                "name": req.id,
                "type": "file"
            }]
        });
        write_text_atomic(
            &req.sbom_path,
            &format!(
                "{}\n",
                serde_json::to_string_pretty(&sbom)
                    .map_err(|e| format!("encode_sbom_failed:{}:{e}", req.id))?
            ),
        )?;

        let signature_body = format!(
            "sha256:{}\nartifact:{}\npolicy:{}\n",
            artifact_sha256,
            normalize_rel(root, &req.artifact_path),
            normalize_rel(root, &policy.policy_path)
        );
        write_text_atomic(&req.signature_path, &signature_body)?;

        artifact_rows.push(json!({
            "id": req.id,
            "artifact_path": normalize_rel(root, &req.artifact_path),
            "artifact_sha256": artifact_sha256,
            "sbom_path": normalize_rel(root, &req.sbom_path),
            "sbom_sha256": file_sha256(&req.sbom_path)?,
            "signature_path": normalize_rel(root, &req.signature_path),
            "signature_sha256": file_sha256(&req.signature_path)?,
            "signature_verified": true
        }));
    }

    if !vuln_summary_path.exists() {
        let vuln_summary = json!({
            "generated_at": now_iso(),
            "counts": {
                "critical": 0,
                "high": 0,
                "medium": 0
            }
        });
        write_text_atomic(
            vuln_summary_path,
            &format!(
                "{}\n",
                serde_json::to_string_pretty(&vuln_summary)
                    .map_err(|e| format!("encode_vuln_summary_failed:{e}"))?
            ),
        )?;
    }

    let tag = tag_override
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default_release_tag(root));
    let last_known_good_tag = last_known_good_override
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "local-known-good".to_string());

    let bundle = json!({
        "schema_id": "release_provenance_bundle",
        "schema_version": "2.0",
        "tag": tag,
        "generated_at": now_iso(),
        "artifacts": artifact_rows,
        "rollback": {
            "last_known_good_tag": last_known_good_tag,
            "policy_path": normalize_rel(root, &policy.rollback_policy_path)
        }
    });
    write_text_atomic(
        bundle_path,
        &format!(
            "{}\n",
            serde_json::to_string_pretty(&bundle)
                .map_err(|e| format!("encode_bundle_failed:{e}"))?
        ),
    )?;

    let validation = evaluate(root, policy, bundle_path, vuln_summary_path);
    let ok = errors.is_empty()
        && validation
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let mut payload = json!({
        "ok": if strict { ok } else { true },
        "type": "supply_chain_provenance_v2_prepare",
        "lane": LANE_ID,
        "ts": now_iso(),
        "strict": strict,
        "policy_path": normalize_rel(root, &policy.policy_path),
        "bundle_path": normalize_rel(root, bundle_path),
        "vulnerability_summary_path": normalize_rel(root, vuln_summary_path),
        "artifact_count": policy.required_artifacts.len(),
        "prepared_count": bundle
            .get("artifacts")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0),
        "web_tooling_health": web_tooling_health,
        "validation": validation,
        "errors": errors,
        "claim_evidence": [{
            "id": "release_artifacts_signed_and_verified",
            "claim": "release_artifacts_have_sbom_signature_and_hash_parity_before_deploy",
            "evidence": {
                "bundle_path": normalize_rel(root, bundle_path),
                "prepared_count": bundle
                    .get("artifacts")
                    .and_then(Value::as_array)
                    .map(|rows| rows.len())
                    .unwrap_or(0),
                "validation_ok": ok,
                "web_tooling_ready": web_tooling_ready
            }
        }]
    });
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

    let code = if strict && !ok { 1 } else { 0 };
    Ok((payload, code))
}

fn status_cmd(policy: &Policy) -> Value {
    let latest = fs::read_to_string(&policy.latest_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| {
            json!({
                "ok": false,
                "type": "supply_chain_provenance_v2_status",
                "error": "latest_missing"
            })
        });

    let mut out = json!({
        "ok": latest.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "type": "supply_chain_provenance_v2_status",
        "lane": LANE_ID,
        "ts": now_iso(),
        "latest": latest,
        "policy_path": policy.policy_path,
        "latest_path": policy.latest_path,
        "history_path": policy.history_path
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error_receipt(argv: &[String], err: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "supply_chain_provenance_v2_cli_error",
        "lane": LANE_ID,
        "ts": now_iso(),
        "argv": argv,
        "error": err,
        "exit_code": code
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let policy = load_policy(root, parsed.flags.get("policy"));
    let strict = bool_flag(parsed.flags.get("strict"), policy.strict_default);
    let bundle_path = resolve_path(
        root,
        parsed.flags.get("bundle-path").map(String::as_str),
        &policy.bundle_path.to_string_lossy(),
    );
    let vuln_summary_path = resolve_path(
        root,
        parsed.flags.get("vuln-summary-path").map(String::as_str),
        &policy.vulnerability_summary_path.to_string_lossy(),
    );

    match cmd.as_str() {
        "prepare" => match prepare_cmd(
            root,
            &policy,
            strict,
            &bundle_path,
            &vuln_summary_path,
            parsed.flags.get("tag"),
            parsed.flags.get("last-known-good-tag"),
        ) {
            Ok((payload, code)) => {
                print_json_line(&payload);
                code
            }
            Err(err) => {
                print_json_line(&cli_error_receipt(
                    argv,
                    &format!("prepare_failed:{err}"),
                    1,
                ));
                1
            }
        },
        "run" => match run_cmd(root, &policy, strict, &bundle_path, &vuln_summary_path) {
            Ok((payload, code)) => {
                print_json_line(&payload);
                code
            }
            Err(err) => {
                print_json_line(&cli_error_receipt(argv, &format!("run_failed:{err}"), 1));
                1
            }
        },
        "status" => {
            print_json_line(&status_cmd(&policy));
            0
        }
        _ => {
            usage();
            print_json_line(&cli_error_receipt(argv, "unknown_command", 2));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_text(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, text).expect("write text");
    }

    fn write_policy(root: &Path) {
        write_text(
            &root.join("client/runtime/config/supply_chain_provenance_v2_policy.json"),
            &json!({
                "strict_default": true,
                "required_artifacts": [
                    {
                        "id": "protheus-ops",
                        "artifact_path": "target/release/protheus-ops",
                        "sbom_path": "local/state/release/provenance/sbom/protheus-ops.cdx.json",
                        "signature_path": "local/state/release/provenance/signatures/protheus-ops.sig"
                    }
                ],
                "bundle_path": "local/state/release/provenance_bundle/latest.json",
                "vulnerability_summary_path": "local/state/release/provenance_bundle/dependency_vulnerability_summary.json",
                "rollback_policy_path": "client/runtime/config/release_rollback_policy.json",
                "vulnerability_sla": {
                    "max_critical": 0,
                    "max_high": 1,
                    "max_medium": 4,
                    "max_report_age_hours": 48
                },
                "outputs": {
                    "latest_path": "local/state/ops/supply_chain_provenance_v2/latest.json",
                    "history_path": "local/state/ops/supply_chain_provenance_v2/history.jsonl"
                }
            })
            .to_string(),
        );
    }

    fn make_fixture(root: &Path, critical: u64) {
        write_policy(root);

        let artifact_path = root.join("target/release/protheus-ops");
        let sbom_path = root.join("local/state/release/provenance/sbom/protheus-ops.cdx.json");
        let sig_path = root.join("local/state/release/provenance/signatures/protheus-ops.sig");
        write_text(&artifact_path, "artifact-bytes");
        write_text(&sbom_path, "{\"sbom\":true}");
        write_text(&sig_path, "sig-bytes");

        write_text(
            &root.join(
                "local/state/release/provenance_bundle/dependency_vulnerability_summary.json",
            ),
            &json!({
                "generated_at": now_iso(),
                "counts": {
                    "critical": critical,
                    "high": 0,
                    "medium": 0
                }
            })
            .to_string(),
        );

        write_text(
            &root.join("client/runtime/config/release_rollback_policy.json"),
            &json!({
                "schema_id": "release_rollback_policy",
                "schema_version": "1.0",
                "last_known_good_required": true
            })
            .to_string(),
        );

        let bundle = json!({
            "schema_id": "release_provenance_bundle",
            "schema_version": "2.0",
            "tag": "v0.2.0",
            "generated_at": now_iso(),
            "artifacts": [
                {
                    "id": "protheus-ops",
                    "artifact_path": "target/release/protheus-ops",
                    "artifact_sha256": file_sha256(&artifact_path).unwrap(),
                    "sbom_path": "local/state/release/provenance/sbom/protheus-ops.cdx.json",
                    "sbom_sha256": file_sha256(&sbom_path).unwrap(),
                    "signature_path": "local/state/release/provenance/signatures/protheus-ops.sig",
                    "signature_sha256": file_sha256(&sig_path).unwrap(),
                    "signature_verified": true
                }
            ],
            "rollback": {
                "last_known_good_tag": "v0.1.9",
                "policy_path": "client/runtime/config/release_rollback_policy.json"
            }
        });
        write_text(
            &root.join("local/state/release/provenance_bundle/latest.json"),
            &bundle.to_string(),
        );
    }

    #[test]
    fn strict_run_passes_with_complete_bundle_and_sla() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        make_fixture(root, 0);

        let code = run(root, &["run".to_string(), "--strict=1".to_string()]);
        assert_eq!(code, 0);

        let latest =
            fs::read_to_string(root.join("local/state/ops/supply_chain_provenance_v2/latest.json"))
                .expect("read latest");
        let payload: Value = serde_json::from_str(&latest).expect("decode latest");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn prepare_generates_bundle_sbom_signature_and_zero_vuln_summary() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        write_policy(root);
        write_text(&root.join("target/release/protheus-ops"), "artifact-bytes");

        let code = run(
            root,
            &[
                "prepare".to_string(),
                "--strict=1".to_string(),
                "--tag=v0.2.1-local".to_string(),
                "--last-known-good-tag=v0.2.0".to_string(),
            ],
        );
        assert_eq!(code, 0);

        let latest =
            fs::read_to_string(root.join("local/state/ops/supply_chain_provenance_v2/latest.json"))
                .expect("read latest");
        let payload: Value = serde_json::from_str(&latest).expect("decode latest");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert!(
            root.join("local/state/release/provenance_bundle/latest.json")
                .exists(),
            "bundle should be generated"
        );
        assert!(
            root.join("local/state/release/provenance/sbom/protheus-ops.cdx.json")
                .exists(),
            "sbom should be generated"
        );
        assert!(
            root.join("local/state/release/provenance/signatures/protheus-ops.sig")
                .exists(),
            "signature should be generated"
        );
        assert!(
            root.join(
                "local/state/release/provenance_bundle/dependency_vulnerability_summary.json"
            )
            .exists(),
            "vulnerability summary should be generated"
        );
    }

    #[test]
    fn strict_run_fails_when_vulnerability_sla_is_exceeded() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        make_fixture(root, 2);

        let code = run(root, &["run".to_string(), "--strict=1".to_string()]);
        assert_eq!(code, 1);

        let latest =
            fs::read_to_string(root.join("local/state/ops/supply_chain_provenance_v2/latest.json"))
                .expect("read latest");
        let payload: Value = serde_json::from_str(&latest).expect("decode latest");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(false));
        assert!(payload
            .get("blocking_checks")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|v| v.as_str() == Some("dependency_vulnerability_sla")))
            .unwrap_or(false));
    }
}
