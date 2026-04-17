fn normalize_pulse_token(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in raw.trim().to_ascii_lowercase().chars() {
        let mapped = if ch.is_ascii_alphanumeric() || ch == '_' {
            ch
        } else if ch.is_ascii_whitespace() || matches!(ch, '-' | '.' | '/' | ':') {
            '_'
        } else {
            continue;
        };
        if mapped == '_' {
            if prev_sep || out.is_empty() {
                continue;
            }
            prev_sep = true;
            out.push('_');
            continue;
        }
        prev_sep = false;
        out.push(mapped);
    }
    out.trim_matches('_').to_string()
}

fn canonical_directive_pulse_event_type(raw: &str) -> String {
    let token = normalize_pulse_token(raw);
    match token.as_str() {
        "autonomyrun" | "autonomy_run_event" | "directive_pulse_autonomy_run" => {
            "autonomy_run".to_string()
        }
        _ => token,
    }
}

fn canonical_directive_pulse_result(raw: &str) -> String {
    let token = normalize_pulse_token(raw);
    match token.as_str() {
        "stop_repeat_gate_directive_pulse_cooldown_gate"
        | "directive_pulse_cooldown"
        | "stop_repeat_gate_directive_pulse" => {
            "stop_repeat_gate_directive_pulse_cooldown".to_string()
        }
        _ => token,
    }
}

fn canonical_directive_pulse_objective_id(raw: &str) -> String {
    let id = compute_sanitize_directive_objective_id(&SanitizeDirectiveObjectiveIdInput {
        value: Some(raw.to_string()),
    })
    .objective_id;
    if !id.is_empty() {
        return id;
    }
    normalize_pulse_token(raw)
}

pub fn compute_recent_directive_pulse_cooldown_count(
    input: &RecentDirectivePulseCooldownCountInput,
) -> RecentDirectivePulseCooldownCountOutput {
    let objective_id = input
        .objective_id
        .as_ref()
        .map(|v| canonical_directive_pulse_objective_id(v))
        .unwrap_or_default();
    if objective_id.is_empty() {
        return RecentDirectivePulseCooldownCountOutput { count: 0 };
    }
    let hours = non_negative_number(input.hours).unwrap_or(24.0).max(1.0);
    let now_ms = non_negative_number(input.now_ms).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|v| v.as_millis() as f64)
            .unwrap_or(0.0)
    });
    let cutoff = now_ms - (hours * 3_600_000.0);

    let mut count = 0_u32;
    for evt in &input.events {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|v| canonical_directive_pulse_event_type(v))
            .unwrap_or_default();
        if event_type != "autonomy_run" {
            continue;
        }
        let result = evt
            .result
            .as_ref()
            .map(|v| canonical_directive_pulse_result(v))
            .unwrap_or_default();
        if result != "stop_repeat_gate_directive_pulse_cooldown" {
            continue;
        }
        let ts = evt
            .ts
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        let ts_ms = compute_parse_iso_ts(&ParseIsoTsInput {
            ts: if ts.is_empty() { None } else { Some(ts) },
        })
        .timestamp_ms;
        let Some(ms) = ts_ms else {
            continue;
        };
        if ms < cutoff {
            continue;
        }
        let event_objective = evt
            .objective_id
            .as_ref()
            .map(|v| canonical_directive_pulse_objective_id(v))
            .filter(|v| !v.is_empty())
            .or_else(|| {
                evt.sample_objective_id
                    .as_ref()
                    .map(|v| canonical_directive_pulse_objective_id(v))
                    .filter(|v| !v.is_empty())
            })
            .unwrap_or_default();
        if event_objective == objective_id {
            count += 1;
        }
    }

    RecentDirectivePulseCooldownCountOutput { count }
}

