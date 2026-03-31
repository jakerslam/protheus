fn cross_plane_guard_profile(root: &Path) -> Result<Value, String> {
    let profile_path = enterprise_state_root(root).join("f100/zero_trust_profile.json");
    let profile = read_json(&profile_path)
        .map_err(|_| format!("zero_trust_profile_missing:{}", profile_path.display()))?;
    let signed_jwt = profile
        .get("signed_jwt")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let egress = profile
        .get("egress")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let cmek_key = profile
        .get("cmek_key")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let private_link = profile
        .get("private_link")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let guard_ok = cross_plane_guard_ok(signed_jwt, &cmek_key, &private_link, &egress);
    Ok(json!({
        "profile_path": profile_path.display().to_string(),
        "signed_jwt": signed_jwt,
        "cmek_key": cmek_key,
        "private_link": private_link,
        "egress": egress,
        "guard_ok": guard_ok
    }))
}

fn append_claim_evidence(payload: &mut Value, row: Value) {
    let mut rows = payload
        .get("claim_evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    rows.push(row);
    payload["claim_evidence"] = Value::Array(rows);
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv
        .iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h" | "help"))
    {
        usage();
        return 0;
    }

    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "run".to_string());

    let strict_default = cmd == "run";
    let strict = bool_flag(
        parsed.flags.get("strict").map(String::as_str),
        strict_default,
    );
    let policy_path = parsed
        .flags
        .get("policy")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_POLICY_REL.to_string());
    let cross_plane_guard = if requires_cross_plane_jwt_guard(&cmd) {
        Some(cross_plane_guard_profile(root))
    } else {
        None
    };

    let result = match cmd.as_str() {
        "run" | "status" => {
            run_with_policy(root, &cmd, strict, &policy_path).map(with_receipt_hash)
        }
        "export-compliance" => {
            let profile = parsed
                .flags
                .get("profile")
                .map(|v| v.as_str())
                .unwrap_or("auditor");
            run_export_compliance(root, strict, &policy_path, profile)
        }
        "identity-surface" => run_identity_surface(root, strict, &parsed.flags),
        "certify-scale" => run_scale_certification(root, strict, &parsed.flags),
        "enable-bedrock" => run_enable_bedrock(root, strict, &parsed.flags),
        "moat-license" => run_moat_license(root, strict, &parsed.flags),
        "moat-contrast" => run_moat_contrast(root, strict, &parsed.flags),
        "moat-launch-sim" => run_moat_launch_sim(root, strict, &parsed.flags),
        "genesis-truth-gate" => run_genesis_truth_gate(root, strict, &parsed.flags),
        "genesis-thin-wrapper-audit" => run_genesis_thin_wrapper_audit(root, strict, &parsed.flags),
        "genesis-doc-freeze" => run_genesis_doc_freeze(root, strict, &parsed.flags),
        "genesis-bootstrap" => run_genesis_bootstrap(root, strict, &parsed.flags),
        "genesis-installer-sim" => run_genesis_installer_sim(root, strict, &parsed.flags),
        "zero-trust-profile" => {
            enterprise_moat_extensions::run_zero_trust_profile(root, strict, &parsed.flags)
        }
        "ops-bridge" => enterprise_moat_extensions::run_ops_bridge(root, strict, &parsed.flags),
        "scale-ha-certify" => {
            enterprise_moat_extensions::run_scale_ha_certify(root, strict, &parsed.flags)
        }
        "deploy-modules" => {
            enterprise_moat_extensions::run_deploy_modules(root, strict, &parsed.flags)
        }
        "super-gate" => enterprise_moat_extensions::run_super_gate(root, strict),
        "adoption-bootstrap" => {
            enterprise_moat_extensions::run_adoption_bootstrap(root, strict, &parsed.flags)
        }
        "replay" => enterprise_moat_extensions::run_replay(root, strict, &parsed.flags),
        "explore" => enterprise_moat_extensions::run_explore(root, strict),
        "ai" => enterprise_moat_extensions::run_ai(root, strict, &parsed.flags),
        "sync" => enterprise_moat_extensions::run_sync(root, strict, &parsed.flags),
        "energy-cert" => enterprise_moat_extensions::run_energy_cert(root, strict, &parsed.flags),
        "migrate-ecosystem" => {
            enterprise_moat_extensions::run_migrate_ecosystem(root, strict, &parsed.flags)
        }
        "chaos-run" => enterprise_moat_extensions::run_chaos(root, strict, &parsed.flags),
        "assistant-mode" | "assistant_mode" => {
            enterprise_moat_extensions::run_assistant_mode(root, strict, &parsed.flags)
        }
        "dashboard" => Ok(run_dashboard(root)),
        _ => {
            usage();
            Ok(with_receipt_hash(json!({
                "ok": false,
                "type": "enterprise_hardening_cli_error",
                "lane": "enterprise_hardening",
                "ts": now_iso(),
                "error": "unknown_command",
                "command": cmd
            })))
        }
    };

    match result {
        Ok(mut payload) => {
            if let Some(guard_result) = &cross_plane_guard {
                match guard_result {
                    Ok(guard) => {
                        payload["cross_plane_jwt_guard"] = guard.clone();
                        let guard_ok = guard
                            .get("guard_ok")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        append_claim_evidence(
                            &mut payload,
                            json!({
                                "id": "V7-F100-002.3",
                                "claim": "cross_plane_calls_require_signed_jwt_cmek_and_private_network_guard",
                                "evidence": {
                                    "guard_ok": guard_ok,
                                    "profile_path": guard.get("profile_path").cloned().unwrap_or(Value::Null)
                                }
                            }),
                        );
                        if strict && !guard_ok {
                            payload["ok"] = Value::Bool(false);
                            payload["error"] =
                                Value::String("cross_plane_jwt_guard_failed".to_string());
                        }
                    }
                    Err(err) => {
                        payload["ok"] = Value::Bool(false);
                        payload["error"] = Value::String(crate::clean(err, 200));
                    }
                }
            }
            if let Err(err) = persist_enterprise_receipt(root, &payload) {
                let out = with_receipt_hash(json!({
                    "ok": false,
                    "type": "enterprise_hardening",
                    "lane": "enterprise_hardening",
                    "mode": cmd,
                    "strict": strict,
                    "ts": now_iso(),
                    "error": format!("persist_failed:{err}")
                }));
                print_pretty(&out);
                return 1;
            }
            print_pretty(&payload);
            if payload.get("type").and_then(Value::as_str) == Some("enterprise_hardening_cli_error")
            {
                2
            } else {
                command_exit(strict, &payload)
            }
        }
        Err(err) => {
            let out = with_receipt_hash(json!({
                "ok": false,
                "type": "enterprise_hardening",
                "lane": "enterprise_hardening",
                "mode": cmd,
                "strict": strict,
                "ts": now_iso(),
                "policy_path": policy_path,
                "error": err
            }));
            let _ = persist_enterprise_receipt(root, &out);
            print_pretty(&out);
            1
        }
    }
}

#[cfg(test)]
#[path = "../enterprise_hardening_tests.rs"]
mod enterprise_hardening_tests;

