fn canonical_interface_channel(raw: &str) -> Option<&'static str> {
    let token = normalize_token_runtime(raw, 80);
    match token.as_str() {
        "belief_update" | "belief-update" | "belief" => Some("belief_update"),
        "strategy_hint" | "strategy-hint" | "strategy" => Some("strategy_hint"),
        "workflow_hint" | "workflow-hint" | "workflow" => Some("workflow_hint"),
        "code_change_proposal"
        | "code-change-proposal"
        | "codechangeproposal"
        | "code_change"
        | "code-change" => Some("code_change_proposal"),
        _ => None,
    }
}

fn resolve_interface_channel_value<'a>(
    map: Option<&'a serde_json::Map<String, Value>>,
    channel: &str,
) -> Option<&'a Value> {
    if let Some(direct) = map.and_then(|m| m.get(channel)) {
        return Some(direct);
    }
    map.and_then(|m| {
        m.iter().find_map(|(key, value)| {
            let canonical = canonical_interface_channel(key.as_str())?;
            if canonical == channel {
                Some(value)
            } else {
                None
            }
        })
    })
}

pub fn compute_build_output_interfaces(
    input: &BuildOutputInterfacesInput,
) -> BuildOutputInterfacesOutput {
    let outputs = input.outputs.as_ref().and_then(|v| v.as_object());
    let mode = compute_normalize_mode(&NormalizeModeInput {
        value: input.mode.clone(),
    })
    .value;
    let sandbox_verified = to_bool_like(input.sandbox_verified.as_ref(), false);
    let explicit_code_proposal_emit =
        to_bool_like(input.explicit_code_proposal_emit.as_ref(), false);
    let channel_payloads = input.channel_payloads.as_ref().and_then(|v| v.as_object());
    let base_payload = input.base_payload.clone().unwrap_or_else(|| json!({}));
    let channel_names = [
        "belief_update",
        "strategy_hint",
        "workflow_hint",
        "code_change_proposal",
    ];

    let mut channels = serde_json::Map::new();
    for name in channel_names {
        let cfg = resolve_interface_channel_value(outputs, name);
        let cfg_enabled = map_bool_key(cfg, "enabled", false);
        let test_enabled = map_bool_key(cfg, "test_enabled", false);
        let live_enabled = map_bool_key(cfg, "live_enabled", false);
        let require_sandbox = map_bool_key(cfg, "require_sandbox_verification", false);
        let require_explicit_emit = map_bool_key(cfg, "require_explicit_emit", false);

        let gate_mode = if mode == "test" {
            test_enabled
        } else {
            live_enabled
        };
        let gate_sandbox = if require_sandbox {
            sandbox_verified
        } else {
            true
        };
        let gate_explicit = if require_explicit_emit {
            if name == "code_change_proposal" {
                explicit_code_proposal_emit
            } else {
                true
            }
        } else {
            true
        };
        let enabled = cfg_enabled && gate_mode && gate_sandbox && gate_explicit;

        let mut reasons = Vec::<Value>::new();
        if !cfg_enabled {
            reasons.push(json!("channel_disabled"));
        }
        if !gate_mode {
            reasons.push(json!(if mode == "test" {
                "test_mode_disabled"
            } else {
                "live_mode_disabled"
            }));
        }
        if !gate_sandbox {
            reasons.push(json!("sandbox_verification_required"));
        }
        if !gate_explicit {
            reasons.push(json!("explicit_emit_required"));
        }

        let payload = if enabled {
            let candidate = resolve_interface_channel_value(channel_payloads, name);
            if js_truthy(candidate) {
                candidate.cloned().unwrap_or_else(|| base_payload.clone())
            } else {
                base_payload.clone()
            }
        } else {
            Value::Null
        };

        channels.insert(
            name.to_string(),
            json!({
                "enabled": enabled,
                "gated_reasons": reasons,
                "payload": payload
            }),
        );
    }

    let default_channel_raw = value_to_string(resolve_interface_channel_value(
        outputs,
        "default_channel",
    ));
    let default_channel = canonical_interface_channel(default_channel_raw.as_str())
        .map(|v| v.to_string())
        .unwrap_or_else(|| {
            canonical_interface_channel(
                value_to_string(outputs.and_then(|m| m.get("defaultChannel"))).as_str(),
            )
            .map(|v| v.to_string())
            .unwrap_or_default()
        });
    let default_channel = if default_channel.is_empty() {
        "strategy_hint".to_string()
    } else {
        default_channel
    };
    let active_channel = if channels
        .get(&default_channel)
        .and_then(|v| v.as_object())
        .and_then(|m| m.get("enabled"))
        .and_then(|v| v.as_bool())
        == Some(true)
    {
        Some(default_channel.clone())
    } else {
        channel_names
            .iter()
            .find(|name| {
                channels
                    .get(**name)
                    .and_then(|v| v.as_object())
                    .and_then(|m| m.get("enabled"))
                    .and_then(|v| v.as_bool())
                    == Some(true)
            })
            .map(|name| (*name).to_string())
    };

    BuildOutputInterfacesOutput {
        default_channel,
        active_channel,
        channels: Value::Object(channels),
    }
}