pub fn compute_proposal_directive_text(
    input: &ProposalDirectiveTextInput,
) -> ProposalDirectiveTextOutput {
    let proposal = input.proposal.as_ref().unwrap_or(&serde_json::Value::Null);
    let mut parts = vec![
        js_like_string(json_path(proposal, &["title"]).unwrap_or(&serde_json::Value::Null)),
        js_like_string(json_path(proposal, &["type"]).unwrap_or(&serde_json::Value::Null)),
        js_like_string(json_path(proposal, &["summary"]).unwrap_or(&serde_json::Value::Null)),
        js_like_string(json_path(proposal, &["notes"]).unwrap_or(&serde_json::Value::Null)),
        js_like_string(
            json_path(proposal, &["expected_impact"]).unwrap_or(&serde_json::Value::Null),
        ),
        js_like_string(json_path(proposal, &["risk"]).unwrap_or(&serde_json::Value::Null)),
        js_like_string(
            json_path(proposal, &["meta", "preview"]).unwrap_or(&serde_json::Value::Null),
        ),
        js_like_string(json_path(proposal, &["meta", "url"]).unwrap_or(&serde_json::Value::Null)),
        js_like_string(
            json_path(proposal, &["meta", "normalized_objective"])
                .unwrap_or(&serde_json::Value::Null),
        ),
        js_like_string(
            json_path(proposal, &["meta", "normalized_expected_outcome"])
                .unwrap_or(&serde_json::Value::Null),
        ),
        js_like_string(
            json_path(proposal, &["meta", "normalized_validation_metric"])
                .unwrap_or(&serde_json::Value::Null),
        ),
    ];

    let hint_tokens = js_array_to_strings(json_path(proposal, &["meta", "normalized_hint_tokens"]));
    if !hint_tokens.is_empty() {
        parts.push(hint_tokens.join(" "));
    }
    let archetypes = js_array_to_strings(json_path(proposal, &["meta", "normalized_archetypes"]));
    if !archetypes.is_empty() {
        parts.push(archetypes.join(" "));
    }
    let topics = js_array_to_strings(json_path(proposal, &["meta", "topics"]));
    if !topics.is_empty() {
        parts.push(topics.join(" "));
    }
    let validation = js_array_to_strings(json_path(proposal, &["validation"]));
    if !validation.is_empty() {
        parts.push(validation.join(" "));
    }

    if let Some(serde_json::Value::Array(rows)) = json_path(proposal, &["evidence"]) {
        for ev in rows {
            parts.push(js_like_string(
                json_path(ev, &["match"]).unwrap_or(&serde_json::Value::Null),
            ));
            parts.push(js_like_string(
                json_path(ev, &["evidence_ref"]).unwrap_or(&serde_json::Value::Null),
            ));
        }
    }

    let joined = parts.join(" ");
    let normalized =
        compute_normalize_directive_text(&NormalizeDirectiveTextInput { text: Some(joined) })
            .normalized;
    ProposalDirectiveTextOutput { text: normalized }
}

pub fn compute_objective_ids_from_pulse_context(
    input: &ObjectiveIdsFromPulseContextInput,
) -> ObjectiveIdsFromPulseContextOutput {
    let mut ids = Vec::<String>::new();
    let mut seen = std::collections::BTreeSet::<String>::new();

    for row in &input.objectives {
        let id = js_like_string(json_path(row, &["id"]).unwrap_or(&serde_json::Value::Null))
            .trim()
            .to_string();
        if id.is_empty() || !seen.insert(id.clone()) {
            continue;
        }
        ids.push(id);
    }

    if ids.is_empty() && input.fallback_enabled {
        for raw in &input.fallback_ids {
            let id = raw.trim().to_string();
            if id.is_empty() || !seen.insert(id.clone()) {
                continue;
            }
            ids.push(id);
        }
    }

    ObjectiveIdsFromPulseContextOutput { ids }
}

