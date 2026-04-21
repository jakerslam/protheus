fn attach_execution_receipt(mut out: Value, command: &str, status: &str) -> Value {
    out["execution_receipt"] = json!({
        "lane": "substrate_plane_bio_feedback",
        "command": command,
        "status": status,
        "source": "OPENCLAW-TOOLING-WEB-104",
        "tool_runtime_class": "receipt_wrapped"
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_bio_feedback(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        BIO_FEEDBACK_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "substrate_biological_feedback_contract",
            "allowed_ops": ["stimulate", "degrade", "status"],
            "fallback_mode": "silicon-only",
            "require_consent_for_stimulation": true
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
        let latest = read_json(&bio_feedback_state_path(root)).unwrap_or_else(|| Value::Null);
        let out = json!({
            "ok": true,
            "strict": strict,
            "type": "substrate_plane_bio_feedback",
            "lane": "core/layer0/ops",
            "op": op,
            "latest": latest,
            "claim_evidence": [
                {
                    "id": "V6-SUBSTRATE-002.2",
                    "claim": "closed_loop_feedback_status_surfaces_current_mode_and_degrade_path",
                    "evidence": {
                        "has_latest": !latest.is_null()
                    }
                }
            ]
        });
        return attach_execution_receipt(out, "status", "success");
    }

    if op != "stimulate" && op != "degrade" {
        return attach_execution_receipt(json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_bio_feedback",
            "errors": ["substrate_bio_feedback_op_invalid"]
        }), op.as_str(), "error");
    }

    let interface = read_json(&bio_interface_state_path(root)).unwrap_or_else(|| Value::Null);
    if strict && op == "stimulate" && interface.is_null() {
        return attach_execution_receipt(json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_bio_feedback",
            "errors": ["substrate_bio_feedback_requires_bio_interface_event"]
        }), op.as_str(), "error");
    }
    if strict
        && op == "stimulate"
        && contract
            .get("require_consent_for_stimulation")
            .and_then(Value::as_bool)
            .unwrap_or(true)
        && !parse_bool(parsed.flags.get("consent"), false)
    {
        return attach_execution_receipt(json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_bio_feedback",
            "errors": ["substrate_bio_feedback_consent_required"]
        }), op.as_str(), "error");
    }

    let mode = if op == "degrade" {
        clean(
            parsed
                .flags
                .get("mode")
                .cloned()
                .unwrap_or_else(|| "silicon-only".to_string()),
            30,
        )
    } else {
        clean(
            parsed
                .flags
                .get("mode")
                .cloned()
                .unwrap_or_else(|| "closed-loop".to_string()),
            30,
        )
    }
    .to_ascii_lowercase();
    let degrade = default_degradation(&ExoticDomain::Neural);
    let command_payload = if op == "stimulate" {
        json!({
            "mode": mode,
            "stimulation_level": interface
                .get("mapped_controls")
                .and_then(|v| v.get("attention_gain"))
                .and_then(Value::as_f64)
                .unwrap_or(1.0),
            "reason": "closed_loop_adjustment"
        })
    } else {
        json!({
            "mode": contract
                .get("fallback_mode")
                .and_then(Value::as_str)
                .unwrap_or("silicon-only"),
            "reason": "operator_or_policy_degrade"
        })
    };
    let feedback = json!({
        "version": "v1",
        "op": op,
        "mode": command_payload.get("mode").cloned().unwrap_or(Value::Null),
        "command_payload": command_payload,
        "degrade_contract": {
            "primary": degrade.primary,
            "fallback": degrade.fallback,
            "reason": degrade.reason
        },
        "ts": crate::now_iso()
    });
    let path = bio_feedback_state_path(root);
    let _ = write_json(&path, &feedback);
    let _ = append_jsonl(
        &state_root(root)
            .join("bio")
            .join("feedback")
            .join("history.jsonl"),
        &feedback,
    );
    let out = json!({
        "ok": true,
        "strict": strict,
        "type": "substrate_plane_bio_feedback",
        "lane": "core/layer0/ops",
        "op": op,
        "feedback": feedback,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&feedback.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-SUBSTRATE-002.2",
                "claim": "closed_loop_biological_feedback_supports_deterministic_degrade_to_silicon_only_mode",
                "evidence": {
                    "mode": feedback.get("mode").cloned().unwrap_or(Value::Null)
                }
            }
        ]
    });
    attach_execution_receipt(out, op.as_str(), "success")
}

