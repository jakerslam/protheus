fn run_bioethics_policy(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        BIO_ETHICS_POLICY_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "substrate_biological_ethics_policy_contract",
            "required_approvals": ["HMAN-BIO-001"],
            "blocked_external_prepared": true,
            "high_risk_requires_explicit_approval": true,
            "consent_required": true
        }),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        20,
    )
    .to_ascii_lowercase();

    let path = bioethics_state_path(root);
    let mut state = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "approvals": {},
            "consent": false,
            "high_risk_approved": false,
            "last_enforced_ok": false
        })
    });
    if !state
        .get("approvals")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["approvals"] = Value::Object(serde_json::Map::new());
    }

    if op == "approve" {
        let approval = clean(
            parsed
                .flags
                .get("approval")
                .cloned()
                .unwrap_or_else(|| "HMAN-BIO-001".to_string()),
            120,
        );
        let artifact_ref = clean(
            parsed
                .flags
                .get("artifact-ref")
                .cloned()
                .unwrap_or_else(|| "evidence://pending-human-approval".to_string()),
            220,
        );
        state["approvals"][&approval] = json!({
            "approved": true,
            "artifact_ref": artifact_ref,
            "approved_at": crate::now_iso()
        });
        state["updated_at"] = Value::String(crate::now_iso());
        let _ = write_json(&path, &state);
        let _ = append_jsonl(
            &state_root(root)
                .join("bio")
                .join("ethics")
                .join("history.jsonl"),
            &json!({
                "op": "approve",
                "approval": approval,
                "ts": crate::now_iso()
            }),
        );
    } else if op == "enforce" {
        state["consent"] = Value::Bool(parse_bool(parsed.flags.get("consent"), false));
        state["high_risk_approved"] = Value::Bool(parse_bool(parsed.flags.get("high-risk"), false));
        state["updated_at"] = Value::String(crate::now_iso());
    } else if op != "status" {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_bioethics_policy",
            "errors": ["substrate_bioethics_policy_op_invalid"]
        });
    }

    let required = contract
        .get("required_approvals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|row| row.to_string())
        .collect::<Vec<_>>();
    let missing_approvals = required
        .iter()
        .filter(|id| {
            !state
                .get("approvals")
                .and_then(Value::as_object)
                .and_then(|m| m.get(*id))
                .and_then(|v| v.get("approved"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let blocked_external = contract
        .get("blocked_external_prepared")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let consent_required = contract
        .get("consent_required")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let consent_ok = !consent_required
        || state
            .get("consent")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let high_risk_requires = contract
        .get("high_risk_requires_explicit_approval")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let high_risk_ok = !high_risk_requires
        || state
            .get("high_risk_approved")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let mut reason_codes = Vec::<String>::new();
    if !missing_approvals.is_empty() {
        reason_codes.push("missing_required_human_approvals".to_string());
        if blocked_external {
            reason_codes.push("blocked_external_hman_bio_001".to_string());
        }
    }
    if !consent_ok {
        reason_codes.push("bioethics_consent_required".to_string());
    }
    if !high_risk_ok {
        reason_codes.push("bioethics_high_risk_approval_required".to_string());
    }
    let ok = if strict {
        reason_codes.is_empty()
    } else {
        true
    };
    state["last_enforced_ok"] = Value::Bool(ok);
    let _ = write_json(&path, &state);
    let _ = append_jsonl(
        &state_root(root)
            .join("bio")
            .join("ethics")
            .join("history.jsonl"),
        &json!({
            "op": op,
            "ok": ok,
            "reason_codes": reason_codes,
            "ts": crate::now_iso()
        }),
    );

    let mut out = json!({
        "ok": ok,
        "strict": strict,
        "type": "substrate_plane_bioethics_policy",
        "lane": "core/layer0/ops",
        "op": op,
        "policy_state": state,
        "required_approvals": required,
        "missing_approvals": missing_approvals,
        "reason_codes": reason_codes,
        "blocked_external_prepared": blocked_external,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&read_json(&path).unwrap_or_else(|| json!({})).to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-SUBSTRATE-002.4",
                "claim": "bioethics_policy_gate_enforces_revocable_consent_human_approvals_and_high_risk_disable_paths",
                "evidence": {
                    "blocked_external_prepared": blocked_external,
                    "missing_approvals": missing_approvals
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_bio_enable(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        BIO_ENABLE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "substrate_biological_enable_contract",
            "allowed_modes": ["biological", "silicon-only"],
            "persona_visibility": true
        }),
    );
    let mode = clean(
        parsed
            .flags
            .get("mode")
            .cloned()
            .unwrap_or_else(|| "biological".to_string()),
        30,
    )
    .to_ascii_lowercase();
    let allowed = contract
        .get("allowed_modes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .any(|row| row == mode);
    if strict && !allowed {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_bio_enable",
            "errors": ["substrate_bio_enable_mode_invalid"]
        });
    }

    let ethics = read_json(&bioethics_state_path(root)).unwrap_or_else(|| json!({}));
    let ethics_ok = ethics
        .get("last_enforced_ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if strict && mode == "biological" && !ethics_ok {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_bio_enable",
            "errors": ["substrate_bio_enable_requires_passing_bioethics_policy"]
        });
    }
    let template = read_json(&bio_adapter_template_path(root)).unwrap_or_else(|| json!({}));
    let template_ok = template
        .get("version")
        .and_then(Value::as_str)
        .map(|v| v == "v1")
        .unwrap_or(false);
    if strict && mode == "biological" && !template_ok {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_bio_enable",
            "errors": ["substrate_bio_enable_requires_adapter_template"]
        });
    }

    let activation = json!({
        "version": "v1",
        "mode": mode,
        "adapter_id": clean(
            parsed
                .flags
                .get("adapter")
                .cloned()
                .unwrap_or_else(|| "bio-neural-generic".to_string()),
            120
        ),
        "persona": clean(
            parsed
                .flags
                .get("persona")
                .cloned()
                .unwrap_or_else(|| "operator".to_string()),
            120
        ),
        "dashboard_hint": "infring-top substrate biological",
        "command_alias": "infring substrate enable biological",
        "ethics_policy_ok": ethics_ok,
        "ts": crate::now_iso()
    });
    let path = bio_enable_state_path(root);
    let _ = write_json(&path, &activation);
    let _ = append_jsonl(
        &state_root(root)
            .join("bio")
            .join("enable")
            .join("history.jsonl"),
        &activation,
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "substrate_plane_bio_enable",
        "lane": "core/layer0/ops",
        "activation": activation,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&read_json(&path).unwrap_or_else(|| json!({})).to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-SUBSTRATE-002.5",
                "claim": "biological_substrate_activation_is_visible_in_cli_persona_and_dashboard_surfaces",
                "evidence": {
                    "command_alias": "infring substrate enable biological"
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let strict = parse_bool(parsed.flags.get("strict"), true);
    let conduit = if command != "status" {
        Some(conduit_enforcement(root, &parsed, strict, &command))
    } else {
        None
    };
    if strict
        && conduit
            .as_ref()
            .and_then(|v| v.get("ok"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return emit(
            root,
            json!({
                "ok": false,
                "strict": strict,
                "type": "substrate_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "csi-capture" => run_csi_capture(root, &parsed, strict),
        "csi-module" => run_csi_module(root, &parsed, strict),
        "csi-embedded-profile" => run_csi_embedded_profile(root, &parsed, strict),
        "csi-policy" => run_csi_policy(root, &parsed, strict),
        "eye-bind" => run_eye_bind(root, &parsed, strict),
        "bio-interface" => run_bio_interface(root, &parsed, strict),
        "bio-feedback" => run_bio_feedback(root, &parsed, strict),
        "bio-adapter-template" => run_bio_adapter_template(root, &parsed, strict),
        "bioethics-policy" => run_bioethics_policy(root, &parsed, strict),
        "bio-enable" => run_bio_enable(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "substrate_plane_error",
            "error": "unknown_command",
            "command": command
        }),
    };
    if command == "status" {
        print_json(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["csi-capture".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "csi-capture");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn decode_signal_u64_handles_bounds() {
        assert_eq!(decode_signal_u64("abcdef", 99), 0);
        assert!(decode_signal_u64("001122334455", 0) > 0);
    }
}