pub fn compute_policy_hold_objective_context(
    input: &PolicyHoldObjectiveContextInput,
) -> PolicyHoldObjectiveContextOutput {
    let mut ids = Vec::<String>::new();
    let mut seen = std::collections::BTreeSet::<String>::new();
    for raw in &input.candidate_objective_ids {
        let id = compute_sanitize_directive_objective_id(&SanitizeDirectiveObjectiveIdInput {
            value: Some(raw.clone()),
        })
        .objective_id;
        if id.is_empty() || !seen.insert(id.clone()) {
            continue;
        }
        ids.push(id);
    }
    if ids.is_empty() {
        for raw in &input.pool_objective_ids {
            let id = compute_sanitize_directive_objective_id(&SanitizeDirectiveObjectiveIdInput {
                value: Some(raw.clone()),
            })
            .objective_id;
            if id.is_empty() || !seen.insert(id.clone()) {
                continue;
            }
            ids.push(id);
        }
    }
    let dominant = compute_sanitize_directive_objective_id(&SanitizeDirectiveObjectiveIdInput {
        value: input.dominant_objective_id.clone(),
    })
    .objective_id;
    let objective_id = if !dominant.is_empty() {
        Some(dominant.clone())
    } else {
        ids.first().cloned()
    };
    let objective_source = if objective_id.is_some() {
        if !dominant.is_empty() {
            Some("directive_pulse_dominant".to_string())
        } else {
            Some("directive_pulse_pool".to_string())
        }
    } else {
        None
    };
    let objective_ids = if ids.len() > 1 {
        Some(ids.into_iter().take(8).collect())
    } else {
        None
    };
    PolicyHoldObjectiveContextOutput {
        objective_id,
        objective_source,
        objective_ids,
    }
}

pub fn compute_proposal_semantic_objective_id(
    input: &ProposalSemanticObjectiveIdInput,
) -> ProposalSemanticObjectiveIdOutput {
    let proposal = input.proposal.as_ref().unwrap_or(&serde_json::Value::Null);
    let candidates = vec![
        json_path(proposal, &["meta", "objective_id"]).map(js_like_string),
        json_path(proposal, &["meta", "directive_objective_id"]).map(js_like_string),
        json_path(proposal, &["meta", "linked_objective_id"]).map(js_like_string),
        Some(
            compute_parse_directive_objective_arg(&ParseDirectiveObjectiveArgInput {
                command: json_path(proposal, &["suggested_next_command"]).map(js_like_string),
            })
            .objective_id,
        ),
        Some(
            compute_parse_directive_objective_arg(&ParseDirectiveObjectiveArgInput {
                command: json_path(proposal, &["suggested_command"]).map(js_like_string),
            })
            .objective_id,
        ),
    ];
    for raw in candidates.into_iter().flatten() {
        let id = compute_sanitize_directive_objective_id(&SanitizeDirectiveObjectiveIdInput {
            value: Some(raw),
        })
        .objective_id;
        if !id.is_empty() {
            return ProposalSemanticObjectiveIdOutput { objective_id: id };
        }
    }
    ProposalSemanticObjectiveIdOutput {
        objective_id: String::new(),
    }
}

pub fn compute_criteria_pattern_keys(
    input: &CriteriaPatternKeysInput,
) -> CriteriaPatternKeysOutput {
    let hint =
        normalize_spaces(input.capability_key_hint.as_deref().unwrap_or("")).to_ascii_lowercase();
    let descriptor = normalize_spaces(input.capability_descriptor_key.as_deref().unwrap_or(""))
        .to_ascii_lowercase();
    let cap_key = if !hint.is_empty() {
        hint
    } else if !descriptor.is_empty() {
        descriptor
    } else {
        "unknown".to_string()
    };
    let mut keys = std::collections::BTreeSet::<String>::new();
    for row in &input.rows {
        let metric = compute_normalize_criteria_metric(&NormalizeCriteriaMetricInput {
            value: row.metric.clone(),
        })
        .metric;
        if metric.is_empty() {
            continue;
        }
        keys.insert(format!("{cap_key}|{metric}"));
    }
    CriteriaPatternKeysOutput {
        keys: keys.into_iter().collect(),
    }
}

pub fn compute_success_criteria_requirement(
    input: &SuccessCriteriaRequirementInput,
) -> SuccessCriteriaRequirementOutput {
    let mut exempt_types = Vec::<String>::new();
    let mut seen = std::collections::BTreeSet::<String>::new();
    for raw in input
        .policy_exempt_types
        .iter()
        .chain(input.env_exempt_types.iter())
    {
        let value = normalize_spaces(raw).to_ascii_lowercase();
        if value.is_empty() || !seen.insert(value.clone()) {
            continue;
        }
        exempt_types.push(value);
    }
    let raw_min = input.min_success_criteria_count.unwrap_or(1.0);
    let min_count = if !raw_min.is_finite() || raw_min < 0.0 {
        0.0
    } else if raw_min > 5.0 {
        5.0
    } else {
        raw_min
    };
    SuccessCriteriaRequirementOutput {
        required: input.require_success_criteria.unwrap_or(true),
        min_count,
        exempt_types,
    }
}