fn run_bio_adapter_template(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        BIO_ADAPTER_TEMPLATE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "substrate_biological_adapter_template_contract",
            "template_fields": ["spike_rate_channels", "stimulation_channels", "health_telemetry_fields"],
            "layer0_substrate_agnostic": true
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
        let latest = read_json(&bio_adapter_template_path(root)).unwrap_or_else(|| Value::Null);
        let out = json!({
            "ok": true,
            "strict": strict,
            "type": "substrate_plane_bio_adapter_template",
            "lane": "core/layer0/ops",
            "op": "status",
            "latest": latest,
            "claim_evidence": [
                {
                    "id": "V6-SUBSTRATE-002.3",
                    "claim": "biological_adapter_template_surfaces_spike_stimulation_and_health_telemetry_descriptors",
                    "evidence": {
                        "has_latest": !latest.is_null()
                    }
                }
            ]
        });
        return attach_execution_receipt(out, "status", "success");
    }
    if op != "emit" {
        return attach_execution_receipt(json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_bio_adapter_template",
            "errors": ["substrate_bio_adapter_template_op_invalid"]
        }), "emit", "error");
    }

    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("substrate_bio_adapter_template_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "substrate_biological_adapter_template_contract"
    {
        errors.push("substrate_bio_adapter_template_contract_kind_invalid".to_string());
    }
    let spike_channels = split_csv_clean(
        parsed
            .flags
            .get("spike-channels")
            .map(String::as_str)
            .unwrap_or("spike_rate_hz,burst_index"),
        120,
    );
    let stimulation_channels = split_csv_clean(
        parsed
            .flags
            .get("stimulation-channels")
            .map(String::as_str)
            .unwrap_or("stim_current_ua,stim_pulse_width_us"),
        120,
    );
    let telemetry_fields = split_csv_clean(
        parsed
            .flags
            .get("health-telemetry")
            .map(String::as_str)
            .unwrap_or("latency_ms,power_mw,artifact_rate"),
        120,
    );
    if strict && spike_channels.is_empty() {
        errors.push("substrate_bio_adapter_template_spike_channels_required".to_string());
    }
    if strict && stimulation_channels.is_empty() {
        errors.push("substrate_bio_adapter_template_stimulation_channels_required".to_string());
    }
    if strict && telemetry_fields.is_empty() {
        errors.push("substrate_bio_adapter_template_health_telemetry_required".to_string());
    }
    if !errors.is_empty() {
        return attach_execution_receipt(json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_bio_adapter_template",
            "errors": errors
        }), "emit", "error");
    }

    let adapter_id = clean(
        parsed
            .flags
            .get("adapter")
            .cloned()
            .unwrap_or_else(|| "bio-neural-generic".to_string()),
        120,
    );
    let template = json!({
        "version": "v1",
        "adapter_id": adapter_id,
        "spike_rate_channels": spike_channels,
        "stimulation_channels": stimulation_channels,
        "health_telemetry_fields": telemetry_fields,
        "layer0_substrate_agnostic": contract
            .get("layer0_substrate_agnostic")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "ts": crate::now_iso()
    });
    let path = bio_adapter_template_path(root);
    let _ = write_json(&path, &template);
    let _ = append_jsonl(
        &state_root(root)
            .join("bio")
            .join("adapter")
            .join("history.jsonl"),
        &template,
    );
    let out = json!({
        "ok": true,
        "strict": strict,
        "type": "substrate_plane_bio_adapter_template",
        "lane": "core/layer0/ops",
        "op": "emit",
        "template": template,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&template.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-SUBSTRATE-002.3",
                "claim": "pluggable_biological_adapter_template_declares_channels_and_health_telemetry",
                "evidence": {
                    "adapter_id": adapter_id
                }
            }
        ]
    });
    attach_execution_receipt(out, "emit", "success")
}
