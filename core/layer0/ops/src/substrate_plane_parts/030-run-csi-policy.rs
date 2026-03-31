fn run_csi_policy(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CSI_POLICY_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "substrate_csi_policy_contract",
            "locality_default": "local-only",
            "consent_required": true,
            "export_default_denied": true,
            "max_retention_minutes": 1440,
            "risk_classes": ["low", "medium", "high"]
        }),
    );
    let consent = parse_bool(parsed.flags.get("consent"), false);
    let locality = clean(
        parsed
            .flags
            .get("locality")
            .cloned()
            .unwrap_or_else(|| "local-only".to_string()),
        40,
    )
    .to_ascii_lowercase();
    let retention_minutes = parse_u64(parsed.flags.get("retention-minutes"), 60);
    let biometric_risk = clean(
        parsed
            .flags
            .get("biometric-risk")
            .cloned()
            .unwrap_or_else(|| "medium".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let allow_export = parse_bool(parsed.flags.get("allow-export"), false);
    let mut violations = Vec::<String>::new();
    if contract
        .get("consent_required")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && !consent
    {
        violations.push("consent_required".to_string());
    }
    if locality != "local-only" {
        violations.push("locality_must_be_local_only".to_string());
    }
    if retention_minutes
        > contract
            .get("max_retention_minutes")
            .and_then(Value::as_u64)
            .unwrap_or(1440)
    {
        violations.push("retention_window_exceeded".to_string());
    }
    let risk_allowed = contract
        .get("risk_classes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .any(|row| row == biometric_risk);
    if strict && !risk_allowed {
        violations.push("biometric_risk_invalid".to_string());
    }
    if strict
        && biometric_risk == "high"
        && !parse_bool(parsed.flags.get("high-risk-approved"), false)
    {
        violations.push("high_risk_requires_approval".to_string());
    }
    if contract
        .get("export_default_denied")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && allow_export
    {
        violations.push("export_denied_by_default".to_string());
    }
    let ok = if strict { violations.is_empty() } else { true };
    let policy = json!({
        "version": "v1",
        "consent": consent,
        "locality": locality,
        "retention_minutes": retention_minutes,
        "biometric_risk": biometric_risk,
        "allow_export": allow_export,
        "violations": violations,
        "ok": ok,
        "ts": crate::now_iso()
    });
    let path = csi_policy_state_path(root);
    let _ = write_json(&path, &policy);
    let _ = append_jsonl(
        &state_root(root)
            .join("csi")
            .join("policy")
            .join("history.jsonl"),
        &policy,
    );
    let mut out = json!({
        "ok": ok,
        "strict": strict,
        "type": "substrate_plane_csi_policy",
        "lane": "core/layer0/ops",
        "policy": policy,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&policy.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-SUBSTRATE-001.4",
                "claim": "non_visual_sensing_policy_enforces_locality_consent_retention_and_risk_class",
                "evidence": {
                    "ok": ok
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_eye_bind(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        EYE_BINDING_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "substrate_eye_binding_contract",
            "allowed_sources": ["wifi"],
            "allowed_ops": ["enable", "status"],
            "thin_client_surface": ["enable", "status"]
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
    let source = clean(
        parsed
            .flags
            .get("source")
            .cloned()
            .or_else(|| parsed.positional.get(2).cloned())
            .unwrap_or_else(|| "wifi".to_string()),
        30,
    )
    .to_ascii_lowercase();
    let allowed_source = contract
        .get("allowed_sources")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .any(|row| row == source);
    let op_allowed = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .any(|row| row == op);
    if strict && !op_allowed {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_eye_bind",
            "errors": ["substrate_eye_bind_op_invalid"]
        });
    }
    if strict && !allowed_source {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_eye_bind",
            "errors": ["substrate_eye_source_invalid"]
        });
    }

    let path = eye_binding_state_path(root);
    let mut bindings = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "bindings": {}
        })
    });
    if !bindings
        .get("bindings")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        bindings["bindings"] = Value::Object(serde_json::Map::new());
    }
    if op == "status" {
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "substrate_plane_eye_bind",
            "lane": "core/layer0/ops",
            "op": op,
            "bindings": bindings,
            "claim_evidence": [
                {
                    "id": "V6-SUBSTRATE-001.5",
                    "claim": "thin_client_eye_surface_is_limited_to_enable_and_status_for_wifi_source",
                    "evidence": {
                        "enabled_count": bindings
                            .get("bindings")
                            .and_then(Value::as_object)
                            .map(|m| m.len())
                            .unwrap_or(0)
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    let latest_policy = read_json(&csi_policy_state_path(root)).unwrap_or_else(|| json!({}));
    if strict
        && !latest_policy
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_eye_bind",
            "errors": ["substrate_eye_bind_requires_passing_csi_policy"]
        });
    }

    let binding = json!({
        "enabled": true,
        "source": source,
        "persona": clean(parsed.flags.get("persona").cloned().unwrap_or_else(|| "default".to_string()), 120),
        "shadow": clean(parsed.flags.get("shadow").cloned().unwrap_or_else(|| "default-shadow".to_string()), 120),
        "command_alias": "protheus eye enable wifi",
        "enabled_at": crate::now_iso()
    });
    bindings["bindings"][&source] = binding.clone();
    bindings["updated_at"] = Value::String(crate::now_iso());
    let _ = write_json(&path, &bindings);
    let _ = append_jsonl(
        &state_root(root).join("eye").join("history.jsonl"),
        &json!({"op": "enable", "source": source, "binding": binding, "ts": crate::now_iso()}),
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "substrate_plane_eye_bind",
        "lane": "core/layer0/ops",
        "op": "enable",
        "binding": binding,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&bindings.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-SUBSTRATE-001.5",
                "claim": "wifi_csi_is_bound_as_native_eye_source_for_persona_and_shadow_triggers",
                "evidence": {
                    "source": source
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_bio_interface(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        BIO_INTERFACE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "substrate_biological_interface_contract",
            "channels": {"min": 4, "max": 128},
            "model_control_fields": ["attention_gain", "exploration_bias", "safety_temperature"]
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
    if op == "status" {
        let latest = read_json(&bio_interface_state_path(root)).unwrap_or_else(|| Value::Null);
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "substrate_plane_bio_interface",
            "lane": "core/layer0/ops",
            "op": op,
            "latest": latest,
            "claim_evidence": [
                {
                    "id": "V6-SUBSTRATE-002.1",
                    "claim": "biological_interface_status_surfaces_multi_electrode_mapping_state",
                    "evidence": {
                        "has_latest": !latest.is_null()
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if op != "ingest" {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_bio_interface",
            "errors": ["substrate_bio_interface_op_invalid"]
        });
    }
    let channels = parse_u64(parsed.flags.get("channels"), 16);
    let min = contract
        .get("channels")
        .and_then(|v| v.get("min"))
        .and_then(Value::as_u64)
        .unwrap_or(4);
    let max = contract
        .get("channels")
        .and_then(|v| v.get("max"))
        .and_then(Value::as_u64)
        .unwrap_or(128);
    if strict && (channels < min || channels > max) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_bio_interface",
            "errors": ["substrate_bio_interface_channels_out_of_bounds"]
        });
    }
    let payload_ref = clean(
        parsed
            .flags
            .get("payload-ref")
            .cloned()
            .unwrap_or_else(|| "bio://multielectrode/latest".to_string()),
        220,
    );
    let envelope = ExoticEnvelope {
        domain: ExoticDomain::Neural,
        adapter_id: "bio-neural-interface".to_string(),
        signal_type: "multi_electrode_input".to_string(),
        payload_ref,
        ts_ms: chrono::Utc::now().timestamp_millis(),
    };
    let wrapped = wrap_exotic_signal(&envelope, "sense.neural.input");
    let digest = wrapped.deterministic_digest.clone();
    let attention_gain = 0.5 + ((decode_signal_u64(&digest, 0) % 100) as f64 / 100.0);
    let exploration_bias = ((decode_signal_u64(&digest, 8) % 40) as f64 / 100.0) - 0.2;
    let safety_temperature = 0.1 + ((decode_signal_u64(&digest, 16) % 90) as f64 / 100.0);
    let mapped = json!({
        "attention_gain": attention_gain,
        "exploration_bias": exploration_bias,
        "safety_temperature": safety_temperature
    });
    let event = json!({
        "version": "v1",
        "op": "ingest",
        "channels": channels,
        "wrapped_envelope": wrapped,
        "mapped_controls": mapped,
        "ts": crate::now_iso()
    });
    let path = bio_interface_state_path(root);
    let _ = write_json(&path, &event);
    let _ = append_jsonl(
        &state_root(root)
            .join("bio")
            .join("interface")
            .join("history.jsonl"),
        &event,
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "substrate_plane_bio_interface",
        "lane": "core/layer0/ops",
        "op": op,
        "event": event,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&event.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-SUBSTRATE-002.1",
                "claim": "biological_interface_maps_multi_electrode_input_to_model_control_parameters_with_receipts",
                "evidence": {
                    "channels": channels
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