pub fn compute_success_criteria_policy_for_proposal(
    input: &SuccessCriteriaPolicyForProposalInput,
) -> SuccessCriteriaPolicyForProposalOutput {
    let proposal_type =
        normalize_spaces(input.proposal_type.as_deref().unwrap_or("")).to_ascii_lowercase();
    let mut exempt = false;
    for raw in &input.base_exempt_types {
        let value = normalize_spaces(raw).to_ascii_lowercase();
        if !value.is_empty() && !proposal_type.is_empty() && value == proposal_type {
            exempt = true;
            break;
        }
    }
    SuccessCriteriaPolicyForProposalOutput {
        required: input.base_required && !exempt,
        min_count: input.base_min_count,
        exempt,
    }
}

pub fn compute_capability_descriptor(
    input: &CapabilityDescriptorInput,
) -> CapabilityDescriptorOutput {
    let kind = normalize_spaces(input.actuation_kind.as_deref().unwrap_or("")).to_ascii_lowercase();
    if !kind.is_empty() {
        return CapabilityDescriptorOutput {
            key: format!("actuation:{kind}"),
            aliases: vec!["actuation".to_string()],
        };
    }
    let proposal_type =
        normalize_spaces(input.proposal_type.as_deref().unwrap_or("")).to_ascii_lowercase();
    let typ = if proposal_type.is_empty() {
        "unknown".to_string()
    } else {
        proposal_type
    };
    CapabilityDescriptorOutput {
        key: format!("proposal:{typ}"),
        aliases: vec!["proposal".to_string()],
    }
}

pub fn compute_normalize_token_usage_shape(
    input: &NormalizeTokenUsageShapeInput,
) -> NormalizeTokenUsageShapeOutput {
    let prompt =
        non_negative_number(input.prompt_tokens).or(non_negative_number(input.input_tokens));
    let completion =
        non_negative_number(input.completion_tokens).or(non_negative_number(input.output_tokens));
    let total_direct =
        non_negative_number(input.total_tokens).or(non_negative_number(input.tokens_used));
    let total = if let Some(v) = total_direct {
        Some(v)
    } else if prompt.is_some() || completion.is_some() {
        Some(prompt.unwrap_or(0.0) + completion.unwrap_or(0.0))
    } else {
        None
    };
    if total.is_none() && prompt.is_none() && completion.is_none() {
        return NormalizeTokenUsageShapeOutput {
            has_value: false,
            usage: None,
        };
    }
    NormalizeTokenUsageShapeOutput {
        has_value: true,
        usage: Some(NormalizeTokenUsageShapeValueOutput {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: total,
            source: normalize_spaces(input.source.as_deref().unwrap_or("unknown")),
        }),
    }
}

pub fn compute_is_directive_clarification_proposal(
    input: &IsDirectiveClarificationProposalInput,
) -> IsDirectiveClarificationProposalOutput {
    let proposal_type = input
        .proposal_type
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    IsDirectiveClarificationProposalOutput {
        is_clarification: proposal_type == "directive_clarification",
    }
}

pub fn compute_is_directive_decomposition_proposal(
    input: &IsDirectiveDecompositionProposalInput,
) -> IsDirectiveDecompositionProposalOutput {
    let proposal_type = input
        .proposal_type
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    IsDirectiveDecompositionProposalOutput {
        is_decomposition: proposal_type == "directive_decomposition",
    }
}

pub fn compute_sanitize_directive_objective_id(
    input: &SanitizeDirectiveObjectiveIdInput,
) -> SanitizeDirectiveObjectiveIdOutput {
    let raw = input.value.as_deref().unwrap_or("").trim();
    if raw.is_empty() {
        return SanitizeDirectiveObjectiveIdOutput {
            objective_id: String::new(),
        };
    }
    let re = Regex::new(r"^T[0-9]_[A-Za-z0-9_]+$").expect("valid directive objective id regex");
    if !re.is_match(raw) {
        return SanitizeDirectiveObjectiveIdOutput {
            objective_id: String::new(),
        };
    }
    SanitizeDirectiveObjectiveIdOutput {
        objective_id: raw.to_string(),
    }
}