pub fn compute_build_code_change_proposal_draft(
    input: &BuildCodeChangeProposalDraftInput,
) -> BuildCodeChangeProposalDraftOutput {
    let base = input.base.as_ref().and_then(|v| v.as_object());
    let args = input.args.as_ref().and_then(|v| v.as_object());
    let opts = input.opts.as_ref().and_then(|v| v.as_object());

    let read_text =
        |root: Option<&serde_json::Map<String, Value>>, keys: &[&str], max_len: usize| {
            keys.iter()
                .find_map(|key| {
                    root.and_then(|m| m.get(*key))
                        .map(|v| value_to_string(Some(v)))
                })
                .map(|value| clean_text_runtime(&value, max_len))
                .unwrap_or_default()
        };
    let read_value = |root: Option<&serde_json::Map<String, Value>>, keys: &[&str]| {
        keys.iter()
            .find_map(|key| root.and_then(|m| m.get(*key)))
            .cloned()
    };

    let objective =
        clean_text_runtime(&value_to_string(base.and_then(|m| m.get("objective"))), 260);
    let objective_id = clean_text_runtime(
        &value_to_string(base.and_then(|m| m.get("objective_id"))),
        140,
    );
    let objective_id_value = if objective_id.is_empty() {
        Value::Null
    } else {
        Value::String(objective_id.clone())
    };

    let title = {
        let explicit = read_text(args, &["code_change_title", "code-change-title"], 180);
        if !explicit.is_empty() {
            explicit
        } else {
            clean_text_runtime(
                &format!(
                    "Inversion-driven code-change proposal: {}",
                    if objective.is_empty() {
                        "unknown objective"
                    } else {
                        &objective
                    }
                ),
                180,
            )
        }
    };
    let summary = {
        let explicit = read_text(args, &["code_change_summary", "code-change-summary"], 420);
        if !explicit.is_empty() {
            explicit
        } else {
            clean_text_runtime(
                &format!(
                    "Use guarded inversion outputs to propose a reversible code change for objective \"{}\".",
                    if objective.is_empty() {
                        "unknown"
                    } else {
                        &objective
                    }
                ),
                420,
            )
        }
    };
    let proposed_files = compute_normalize_text_list(&NormalizeTextListInput {
        value: read_value(args, &["code_change_files", "code-change-files"]),
        max_len: Some(220),
        max_items: Some(32),
    })
    .items;
    let proposed_tests = compute_normalize_text_list(&NormalizeTextListInput {
        value: read_value(args, &["code_change_tests", "code-change-tests"]),
        max_len: Some(220),
        max_items: Some(32),
    })
    .items;

    let ts = {
        let value = clean_text_runtime(&value_to_string(base.and_then(|m| m.get("ts"))), 64);
        if value.is_empty() {
            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        } else {
            value
        }
    };
    let risk_note = {
        let value = read_text(args, &["code_change_risk", "code-change-risk"], 320);
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let proposal_id_seed = format!(
        "{}|{}|{}",
        if objective_id.is_empty() {
            objective.as_str()
        } else {
            objective_id.as_str()
        },
        title,
        ts
    );
    let proposal_id = stable_id_runtime(&proposal_id_seed, "icp");
    let mode = {
        let value = clean_text_runtime(&value_to_string(base.and_then(|m| m.get("mode"))), 24);
        if value.is_empty() {
            "test".to_string()
        } else {
            value
        }
    };
    let shadow_mode = to_bool_like(base.and_then(|m| m.get("shadow_mode")), true);
    let impact = compute_normalize_impact(&NormalizeImpactInput {
        value: Some(value_to_string(base.and_then(|m| m.get("impact")))),
    })
    .value;
    let target = compute_normalize_target(&NormalizeTargetInput {
        value: Some(value_to_string(base.and_then(|m| m.get("target")))),
    })
    .value;
    let certainty = round6(clamp_number(
        parse_number_like(base.and_then(|m| m.get("certainty"))).unwrap_or(0.0),
        0.0,
        1.0,
    ));
    let maturity_band = {
        let value = clean_text_runtime(
            &value_to_string(base.and_then(|m| m.get("maturity_band"))),
            24,
        );
        if value.is_empty() {
            "novice".to_string()
        } else {
            value
        }
    };
    let reasons = base
        .and_then(|m| m.get("reasons"))
        .and_then(|v| v.as_array())
        .map(|rows| rows.iter().take(8).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let session_id_value = {
        let value = clean_text_runtime(
            &value_to_string(opts.and_then(|m| m.get("session_id"))),
            120,
        );
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let sandbox_verified = to_bool_like(opts.and_then(|m| m.get("sandbox_verified")), false);

    BuildCodeChangeProposalDraftOutput {
        proposal: json!({
            "proposal_id": proposal_id,
            "ts": ts,
            "type": "code_change_proposal",
            "source": "inversion_controller",
            "mode": mode,
            "shadow_mode": shadow_mode,
            "status": "proposal_only",
            "title": title,
            "summary": summary,
            "objective": objective,
            "objective_id": objective_id_value,
            "impact": impact,
            "target": target,
            "certainty": certainty,
            "maturity_band": maturity_band,
            "reasons": reasons,
            "session_id": session_id_value,
            "sandbox_verified": sandbox_verified,
            "proposed_files": proposed_files,
            "proposed_tests": proposed_tests,
            "risk_note": risk_note,
            "governance": {
                "require_mirror_simulation": true,
                "require_human_approval": true,
                "live_apply_locked": true
            }
        }),
    }
}

pub fn compute_normalize_library_row(
    input: &NormalizeLibraryRowInput,
) -> NormalizeLibraryRowOutput {
    let src = input.row.as_ref().and_then(|v| v.as_object());

    let id = clean_text_runtime(&value_to_string(src.and_then(|m| m.get("id"))), 80);
    let ts = clean_text_runtime(&value_to_string(src.and_then(|m| m.get("ts"))), 40);
    let objective = clean_text_runtime(&value_to_string(src.and_then(|m| m.get("objective"))), 280);
    let objective_id = clean_text_runtime(
        &value_to_string(src.and_then(|m| m.get("objective_id"))),
        120,
    );
    let signature = clean_text_runtime(&value_to_string(src.and_then(|m| m.get("signature"))), 240);

    let signature_tokens = if let Some(tokens) = src
        .and_then(|m| m.get("signature_tokens"))
        .and_then(|v| v.as_array())
    {
        tokens
            .iter()
            .map(|row| {
                compute_normalize_word_token(&NormalizeWordTokenInput {
                    value: Some(value_to_string(Some(row))),
                    max_len: Some(40),
                })
                .value
            })
            .filter(|row| !row.is_empty())
            .take(64)
            .collect::<Vec<_>>()
    } else {
        compute_tokenize_text(&TokenizeTextInput {
            value: Some(if !signature.is_empty() {
                signature.clone()
            } else {
                objective.clone()
            }),
            max_tokens: None,
        })
        .tokens
    };

    let target = compute_normalize_target(&NormalizeTargetInput {
        value: Some(value_to_string(src.and_then(|m| m.get("target")))),
    })
    .value;
    let impact = compute_normalize_impact(&NormalizeImpactInput {
        value: Some(value_to_string(src.and_then(|m| m.get("impact")))),
    })
    .value;
    let certainty = clamp_number(
        parse_number_like(src.and_then(|m| m.get("certainty"))).unwrap_or(0.0),
        0.0,
        1.0,
    );
    let filter_stack_input = src
        .and_then(|m| m.get("filter_stack"))
        .cloned()
        .or_else(|| src.and_then(|m| m.get("filters")).cloned())
        .unwrap_or_else(|| json!([]));
    let filter_stack = compute_normalize_list(&NormalizeListInput {
        value: Some(filter_stack_input),
        max_len: Some(120),
    })
    .items;
    let outcome_trit = (normalize_trit_value(
        src.and_then(|m| m.get("outcome_trit"))
            .unwrap_or(&Value::Null),
    ))
    .clamp(-1, 1);
    let result = compute_normalize_result(&NormalizeResultInput {
        value: Some(value_to_string(src.and_then(|m| m.get("result")))),
    })
    .value;
    let maturity_band = compute_normalize_token(&NormalizeTokenInput {
        value: Some(value_to_string(src.and_then(|m| m.get("maturity_band")))),
        max_len: Some(24),
    })
    .value;
    let principle_id = {
        let v = clean_text_runtime(
            &value_to_string(src.and_then(|m| m.get("principle_id"))),
            80,
        );
        if v.is_empty() {
            Value::Null
        } else {
            Value::String(v)
        }
    };
    let session_id = {
        let v = clean_text_runtime(&value_to_string(src.and_then(|m| m.get("session_id"))), 80);
        if v.is_empty() {
            Value::Null
        } else {
            Value::String(v)
        }
    };

    NormalizeLibraryRowOutput {
        row: json!({
            "id": id,
            "ts": ts,
            "objective": objective,
            "objective_id": objective_id,
            "signature": signature,
            "signature_tokens": signature_tokens,
            "target": target,
            "impact": impact,
            "certainty": certainty,
            "filter_stack": filter_stack,
            "outcome_trit": outcome_trit,
            "result": result,
            "maturity_band": maturity_band,
            "principle_id": principle_id,
            "session_id": session_id
        }),
    }
}

pub fn compute_ensure_dir(input: &EnsureDirInput) -> EnsureDirOutput {
    let dir = input.dir_path.as_deref().unwrap_or("").trim();
    if dir.is_empty() {
        return EnsureDirOutput { ok: true };
    }
    let _ = fs::create_dir_all(dir);
    EnsureDirOutput { ok: true }
}
